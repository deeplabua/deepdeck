//! `ddeck status` — a plain-text board without the TUI. Proves the whole chain
//! (hook → log → fold) before there is a screen.

use std::fmt::Write as _;

use deepdeck_core::hooks::truncate_chars;
use deepdeck_core::{AgentState, Board, NoteType, ProjectView, TaskStatus};
use jiff::{SignedDuration, Timestamp};

pub struct Style {
    pub color: bool,
}

impl Style {
    fn paint(&self, code: &str, s: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_owned()
        }
    }
}

/// Projects active within `hours` (all of them when `None`).
pub fn recent(board: Board, now: Timestamp, hours: Option<i64>) -> Board {
    let Some(hours) = hours else { return board };
    let cutoff = now - SignedDuration::from_hours(hours);
    Board {
        projects: board
            .projects
            .into_iter()
            .filter(|p| p.last_activity.is_some_and(|t| t >= cutoff))
            .collect(),
    }
}

/// One number for a status bar: agents waiting for you + open questions.
pub fn render_short(board: &Board) -> String {
    let questions = board
        .projects
        .iter()
        .flat_map(|p| &p.notes)
        .filter(|n| n.note_type == NoteType::Question)
        .count();
    (board.waiting_count() + questions).to_string()
}

pub fn render(board: &Board, now: Timestamp, style: &Style) -> String {
    let mut out = String::new();
    if board.projects.is_empty() {
        out.push_str("No activity yet. Start a Claude Code session — it will show up here.\n");
        return out;
    }
    let waiting = board.waiting_count();
    let n = board.projects.len();
    let _ = write!(
        out,
        "DeepDeck · {n} project{}",
        if n == 1 { "" } else { "s" }
    );
    if waiting > 0 {
        let _ = write!(
            out,
            " · {}",
            style.paint("33", &format!("{waiting} waiting for you"))
        );
    }
    out.push('\n');
    let width = board
        .projects
        .iter()
        .map(|p| p.meta.name.chars().count())
        .max()
        .unwrap_or(0);
    for p in &board.projects {
        out.push('\n');
        render_project(&mut out, p, width, now, style);
    }
    out
}

fn render_project(out: &mut String, p: &ProjectView, width: usize, now: Timestamp, style: &Style) {
    let name = format!("{:width$}", p.meta.name);
    let Some(agent) = p.headline() else {
        let _ = writeln!(out, "  {name}  notes only");
        render_notes(out, p, style);
        return;
    };
    let ago = ago(now, agent.state.since());
    let (mark, label) = match &agent.state {
        AgentState::Waiting { .. } => (
            style.paint("33", "●"),
            style.paint("33", &format!("waiting for you · {ago}")),
        ),
        AgentState::Working { .. } => (style.paint("35", "◐"), format!("working · {ago}")),
        AgentState::Stopped { .. } => (style.paint("32", "✓"), format!("done · {ago}")),
        AgentState::Idle { .. } => (style.paint("90", "○"), format!("idle · {ago}")),
        AgentState::Ended { .. } => (style.paint("90", "○"), format!("ended · {ago}")),
    };
    let agents = if p.agents.len() > 1 {
        format!("  ({} sessions)", p.agents.len())
    } else {
        String::new()
    };
    let _ = writeln!(out, "{mark} {}  {label}{agents}", style.paint("1", &name));

    if let AgentState::Waiting { text, .. } = &agent.state {
        let _ = writeln!(out, "  {}", style.paint("33", &truncate_chars(text, 72)));
    }
    let (done, total) = agent.progress();
    if total > 0 {
        let current = agent
            .current_task()
            .map(|t| format!(" → {}", truncate_chars(&t.subject, 60)))
            .unwrap_or_default();
        let _ = writeln!(out, "  plan {done}/{total}{current}");
        for t in agent
            .tasks
            .iter()
            .filter(|t| t.status != TaskStatus::Completed)
            .take(5)
        {
            let m = match t.status {
                TaskStatus::InProgress => style.paint("35", "◐"),
                _ => "○".to_owned(),
            };
            let _ = writeln!(out, "    {m} {}", truncate_chars(&t.subject, 66));
        }
    }
    if let Some(prompt) = &agent.last_prompt {
        let _ = writeln!(
            out,
            "  {} {}",
            style.paint("90", "you:"),
            truncate_chars(prompt, 68)
        );
    }
    if let Some(recap) = &agent.last_recap {
        let _ = writeln!(
            out,
            "  {} {}",
            style.paint("90", "recap:"),
            truncate_chars(recap, 66)
        );
    }
    render_notes(out, p, style);
}

fn render_notes(out: &mut String, p: &ProjectView, style: &Style) {
    for n in &p.notes {
        let m = match n.note_type {
            NoteType::Question => style.paint("36", "?"),
            NoteType::Idea => style.paint("33", "◆"),
            NoteType::Task => "○".to_owned(),
        };
        let _ = writeln!(out, "  {m} {}", truncate_chars(&n.text, 70));
    }
}

/// `now`, `5m`, `2h`, `3d`.
pub fn ago(now: Timestamp, then: Timestamp) -> String {
    let secs = now.duration_since(then).as_secs().max(0);
    match secs {
        0..60 => "now".into(),
        60..3600 => format!("{}m", secs / 60),
        3600..86_400 => format!("{}h", secs / 3600),
        _ => format!("{}d", secs / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use deepdeck_core::{Event, EventBody, ProjectMeta, Source, fold};

    use super::*;

    fn at(min: i64) -> Timestamp {
        Timestamp::from_second(1_793_430_000 + min * 60).unwrap()
    }

    fn ev(min: i64, body: EventBody) -> Event {
        Event::new(at(min), Source::Claude, Some("s".into()), body)
    }

    #[test]
    fn renders_waiting_project_with_plan() {
        let events = vec![
            ev(
                0,
                EventBody::UserPrompt {
                    text: "а вложения тоже переносим?".into(),
                },
            ),
            ev(
                1,
                EventBody::TaskUpsert {
                    task_id: "1".into(),
                    subject: Some("Adapter".into()),
                    status: Some(TaskStatus::Completed),
                },
            ),
            ev(
                1,
                EventBody::TaskUpsert {
                    task_id: "2".into(),
                    subject: Some("Progress UI".into()),
                    status: Some(TaskStatus::InProgress),
                },
            ),
            ev(
                2,
                EventBody::AgentWaiting {
                    text: "Claude needs your permission".into(),
                    reason: None,
                },
            ),
        ];
        let board = fold([(ProjectMeta::for_root(Path::new("/p/DeepVault")), events)]);
        let text = render(&board, at(4), &Style { color: false });
        assert_eq!(
            text,
            "DeepDeck · 1 project · 1 waiting for you\n\
             \n\
             ● DeepVault  waiting for you · 2m\n  \
             Claude needs your permission\n  \
             plan 1/2 → Progress UI\n    \
             ◐ Progress UI\n  \
             you: а вложения тоже переносим?\n"
        );
        assert_eq!(render_short(&board), "1");
    }

    #[test]
    fn recent_filters_by_hours() {
        let board = fold([(
            ProjectMeta::for_root(Path::new("/p/a")),
            vec![ev(0, EventBody::AgentEnded { reason: None })],
        )]);
        assert_eq!(recent(board.clone(), at(60), Some(2)).projects.len(), 1);
        assert_eq!(recent(board, at(60 * 13), Some(12)).projects.len(), 0);
    }

    #[test]
    fn ago_buckets() {
        assert_eq!(ago(at(0), at(0)), "now");
        assert_eq!(ago(at(5), at(0)), "5m");
        assert_eq!(ago(at(125), at(0)), "2h");
        assert_eq!(ago(at(60 * 50), at(0)), "2d");
    }
}
