//! `ddeck status` — a plain-text board without the TUI. Proves the whole chain
//! (hook → log → fold) before there is a screen.

use std::fmt::Write as _;

use deepdeck_core::hooks::truncate_chars;
use deepdeck_core::{AgentState, Board, NoteType, ProjectView, TaskStatus};
use jiff::{SignedDuration, Timestamp};

use crate::i18n::Strings;

pub struct Style {
    pub color: bool,
}

impl Style {
    /// Colors when stdout is a terminal and `NO_COLOR` is unset.
    pub fn for_stdout() -> Self {
        use std::io::IsTerminal as _;
        Style {
            color: std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
        }
    }

    pub fn paint(&self, code: &str, s: &str) -> String {
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

pub fn render(board: &Board, now: Timestamp, style: &Style, t: &Strings) -> String {
    let mut out = String::new();
    if board.projects.is_empty() {
        out.push_str(t.st_none);
        out.push('\n');
        return out;
    }
    let waiting = board.waiting_count();
    let n = board.projects.len();
    out.push_str(&(t.st_projects)(n));
    if waiting > 0 {
        let _ = write!(out, " · {}", style.paint("33", &(t.st_waiting)(waiting)));
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
        render_project(&mut out, p, width, now, style, t);
    }
    out
}

fn render_project(
    out: &mut String,
    p: &ProjectView,
    width: usize,
    now: Timestamp,
    style: &Style,
    t: &Strings,
) {
    let name = format!("{:width$}", p.meta.name);
    let Some(agent) = p.headline() else {
        let _ = writeln!(out, "  {name}  {}", t.st_notes_only);
        render_notes(out, p, style);
        return;
    };
    let ago = (t.ago)(now.duration_since(agent.state.since()).as_secs());
    let (mark, label) = match &agent.state {
        AgentState::Waiting { .. } => (
            style.paint("33", "●"),
            style.paint("33", &format!("{} · {ago}", t.state_waiting)),
        ),
        AgentState::Working { .. } => (
            style.paint("35", "◐"),
            format!("{} · {ago}", t.state_working),
        ),
        AgentState::Stopped { .. } => (style.paint("32", "✓"), format!("{} · {ago}", t.state_done)),
        AgentState::Idle { .. } => (style.paint("90", "○"), format!("{} · {ago}", t.state_idle)),
        AgentState::Ended { .. } => (style.paint("90", "○"), format!("{} · {ago}", t.state_ended)),
    };
    let agents = if p.agents.len() > 1 {
        format!("  {}", (t.st_sessions)(p.agents.len()))
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
        let _ = writeln!(out, "  {} {done}/{total}{current}", t.plan);
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
            style.paint("90", t.you),
            truncate_chars(prompt, 68)
        );
    }
    if let Some(recap) = &agent.last_recap {
        let _ = writeln!(
            out,
            "  {} {}",
            style.paint("90", t.recap),
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
        let text = render(&board, at(4), &Style { color: false }, &crate::i18n::EN);
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
}
