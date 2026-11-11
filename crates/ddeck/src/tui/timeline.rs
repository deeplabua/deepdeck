//! The Timeline screen (mock-up B): today's events from every project, newest
//! first — "what were we talking about 15 minutes ago". Pure layout.

use std::collections::HashMap;

use deepdeck_core::{Event, EventBody, NoteType, ProjectId, ProjectMeta, TaskStatus};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::strings::Strings;
use super::text::{fit, row, width};
use super::theme::Theme;

/// Stable colour per project (yellow is reserved for "waiting for you").
pub fn project_style(id: &ProjectId, th: &Theme) -> Style {
    const PALETTE: [Color; 6] = [
        Color::Cyan,
        Color::Magenta,
        Color::Green,
        Color::Blue,
        Color::LightRed,
        Color::LightCyan,
    ];
    if !th.color {
        return Style::new();
    }
    let h = id.as_str().bytes().fold(0usize, |h, b| {
        h.wrapping_mul(31).wrapping_add(usize::from(b))
    });
    Style::new().fg(PALETTE[h % PALETTE.len()])
}

/// One rendered timeline row before layout: what happened, in which project.
struct Entry<'a> {
    meta: &'a ProjectMeta,
    ts: Timestamp,
    mark: &'static str,
    mark_style: Style,
    text: String,
    is_prompt: bool,
}

/// `projects`: every project with its log. Only events of `now`'s local day are
/// shown; `filter` keeps one project.
pub fn layout_timeline(
    projects: &[(&ProjectMeta, &[Event])],
    now: Timestamp,
    tz: &TimeZone,
    width_cols: u16,
    filter: Option<&ProjectId>,
    t: &Strings,
    th: &Theme,
) -> Vec<Line<'static>> {
    let total = usize::from(width_cols);
    let today = now.to_zoned(tz.clone()).date();
    let mut entries: Vec<Entry<'_>> = Vec::new();

    for (meta, events) in projects {
        if filter.is_some_and(|f| *f != meta.id) {
            continue;
        }
        // Subjects and note texts are only in the creating event: resolve forward.
        let mut subjects: HashMap<(&str, String), String> = HashMap::new();
        let mut notes: HashMap<&str, (NoteType, String)> = HashMap::new();
        let mut roadmap: Vec<String> = Vec::new();
        // Retracted prompts (session, sent at) → where they were meant to go.
        let retracted: HashMap<(&str, Timestamp), Option<&str>> = events
            .iter()
            .filter_map(|e| match &e.body {
                EventBody::PromptRetract { at, to } => {
                    Some(((e.session.as_deref().unwrap_or(""), *at), to.as_deref()))
                }
                _ => None,
            })
            .collect();
        // Sessions whose next recap answers a retracted prompt.
        let mut hide_recap: HashMap<&str, bool> = HashMap::new();
        for e in *events {
            let session = e.session.as_deref().unwrap_or("");
            let shown = e.ts.to_zoned(tz.clone()).date() == today;
            let entry = |mark, mark_style, text: String| Entry {
                meta,
                ts: e.ts,
                mark,
                mark_style,
                text,
                is_prompt: false,
            };
            let item = match &e.body {
                EventBody::UserPrompt { text } if deepdeck_core::hooks::is_system_message(text) => {
                    None
                }
                EventBody::UserPrompt { .. } if retracted.contains_key(&(session, e.ts)) => {
                    hide_recap.insert(session, true);
                    let text = match retracted[&(session, e.ts)] {
                        Some(to) => (t.tl_moved_to)(to),
                        None => t.tl_retracted.to_owned(),
                    };
                    Some(entry("↩", th.dim(), text))
                }
                EventBody::UserPrompt { text } => {
                    hide_recap.insert(session, false);
                    Some(Entry {
                        is_prompt: true,
                        ..entry("›", th.name(), deepdeck_core::hooks::clean_prompt(text))
                    })
                }
                EventBody::AgentStopped { .. }
                    if hide_recap.insert(session, false) == Some(true) =>
                {
                    None
                }
                EventBody::AgentWaiting { text, .. } => {
                    Some(entry("●", th.waiting(), text.clone()))
                }
                EventBody::AgentStopped { recap } => {
                    recap.clone().map(|r| entry("✓", th.done(), r))
                }
                // Session start/end is noise here and would split conversation contexts.
                EventBody::AgentStarted { .. } | EventBody::AgentEnded { .. } => None,
                EventBody::PlanSnapshot { tasks } => {
                    // The fold numbers snapshot steps "1".."n".
                    for (i, task) in tasks.iter().enumerate() {
                        subjects.insert((session, (i + 1).to_string()), task.subject.clone());
                    }
                    Some(entry("◐", th.working(), (t.tl_plan)(tasks.len())))
                }
                EventBody::TaskUpsert {
                    task_id,
                    subject,
                    status,
                } => {
                    if let Some(s) = subject {
                        subjects.insert((session, task_id.clone()), s.clone());
                    }
                    let subject = subjects
                        .get(&(session, task_id.clone()))
                        .cloned()
                        .unwrap_or_else(|| format!("#{task_id}"));
                    match status {
                        Some(TaskStatus::Completed) => Some(entry("✓", th.done(), subject)),
                        Some(TaskStatus::InProgress) => Some(entry("◐", th.working(), subject)),
                        _ => None,
                    }
                }
                EventBody::NoteAdd {
                    id,
                    note_type,
                    text,
                    ..
                } => {
                    notes.insert(id.as_str(), (*note_type, text.clone()));
                    let (m, s) = note_mark(*note_type, th);
                    Some(entry(m, s, text.clone()))
                }
                EventBody::NoteUpdate { id, text } => {
                    if let Some(n) = notes.get_mut(id.as_str()) {
                        n.1 = text.clone();
                    }
                    None
                }
                EventBody::NoteDone { id } => notes
                    .get(id.as_str())
                    .map(|(_, text)| entry("✓", th.done(), text.clone())),
                EventBody::NoteDelete { .. }
                | EventBody::NoteReopen { .. }
                | EventBody::PromptRetract { .. } => None,
                EventBody::SessionSummary { text } => Some(entry("Σ", th.accent(), text.clone())),
                EventBody::CmdSet { name, .. } => Some(entry("$", th.accent(), (t.tl_cmd)(name))),
                EventBody::CmdDelete { .. } => None,
                EventBody::RoadmapSet { steps } => {
                    roadmap = steps.iter().map(|s| s.subject.clone()).collect();
                    Some(entry("◇", th.accent(), (t.tl_roadmap)(steps.len())))
                }
                EventBody::RoadmapStep { n, status } => {
                    let subject = n.checked_sub(1).and_then(|i| roadmap.get(i)).cloned();
                    match (subject, status) {
                        (Some(s), TaskStatus::Completed) => Some(entry("◆", th.done(), s)),
                        (Some(s), TaskStatus::InProgress) => Some(entry("◇", th.working(), s)),
                        _ => None,
                    }
                }
            };
            if shown && let Some(item) = item {
                entries.push(item);
            }
        }
    }

    entries.sort_by_key(|e| std::cmp::Reverse(e.ts));
    let name_w = entries
        .iter()
        .map(|e| width(&e.meta.name))
        .max()
        .unwrap_or(0)
        .min(12)
        .min(total / 5);

    // Context of an event: the project you last wrote to at that moment (its own
    // project before any prompt). A divider marks where the context changed;
    // `entries` is newest first, so walk it oldest first.
    let mut context: Vec<&ProjectMeta> = Vec::with_capacity(entries.len());
    let mut last_prompt: Option<&ProjectMeta> = None;
    for e in entries.iter().rev() {
        if e.is_prompt {
            last_prompt = Some(e.meta);
        }
        context.push(last_prompt.unwrap_or(e.meta));
    }
    context.reverse();

    let mut lines = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        if i > 0 && context[i].id != context[i - 1].id {
            // `e` is the newest event of the older context.
            let ago = (t.ago)(now.duration_since(e.ts).as_secs());
            let label = format!(" {} ", (t.tl_you_were_in)(&ago, &context[i].name));
            let side = total.saturating_sub(width(&label)) / 2;
            let rest = total.saturating_sub(width(&label) + side);
            let divider = format!("{}{label}{}", "─".repeat(side), "─".repeat(rest));
            lines.push(Line::styled(fit(&divider, total), th.dim()));
        }
        let hhmm = e.ts.to_zoned(tz.clone()).strftime("%H:%M").to_string();
        let name = fit(&e.meta.name, name_w);
        let pad = name_w.saturating_sub(width(&name));
        lines.push(row(
            vec![
                Span::styled(format!(" {hhmm} "), th.dim()),
                Span::styled(
                    format!("{name}{} ", " ".repeat(pad)),
                    project_style(&e.meta.id, th),
                ),
                Span::styled(format!("{} ", e.mark), e.mark_style),
            ],
            &e.text,
            Style::new(),
            None,
            total,
        ));
    }
    if lines.is_empty() {
        lines.push(Line::styled(format!(" {}", t.tl_empty), th.dim()));
    }
    lines
}

fn note_mark(t: NoteType, th: &Theme) -> (&'static str, Style) {
    match t {
        NoteType::Question => ("?", th.question()),
        NoteType::Idea => ("◆", th.idea()),
        NoteType::Task => ("○", th.dim()),
        NoteType::Handoff => ("→", th.waiting()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use deepdeck_core::event::PlanItem;
    use deepdeck_core::{Event, EventBody, Source};

    use super::super::strings::EN;
    use super::super::text::plain;
    use super::*;

    fn at(h: i8, m: i8) -> Timestamp {
        jiff::civil::date(2026, 10, 31)
            .at(h, m, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap()
            .timestamp()
    }

    fn ev(ts: Timestamp, session: &str, body: EventBody) -> Event {
        Event::new(ts, Source::Claude, Some(session.into()), body)
    }

    #[test]
    fn retracted_prompt_and_its_answer_are_hidden() {
        let deck = ProjectMeta::for_root(Path::new("/p/DeepDeck"));
        let events = vec![
            ev(
                at(10, 0),
                "a",
                EventBody::UserPrompt {
                    text: "what next?".into(),
                },
            ),
            ev(
                at(10, 1),
                "a",
                EventBody::AgentStopped {
                    recap: Some("Plan: 004.".into()),
                },
            ),
            ev(
                at(10, 2),
                "a",
                EventBody::UserPrompt {
                    text: "chips No l1O0 tooltip?".into(),
                },
            ),
            ev(
                at(10, 3),
                "a",
                EventBody::AgentStopped {
                    recap: Some("Looks like DeepVault.".into()),
                },
            ),
            ev(
                at(10, 4),
                "a",
                EventBody::UserPrompt {
                    text: "oops, wrong session".into(),
                },
            ),
            ev(
                at(10, 4),
                "a",
                EventBody::PromptRetract {
                    at: at(10, 2),
                    to: Some("DeepVault".into()),
                },
            ),
        ];
        let projects = [(&deck, events.as_slice())];
        let lines: Vec<String> = layout_timeline(
            &projects,
            at(10, 5),
            &TimeZone::UTC,
            60,
            None,
            &EN,
            &Theme { color: false },
        )
        .iter()
        .map(plain)
        .collect();
        assert_eq!(
            lines,
            [
                " 10:04 DeepDeck › oops, wrong session",
                " 10:02 DeepDeck ↩ message moved to DeepVault",
                " 10:01 DeepDeck ✓ Plan: 004.",
                " 10:00 DeepDeck › what next?",
            ]
        );
    }

    #[test]
    fn newest_first_with_context_switch() {
        let vault = ProjectMeta::for_root(Path::new("/p/DeepVault"));
        let snap = ProjectMeta::for_root(Path::new("/p/DeepSnap"));
        let vault_events = vec![
            ev(
                at(0, 5),
                "a",
                EventBody::UserPrompt {
                    text: "yesterday? no, today early".into(),
                },
            ),
            ev(
                at(9, 0),
                "a",
                EventBody::UserPrompt {
                    text: "а вложения тоже переносим?".into(),
                },
            ),
            ev(
                at(9, 1),
                "a",
                EventBody::PlanSnapshot {
                    tasks: vec![
                        PlanItem {
                            subject: "Parser".into(),
                            status: TaskStatus::Pending,
                        },
                        PlanItem {
                            subject: "Import".into(),
                            status: TaskStatus::Pending,
                        },
                    ],
                },
            ),
            ev(
                at(9, 3),
                "a",
                EventBody::TaskUpsert {
                    task_id: "1".into(),
                    subject: None,
                    status: Some(TaskStatus::Completed),
                },
            ),
            ev(
                at(9, 4),
                "a",
                EventBody::AgentWaiting {
                    text: "Claude needs your permission".into(),
                    reason: None,
                },
            ),
        ];
        let snap_events = vec![
            ev(
                at(9, 10),
                "b",
                EventBody::UserPrompt {
                    text: "fix the tray icon".into(),
                },
            ),
            ev(
                at(9, 12),
                "b",
                EventBody::AgentStopped {
                    recap: Some("Tray icon fixed.".into()),
                },
            ),
        ];
        let projects = [
            (&vault, vault_events.as_slice()),
            (&snap, snap_events.as_slice()),
        ];
        let th = Theme { color: false };
        let lines: Vec<String> =
            layout_timeline(&projects, at(9, 20), &TimeZone::UTC, 60, None, &EN, &th)
                .iter()
                .map(plain)
                .collect();
        assert_eq!(
            lines,
            [
                " 09:12 DeepSnap  ✓ Tray icon fixed.",
                " 09:10 DeepSnap  › fix the tray icon",
                "────────────── 16m ago you were in DeepVault ───────────────",
                " 09:04 DeepVault ● Claude needs your permission",
                " 09:03 DeepVault ✓ Parser",
                " 09:01 DeepVault ◐ plan: 2 steps",
                " 09:00 DeepVault › а вложения тоже переносим?",
                " 00:05 DeepVault › yesterday? no, today early",
            ]
        );
        assert!(lines.iter().all(|l| width(l) <= 60));

        let only: Vec<String> = layout_timeline(
            &projects,
            at(9, 20),
            &TimeZone::UTC,
            60,
            Some(&snap.id),
            &EN,
            &th,
        )
        .iter()
        .map(plain)
        .collect();
        assert_eq!(only.len(), 2);

        // Next day: nothing from yesterday.
        let tomorrow = at(9, 20) + jiff::SignedDuration::from_hours(24);
        let empty: Vec<String> =
            layout_timeline(&projects, tomorrow, &TimeZone::UTC, 60, None, &EN, &th)
                .iter()
                .map(plain)
                .collect();
        assert_eq!(empty, [format!(" {}", EN.tl_empty)]);
    }
}
