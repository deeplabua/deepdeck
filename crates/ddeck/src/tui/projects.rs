//! The Projects screen (mock-up A) as a pure function: board → lines.
//! No terminal here, so it is tested as plain text at panel widths.

use std::collections::HashSet;

use deepdeck_core::{AgentState, AgentView, Board, NoteType, ProjectId, ProjectView, TaskStatus};
use jiff::Timestamp;
use ratatui::text::{Line, Span};

use super::strings::Strings;
use ratatui::style::Style;

use super::text::{fit, row, wrap};
use super::theme::Theme;

/// Completed tasks beyond this many are folded into "+ N done".
pub const MAX_DONE_SHOWN: usize = 5;
/// Done notes shown under the open ones; the rest fold into "+ N done".
pub const MAX_DONE_NOTES: usize = 3;

/// Something the cursor can stand on. Every line that can be cut with `…`
/// is an item, so `↵` can open it in full.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item {
    Project(ProjectId),
    /// The agent's blocking message.
    Waiting(ProjectId),
    /// A plan step: project, session, task id.
    Task(ProjectId, String, String),
    Prompt(ProjectId),
    Recap(ProjectId),
    /// A project roadmap step, 1-based.
    Roadmap(ProjectId, usize),
    Note(ProjectId, String),
    /// A saved project command, by name.
    Command(ProjectId, String),
}

impl Item {
    pub fn project(&self) -> &ProjectId {
        match self {
            Item::Project(p)
            | Item::Waiting(p)
            | Item::Task(p, ..)
            | Item::Prompt(p)
            | Item::Recap(p)
            | Item::Roadmap(p, _)
            | Item::Command(p, _)
            | Item::Note(p, _) => p,
        }
    }
}

pub struct View<'a> {
    pub selected: Option<&'a Item>,
    /// Open items: full text wrapped; for a project — all completed tasks.
    pub open: &'a HashSet<Item>,
}

/// One selectable line: gutter, `prefix`, then `text` — cut with `…` and
/// followed by `right`, or, when the item is open, wrapped in full under the prefix.
#[allow(clippy::too_many_arguments)]
pub fn item_lines(
    r: &mut Rendered,
    item: Item,
    view: &View<'_>,
    th: &Theme,
    prefix: Vec<Span<'static>>,
    text: &str,
    style: Style,
    right: Option<Span<'static>>,
    width: usize,
) {
    let selected = view.selected == Some(&item);
    let open = view.open.contains(&item);
    r.items.push((item, r.lines.len()));
    let mut spans = vec![gutter(selected, th)];
    spans.extend(prefix);
    if !open {
        r.lines.push(row(spans, text, style, right, width));
        return;
    }
    let indent: usize = spans.iter().map(|s| super::text::width(&s.content)).sum();
    for (i, chunk) in wrap(text, width.saturating_sub(indent))
        .into_iter()
        .enumerate()
    {
        if i == 0 {
            spans.push(Span::styled(chunk, style));
            r.lines.push(Line::from(std::mem::take(&mut spans)));
        } else {
            r.lines.push(Line::from(vec![
                Span::raw(" ".repeat(indent)),
                Span::styled(chunk, style),
            ]));
        }
    }
}

#[derive(Debug, Default)]
pub struct Rendered {
    pub lines: Vec<Line<'static>>,
    /// Selectable items in screen order, with the row each one starts at.
    pub items: Vec<(Item, usize)>,
}

pub fn layout_projects(
    board: &Board,
    now: Timestamp,
    width: u16,
    view: &View<'_>,
    t: &Strings,
    th: &Theme,
) -> Rendered {
    let width = usize::from(width);
    let mut r = Rendered::default();
    let (quiet, busy): (Vec<&ProjectView>, Vec<&ProjectView>) =
        board.projects.iter().partition(|p| is_quiet(p));
    for (i, p) in busy.into_iter().enumerate() {
        if i > 0 {
            r.lines.push(Line::default());
        }
        project_block(&mut r, p, now, width, view, t, th);
    }
    if !quiet.is_empty() {
        if !r.lines.is_empty() {
            r.lines.push(Line::default());
        }
        let names: Vec<&str> = quiet.iter().map(|p| p.meta.name.as_str()).collect();
        r.lines.push(row(
            vec![
                Span::raw(" "),
                Span::styled(format!("○ {}: ", t.quiet), th.dim()),
            ],
            &names.join(" · "),
            th.dim(),
            None,
            width,
        ));
    }
    r
}

/// Nothing to show but a name: no plan, no conversation, no notes, no summary
/// of a past session, nobody working or waiting. Such projects share one line at the bottom.
fn is_quiet(p: &ProjectView) -> bool {
    p.notes.is_empty()
        && p.roadmap.is_empty()
        && p.last_summary.is_none()
        && p.agents.iter().all(|a| {
            matches!(a.state, AgentState::Idle { .. } | AgentState::Ended { .. })
                && a.tasks.is_empty()
                && a.last_prompt.is_none()
                && a.last_recap.is_none()
        })
}

/// Between turns: the agent's own summary beats the recap of its last reply;
/// a fresh session shows what the previous one left. (Label, text.)
fn resting_line<'a>(
    p: &'a ProjectView,
    agent: &'a AgentView,
    t: &Strings,
) -> Option<(&'static str, &'a str)> {
    let resting = matches!(
        agent.state,
        AgentState::Stopped { .. } | AgentState::Idle { .. } | AgentState::Ended { .. }
    );
    let previous = p
        .last_summary
        .as_ref()
        .filter(|s| s.session != agent.session);
    match (&agent.summary, &agent.last_recap, previous) {
        (Some(s), _, _) if resting => Some((t.summary, s.as_str())),
        (None, Some(recap), _) if resting => Some((t.recap, recap.as_str())),
        (None, None, Some(prev)) if agent.last_prompt.is_none() => {
            Some((t.last_session, prev.text.as_str()))
        }
        _ => None,
    }
}

/// The full text behind a line, as shown when it is opened — what `y` copies.
pub fn item_text(p: &ProjectView, item: &Item, t: &Strings) -> Option<String> {
    let head = p.headline();
    match item {
        Item::Project(_) => Some(p.meta.root.display().to_string()),
        Item::Waiting(_) => match &head?.state {
            AgentState::Waiting { text, .. } => Some(text.clone()),
            _ => None,
        },
        Item::Task(_, session, id) => p
            .agents
            .iter()
            .find(|a| &a.session == session)?
            .tasks
            .iter()
            .find(|task| &task.id == id)
            .map(|task| task.subject.clone()),
        Item::Prompt(_) => head?.last_prompt.clone(),
        Item::Recap(_) => resting_line(p, head?, t).map(|(_, text)| text.to_owned()),
        Item::Roadmap(_, n) => p.roadmap.get(n.checked_sub(1)?).map(|s| s.subject.clone()),
        Item::Note(_, id) => p
            .notes
            .iter()
            .chain(&p.done_notes)
            .find(|n| &n.id == id)
            .map(|n| n.text.clone()),
        Item::Command(_, name) => p
            .commands
            .iter()
            .find(|c| &c.name == name)
            .map(|c| c.command.clone()),
    }
}

/// Selection gutter: one column at the left edge.
pub fn gutter(selected: bool, th: &Theme) -> Span<'static> {
    if selected {
        Span::styled("▌", th.accent())
    } else {
        Span::raw(" ")
    }
}

fn project_block(
    r: &mut Rendered,
    p: &ProjectView,
    now: Timestamp,
    width: usize,
    view: &View<'_>,
    t: &Strings,
    th: &Theme,
) {
    let item = Item::Project(p.meta.id.clone());
    let selected = view.selected == Some(&item);
    r.items.push((item, r.lines.len()));

    let ago = |ts: Timestamp| (t.ago)(now.duration_since(ts).as_secs());
    let agent = p.headline();

    // ── header: mark, name, state · time ──
    let (mark, state_style, word, since) = match agent.map(|a| &a.state) {
        Some(AgentState::Waiting { since, .. }) => {
            ("●", th.waiting(), t.state_waiting, Some(*since))
        }
        Some(AgentState::Working { since }) => ("◐", th.working(), t.state_working, Some(*since)),
        Some(AgentState::Stopped { at }) => ("✓", th.done(), t.state_done, Some(*at)),
        Some(AgentState::Idle { since }) => ("○", th.dim(), t.state_idle, Some(*since)),
        Some(AgentState::Ended { at }) => ("○", th.dim(), t.state_ended, Some(*at)),
        None => ("○", th.dim(), "", p.last_activity),
    };
    let ago_text = since.map(ago).unwrap_or_default();
    let full = if word.is_empty() {
        ago_text.clone()
    } else {
        format!("{word} · {ago_text}")
    };
    // Narrow panel: the mark and colour already say the state; keep the time.
    let fits = 3 + super::text::width(&p.meta.name) + 2 + super::text::width(&full) <= width;
    let state = if fits { full } else { ago_text };
    let live = |a: &&deepdeck_core::AgentView| !matches!(a.state, AgentState::Ended { .. });
    let others = p
        .agents
        .iter()
        .filter(live)
        .count()
        .saturating_sub(usize::from(agent.as_ref().is_some_and(live)));
    let right = if others > 0 && fits {
        format!("{} {state}", (t.more_sessions)(others))
    } else {
        state
    };
    r.lines.push(row(
        vec![
            gutter(selected, th),
            Span::styled(format!("{mark} "), state_style),
        ],
        &p.meta.name,
        th.name(),
        Some(Span::styled(right, state_style)),
        width,
    ));

    if let Some(agent) = agent {
        // ── what the agent is asking ──
        if let AgentState::Waiting { text, .. } = &agent.state {
            let item = Item::Waiting(p.meta.id.clone());
            let sel = view.selected == Some(&item);
            let open = view.open.contains(&item);
            r.items.push((item, r.lines.len()));
            let room = width.saturating_sub(4);
            let chunks = if open {
                wrap(text, room.saturating_sub(2))
            } else {
                vec![fit(text, room.saturating_sub(2))]
            };
            for (i, chunk) in chunks.into_iter().enumerate() {
                let body = format!(" {chunk} ");
                let pad = room.saturating_sub(super::text::width(&body));
                let lead = if i == 0 {
                    gutter(sel, th)
                } else {
                    Span::raw(" ")
                };
                r.lines.push(Line::from(vec![
                    lead,
                    Span::raw("  "),
                    Span::styled(format!("{body}{}", " ".repeat(pad)), th.waiting_banner()),
                ]));
            }
        }

        // ── plan ──
        let (done, total) = agent.progress();
        if total > 0 {
            r.lines.push(row(
                vec![Span::raw("   ")],
                t.plan,
                th.dim(),
                Some(Span::styled(format!("{done}/{total}"), th.dim())),
                width,
            ));
            let fold_done =
                done > MAX_DONE_SHOWN && !view.open.contains(&Item::Project(p.meta.id.clone()));
            if fold_done {
                r.lines.push(row(
                    vec![Span::raw("     ")],
                    &(t.more_done)(done),
                    th.done(),
                    None,
                    width,
                ));
            }
            for task in &agent.tasks {
                let (m, style) = match task.status {
                    TaskStatus::Completed if fold_done => continue,
                    TaskStatus::Completed => ("✓ ", th.done()),
                    TaskStatus::InProgress => ("◐ ", th.working()),
                    _ => ("○ ", th.dim()),
                };
                let text_style = if task.status == TaskStatus::Completed {
                    th.dim()
                } else {
                    Default::default()
                };
                item_lines(
                    r,
                    Item::Task(p.meta.id.clone(), agent.session.clone(), task.id.clone()),
                    view,
                    th,
                    vec![Span::raw("    "), Span::styled(m, style)],
                    &task.subject,
                    text_style,
                    None,
                    width,
                );
            }
        }

        // ── conversation context ──
        if let Some(prompt) = &agent.last_prompt {
            item_lines(
                r,
                Item::Prompt(p.meta.id.clone()),
                view,
                th,
                vec![
                    Span::raw("  "),
                    Span::styled(format!("{} ", t.you), th.dim()),
                ],
                prompt,
                Style::new(),
                None,
                width,
            );
        }
        let line = resting_line(p, agent, t);
        if let Some((label, text)) = line {
            item_lines(
                r,
                Item::Recap(p.meta.id.clone()),
                view,
                th,
                vec![Span::raw("  "), Span::styled(format!("{label} "), th.dim())],
                text,
                Style::new(),
                None,
                width,
            );
        }
    }

    // ── roadmap: what comes after the current request ──
    if !p.roadmap.is_empty() {
        let total = p.roadmap.len();
        let done = p
            .roadmap
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        r.lines.push(row(
            vec![Span::raw("   ")],
            t.roadmap,
            th.accent(),
            Some(Span::styled(format!("{done}/{total}"), th.dim())),
            width,
        ));
        let fold_done =
            done > MAX_DONE_SHOWN && !view.open.contains(&Item::Project(p.meta.id.clone()));
        if fold_done {
            r.lines.push(row(
                vec![Span::raw("     ")],
                &(t.more_done)(done),
                th.done(),
                None,
                width,
            ));
        }
        for (i, step) in p.roadmap.iter().enumerate() {
            let (m, style) = match step.status {
                TaskStatus::Completed if fold_done => continue,
                TaskStatus::Completed => ("✓ ", th.done()),
                TaskStatus::InProgress => ("◐ ", th.working()),
                _ => ("○ ", th.dim()),
            };
            let text_style = if step.status == TaskStatus::Completed {
                th.dim()
            } else {
                Style::new()
            };
            item_lines(
                r,
                Item::Roadmap(p.meta.id.clone(), i + 1),
                view,
                th,
                vec![Span::raw("    "), Span::styled(m, style)],
                &step.subject,
                text_style,
                None,
                width,
            );
        }
    }

    // ── notes, grouped like plan and roadmap: questions, ideas, tasks ──
    for (note_type, title, mark, style) in [
        (NoteType::Question, t.notes_questions, "? ", th.question()),
        (NoteType::Idea, t.notes_ideas, "◆ ", th.idea()),
        (NoteType::Task, t.notes_tasks, "○ ", th.dim()),
    ] {
        let open: Vec<_> = p
            .notes
            .iter()
            .filter(|n| n.note_type == note_type)
            .collect();
        let done: Vec<_> = p
            .done_notes
            .iter()
            .filter(|n| n.note_type == note_type)
            .collect();
        if open.is_empty() && done.is_empty() {
            continue;
        }
        let total = open.len() + done.len();
        r.lines.push(row(
            vec![Span::raw("   ")],
            title,
            style,
            Some(Span::styled(format!("{}/{total}", done.len()), th.dim())),
            width,
        ));
        for n in open {
            item_lines(
                r,
                Item::Note(p.meta.id.clone(), n.id.clone()),
                view,
                th,
                vec![Span::raw("    "), Span::styled(mark, style)],
                &n.text,
                Style::new(),
                None,
                width,
            );
        }
        // Done notes stay, dimmed and reopenable with ␣; the oldest fold away.
        let fold =
            done.len() > MAX_DONE_NOTES && !view.open.contains(&Item::Project(p.meta.id.clone()));
        let shown = if fold { MAX_DONE_NOTES } else { done.len() };
        for n in &done[..shown] {
            item_lines(
                r,
                Item::Note(p.meta.id.clone(), n.id.clone()),
                view,
                th,
                vec![Span::raw("    "), Span::styled("✓ ", th.done())],
                &n.text,
                th.dim(),
                None,
                width,
            );
        }
        if fold {
            r.lines.push(row(
                vec![Span::raw("     ")],
                &(t.more_done)(done.len() - shown),
                th.dim(),
                None,
                width,
            ));
        }
    }

    // ── saved commands: the project's runbook (`ddeck cmd run <name>`) ──
    if !p.commands.is_empty() {
        r.lines.push(row(
            vec![Span::raw("   ")],
            t.commands,
            th.accent(),
            Some(Span::styled(p.commands.len().to_string(), th.dim())),
            width,
        ));
        for c in &p.commands {
            item_lines(
                r,
                Item::Command(p.meta.id.clone(), c.name.clone()),
                view,
                th,
                vec![Span::raw("    "), Span::styled("$ ", th.accent())],
                &format!("{} · {}", c.name, c.command),
                Style::new(),
                None,
                width,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use deepdeck_core::{Event, EventBody, ProjectMeta, Source, fold};

    use super::super::strings::EN;
    use super::super::text::{plain, width};
    use super::*;

    fn at(min: i64) -> Timestamp {
        Timestamp::from_second(1_793_430_000 + min * 60).unwrap()
    }

    fn claude(min: i64, body: EventBody) -> Event {
        Event::new(at(min), Source::Claude, Some("s1".into()), body)
    }

    fn task(min: i64, id: &str, subject: &str, status: TaskStatus) -> Event {
        claude(
            min,
            EventBody::TaskUpsert {
                task_id: id.into(),
                subject: Some(subject.into()),
                status: Some(status),
            },
        )
    }

    fn note(min: i64, id: &str, note_type: NoteType, text: &str) -> Event {
        Event::new(
            at(min),
            Source::User,
            None,
            EventBody::NoteAdd {
                id: id.into(),
                note_type,
                text: text.into(),
            },
        )
    }

    fn board() -> Board {
        let vault = vec![
            claude(
                0,
                EventBody::UserPrompt {
                    text: "а вложения тоже переносим?".into(),
                },
            ),
            task(1, "1", "Парсер 1PUX", TaskStatus::Completed),
            task(1, "2", "Импорт вложений", TaskStatus::InProgress),
            task(
                1,
                "3",
                "UI прогресса импорта с очень длинным названием задачи",
                TaskStatus::Pending,
            ),
            claude(
                2,
                EventBody::AgentWaiting {
                    text: "Claude needs your permission to use Bash".into(),
                    reason: None,
                },
            ),
            note(
                3,
                "n1",
                NoteType::Question,
                "Bitwarden JSON — тоже поддерживать?",
            ),
            note(3, "n2", NoteType::Idea, "экспорт в CSV"),
        ];
        let snap = vec![
            claude(
                0,
                EventBody::UserPrompt {
                    text: "fix the tray icon".into(),
                },
            ),
            claude(
                5,
                EventBody::AgentStopped {
                    recap: Some("Tray icon fixed. Next: release notes.".into()),
                },
            ),
        ];
        fold([
            (ProjectMeta::for_root(Path::new("/p/DeepSnap")), snap),
            (ProjectMeta::for_root(Path::new("/p/DeepVault")), vault),
        ])
    }

    fn render(board: &Board, w: u16, selected: Option<&Item>) -> (Vec<String>, Rendered) {
        let open = HashSet::new();
        let view = View {
            selected,
            open: &open,
        };
        let r = layout_projects(board, at(4), w, &view, &EN, &Theme { color: false });
        (r.lines.iter().map(plain).collect(), r)
    }

    #[test]
    fn width_60() {
        let b = board();
        let vault = Item::Project(b.projects[0].meta.id.clone());
        let (lines, r) = render(&b, 60, Some(&vault));
        assert_eq!(
            lines,
            [
                "▌● DeepVault                            waiting for you · 2m",
                "    Claude needs your permission to use Bash               ",
                "   plan                                                  1/3",
                "     ✓ Парсер 1PUX",
                "     ◐ Импорт вложений",
                "     ○ UI прогресса импорта с очень длинным названием задачи",
                "   you: а вложения тоже переносим?",
                "   questions                                             0/1",
                "     ? Bitwarden JSON — тоже поддерживать?",
                "   ideas                                                 0/1",
                "     ◆ экспорт в CSV",
                "",
                " ✓ DeepSnap                                       done · now",
                "   you: fix the tray icon",
                "   recap: Tray icon fixed. Next: release notes.",
            ]
        );
        // Every line that can be cut is reachable: DeepVault 8 items, DeepSnap 3.
        assert_eq!(r.items.len(), 11);
        let first_note = r
            .items
            .iter()
            .find(|(i, _)| matches!(i, Item::Note(..)))
            .unwrap();
        assert_eq!(first_note.1, 8, "under the 'questions' header");
        assert!(lines.iter().all(|l| width(l) <= 60));
    }

    #[test]
    fn narrow_panel_cuts_with_ellipsis() {
        let (lines, _) = render(&board(), 30, None);
        assert!(lines.iter().all(|l| width(l) <= 30), "{lines:#?}");
        assert_eq!(lines[0], " ● DeepVault                2m");
        assert_eq!(lines[5], "     ○ UI прогресса импорта с…");
    }

    #[test]
    fn quiet_projects_share_one_line() {
        let started = |root: &str| {
            (
                ProjectMeta::for_root(Path::new(root)),
                vec![claude(
                    0,
                    EventBody::AgentStarted {
                        source: "startup".into(),
                        herdr_pane: None,
                    },
                )],
            )
        };
        let busy = (
            ProjectMeta::for_root(Path::new("/p/DeepDeck")),
            vec![claude(1, EventBody::UserPrompt { text: "go".into() })],
        );
        let b = fold([started("/p/oci-arm-catcher"), busy, started("/p/DeepSnap")]);
        let (lines, r) = render(&b, 50, None);
        assert_eq!(lines.last().unwrap(), " ○ idle: DeepSnap · oci-arm-catcher");
        // Only DeepDeck and its prompt: quiet projects are not selectable.
        assert_eq!(r.items.len(), 2);
        let (narrow, _) = render(&b, 24, None);
        assert_eq!(narrow.last().unwrap(), " ○ idle: DeepSnap · oci…");
    }

    #[test]
    fn open_item_wraps_in_full() {
        let b = board();
        let vault = b.projects[0].meta.id.clone();
        let long = Item::Task(vault.clone(), "s1".into(), "3".into());
        let open: HashSet<_> = [long.clone(), Item::Waiting(vault.clone())].into();
        let view = View {
            selected: Some(&long),
            open: &open,
        };
        let r = layout_projects(&b, at(4), 30, &view, &EN, &Theme { color: false });
        let lines: Vec<String> = r.lines.iter().map(plain).collect();
        assert_eq!(
            lines[1..8],
            [
                "    Claude needs your        ",
                "    permission to use Bash   ",
                "   plan                    1/3",
                "     ✓ Парсер 1PUX",
                "     ◐ Импорт вложений",
                "▌    ○ UI прогресса импорта с",
                "       очень длинным названием",
            ]
        );
        assert!(lines.iter().all(|l| width(l) <= 30), "{lines:#?}");
        // Every cut line is reachable by the cursor, in screen order.
        let kinds: Vec<_> = r
            .items
            .iter()
            .map(|(i, _)| std::mem::discriminant(i))
            .collect();
        // DeepVault: project, waiting, 3 tasks, prompt, 2 notes; DeepSnap: project, prompt, recap.
        assert_eq!(kinds.len(), 1 + 1 + 3 + 1 + 2 + 1 + 1 + 1);
    }

    #[test]
    fn commands_section_lists_the_runbook() {
        let set = |m: i64, name: &str, command: &str| {
            Event::new(
                at(m),
                Source::User,
                None,
                EventBody::CmdSet {
                    name: name.into(),
                    command: command.into(),
                },
            )
        };
        let note = Event::new(
            at(0),
            Source::User,
            None,
            EventBody::NoteAdd {
                id: "n1".into(),
                note_type: NoteType::Idea,
                text: "zstd".into(),
            },
        );
        let b = fold([(
            ProjectMeta::for_root(Path::new("/p/DeepDeck")),
            vec![
                note,
                set(
                    1,
                    "release",
                    "scripts/sync-public.sh main \"Release 0.3.0\"",
                ),
                set(2, "test", "cargo test"),
            ],
        )]);
        let (lines, r) = render(&b, 40, None);
        assert_eq!(
            lines[1..],
            [
                "   ideas                             0/1",
                "     ◆ zstd",
                "   commands                            2",
                "     $ release · scripts/sync-public.sh…",
                "     $ test · cargo test",
            ]
        );
        let id = b.projects[0].meta.id.clone();
        assert!(
            r.items
                .iter()
                .any(|(i, _)| *i == Item::Command(id.clone(), "test".into()))
        );
    }

    #[test]
    fn roadmap_section_without_a_live_agent() {
        let set = Event::new(
            at(0),
            Source::Agent,
            Some("old".into()),
            EventBody::RoadmapSet {
                steps: ["Restart agents", "Session 004: release v0.2"]
                    .iter()
                    .map(|s| deepdeck_core::event::PlanItem {
                        subject: (*s).into(),
                        status: TaskStatus::Pending,
                    })
                    .collect(),
            },
        );
        let done = Event::new(
            at(1),
            Source::Agent,
            Some("old".into()),
            EventBody::RoadmapStep {
                n: 1,
                status: TaskStatus::Completed,
            },
        );
        let b = fold([(
            ProjectMeta::for_root(Path::new("/p/DeepDeck")),
            vec![set, done],
        )]);
        let (lines, r) = render(&b, 40, None);
        assert_eq!(
            lines[1..],
            [
                "   roadmap                           1/2",
                "     ✓ Restart agents",
                "     ○ Session 004: release v0.2",
            ]
        );
        assert!(
            r.items
                .iter()
                .any(|(i, _)| *i == Item::Roadmap(b.projects[0].meta.id.clone(), 2))
        );
    }

    #[test]
    fn done_notes_stay_dimmed_and_summary_replaces_recap() {
        let mut events = vec![
            claude(
                0,
                EventBody::UserPrompt {
                    text: "release?".into(),
                },
            ),
            claude(
                1,
                EventBody::AgentStopped {
                    recap: Some("Short recap.".into()),
                },
            ),
            claude(
                1,
                EventBody::SessionSummary {
                    text: "Released 0.4; notes left.".into(),
                },
            ),
        ];
        for i in 0..5 {
            events.push(note(
                2,
                &format!("n{i}"),
                NoteType::Idea,
                &format!("idea {i}"),
            ));
        }
        for i in 0..4 {
            events.push(Event::new(
                at(3 + i),
                Source::User,
                None,
                EventBody::NoteDone {
                    id: format!("n{i}"),
                },
            ));
        }
        let b = fold([(ProjectMeta::for_root(Path::new("/p/DeepSnap")), events)]);
        let (lines, _) = render(&b, 40, None);
        assert_eq!(
            lines[1..],
            [
                "   you: release?",
                "   summary: Released 0.4; notes left.",
                "   ideas                             4/5",
                "     ◆ idea 4",
                "     ✓ idea 3",
                "     ✓ idea 2",
                "     ✓ idea 1",
                "     + 1 done",
            ]
        );
    }

    #[test]
    fn a_new_session_shows_the_last_one() {
        let events = vec![
            Event::new(
                at(0),
                Source::Agent,
                Some("old".into()),
                EventBody::SessionSummary {
                    text: "Parser done; UI next.".into(),
                },
            ),
            Event::new(
                at(1),
                Source::Claude,
                Some("new".into()),
                EventBody::AgentStarted {
                    source: "startup".into(),
                    herdr_pane: None,
                },
            ),
        ];
        let b = fold([(ProjectMeta::for_root(Path::new("/p/x")), events)]);
        let (lines, _) = render(&b, 50, None);
        assert_eq!(lines[1], "   last session: Parser done; UI next.");
    }

    #[test]
    fn many_done_tasks_fold() {
        let mut events: Vec<Event> = (1..=7)
            .map(|i| task(0, &i.to_string(), &format!("t{i}"), TaskStatus::Completed))
            .collect();
        events.push(task(0, "8", "last", TaskStatus::InProgress));
        let b = fold([(ProjectMeta::for_root(Path::new("/p/x")), events)]);
        let (lines, _) = render(&b, 40, None);
        assert_eq!(
            lines[1..4],
            [
                "   plan                              7/8",
                "     + 7 done",
                "     ◐ last"
            ]
        );

        let open: HashSet<_> = [Item::Project(b.projects[0].meta.id.clone())].into();
        let r = layout_projects(
            &b,
            at(4),
            40,
            &View {
                selected: None,
                open: &open,
            },
            &EN,
            &Theme { color: false },
        );
        assert_eq!(r.lines.len(), 1 + 1 + 8);
    }
}
