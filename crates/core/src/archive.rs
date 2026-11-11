//! Which events can leave `events.jsonl` for the archive without changing
//! what the board shows. Pure; the file work is in [`crate::store::rotate`].
//!
//! An old event is archived only when the fold no longer needs it:
//! - a session (its prompts, plan, waits, stops) once its last event is old —
//!   except the session holding the project's latest summary;
//! - a note once it is deleted, or done long ago, and all its events are old;
//! - roadmap events superseded by a later `roadmap.set`;
//! - saved commands replaced by a later `cmd.set` or removed by `cmd.delete`.
//!
//! Everything else stays, however old: open notes, the current roadmap.

use std::collections::{HashMap, HashSet};

use jiff::Timestamp;

use crate::event::{Event, EventBody};

/// Default age after which events move to the archive.
pub const ARCHIVE_AFTER_DAYS: i64 = 14;

/// `keep[i]` — whether `events[i]` must stay in the live log.
pub fn keep_mask(events: &[Event], cutoff: Timestamp) -> Vec<bool> {
    let old = |e: &Event| e.ts < cutoff;

    // Sessions: the time of their last event.
    let mut session_last: HashMap<&str, Timestamp> = HashMap::new();
    // Notes: last event time and whether the note is gone (deleted / done).
    let mut note_last: HashMap<&str, Timestamp> = HashMap::new();
    let mut note_gone: HashMap<&str, bool> = HashMap::new();
    let mut last_roadmap_set = None;
    // Saved commands: index of the event that defines each name's current state.
    let mut cmd_last: HashMap<&str, usize> = HashMap::new();
    let mut latest_summary_session = None;

    for (i, e) in events.iter().enumerate() {
        match note_id(&e.body) {
            Some(id) => {
                note_last.insert(id, e.ts);
                let gone = match &e.body {
                    EventBody::NoteDelete { .. } | EventBody::NoteDone { .. } => true,
                    EventBody::NoteAdd { .. } | EventBody::NoteReopen { .. } => false,
                    _ => *note_gone.get(id).unwrap_or(&false),
                };
                note_gone.insert(id, gone);
            }
            None => match &e.body {
                EventBody::RoadmapSet { .. } => last_roadmap_set = Some(i),
                EventBody::RoadmapStep { .. } => {}
                EventBody::CmdSet { name, .. } | EventBody::CmdDelete { name } => {
                    cmd_last.insert(name, i);
                }
                body => {
                    if let Some(s) = e.session.as_deref() {
                        let t = session_last.entry(s).or_insert(e.ts);
                        *t = (*t).max(e.ts);
                        if matches!(body, EventBody::SessionSummary { .. }) {
                            latest_summary_session = Some(s);
                        }
                    }
                }
            },
        }
    }
    let live_sessions: HashSet<&str> = session_last
        .iter()
        .filter(|(s, t)| **t >= cutoff || Some(**s) == latest_summary_session)
        .map(|(s, _)| *s)
        .collect();

    events
        .iter()
        .enumerate()
        .map(|(i, e)| {
            if !old(e) {
                return true;
            }
            if let Some(id) = note_id(&e.body) {
                return !(note_gone[id] && note_last[id] < cutoff);
            }
            match &e.body {
                EventBody::RoadmapSet { .. } | EventBody::RoadmapStep { .. } => {
                    last_roadmap_set.is_some_and(|last| i >= last)
                }
                // The current definition stays however old; a deletion has nothing left to hide.
                EventBody::CmdSet { name, .. } => cmd_last.get(name.as_str()) == Some(&i),
                EventBody::CmdDelete { .. } => false,
                _ => e
                    .session
                    .as_deref()
                    .is_some_and(|s| live_sessions.contains(s)),
            }
        })
        .collect()
}

fn note_id(body: &EventBody) -> Option<&str> {
    match body {
        EventBody::NoteAdd { id, .. }
        | EventBody::NoteUpdate { id, .. }
        | EventBody::NoteDone { id }
        | EventBody::NoteReopen { id }
        | EventBody::NoteDelete { id } => Some(id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::event::{NoteType, PlanItem, Source, TaskStatus};
    use crate::fold::fold_project;
    use crate::project::ProjectMeta;

    fn at(day: i64) -> Timestamp {
        Timestamp::from_second(1_790_000_000 + day * 86_400).unwrap()
    }

    fn e(day: i64, session: Option<&str>, body: EventBody) -> Event {
        Event::new(at(day), Source::Agent, session.map(Into::into), body)
    }

    fn prompt(day: i64, s: &str, text: &str) -> Event {
        e(day, Some(s), EventBody::UserPrompt { text: text.into() })
    }

    fn note(day: i64, id: &str) -> Event {
        e(
            day,
            None,
            EventBody::NoteAdd {
                id: id.into(),
                note_type: NoteType::Idea,
                text: id.into(),
                run: None,
            },
        )
    }

    fn roadmap(day: i64, step: &str) -> Event {
        e(
            day,
            Some("a"),
            EventBody::RoadmapSet {
                steps: vec![PlanItem {
                    subject: step.into(),
                    status: TaskStatus::Pending,
                }],
            },
        )
    }

    fn cmd(day: i64, name: &str, command: &str) -> Event {
        e(
            day,
            Some("old"),
            EventBody::CmdSet {
                name: name.into(),
                command: command.into(),
            },
        )
    }

    fn sample() -> Vec<Event> {
        vec![
            prompt(0, "old", "long gone"),
            e(1, Some("old"), EventBody::AgentEnded { reason: None }),
            note(1, "open"),
            note(1, "deleted"),
            e(
                2,
                None,
                EventBody::NoteDelete {
                    id: "deleted".into(),
                },
            ),
            note(2, "done-long-ago"),
            e(
                3,
                None,
                EventBody::NoteDone {
                    id: "done-long-ago".into(),
                },
            ),
            note(3, "done-recently"),
            roadmap(3, "first"),
            roadmap(4, "second"),
            e(
                5,
                Some("a"),
                EventBody::RoadmapStep {
                    n: 1,
                    status: TaskStatus::Completed,
                },
            ),
            prompt(5, "summed", "summarised"),
            e(
                5,
                Some("summed"),
                EventBody::SessionSummary {
                    text: "left X".into(),
                },
            ),
            prompt(6, "spans", "old start of a session still in use"),
            prompt(25, "spans", "recent"),
            e(
                25,
                None,
                EventBody::NoteDone {
                    id: "done-recently".into(),
                },
            ),
            prompt(26, "new", "fresh"),
            cmd(2, "release", "make release"),
            cmd(3, "secrets", "dv run --item X --sh 'gh secret set'"),
            cmd(4, "release", "make release V=1"),
            e(
                5,
                Some("old"),
                EventBody::CmdDelete {
                    name: "secrets".into(),
                },
            ),
        ]
    }

    #[test]
    fn archives_only_what_the_board_no_longer_needs() {
        let events = sample();
        let keep = keep_mask(&events, at(20));
        let archived: Vec<usize> = (0..events.len()).filter(|&i| !keep[i]).collect();
        // old session (0, 1), deleted note (3, 4), long-done note (5, 6),
        // superseded roadmap (8), replaced and deleted commands (17, 18, 20);
        // the current "release" (19) stays although its session is gone.
        assert_eq!(archived, [0, 1, 3, 4, 5, 6, 8, 17, 18, 20]);
    }

    #[test]
    fn the_board_is_unchanged_by_archiving() {
        let events = sample();
        let meta = ProjectMeta::for_root(Path::new("/tmp/demo"));
        let keep = keep_mask(&events, at(20));
        let kept: Vec<Event> = events
            .iter()
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|(e, _)| e.clone())
            .collect();
        let full = fold_project(meta.clone(), &events);
        let live = fold_project(meta, &kept);
        assert_eq!(live.notes, full.notes);
        assert_eq!(live.roadmap, full.roadmap);
        assert_eq!(live.commands, full.commands);
        assert_eq!(live.commands.len(), 1);
        assert_eq!(live.commands[0].command, "make release V=1");
        assert_eq!(live.last_summary, full.last_summary);
        assert_eq!(live.last_activity, full.last_activity);
        // Only the ended, long-gone session disappears.
        let sessions = |p: &crate::model::ProjectView| {
            let mut s: Vec<String> = p.agents.iter().map(|a| a.session.clone()).collect();
            s.sort();
            s
        };
        assert_eq!(sessions(&full), ["new", "old", "spans", "summed"]);
        assert_eq!(sessions(&live), ["new", "spans", "summed"]);
        // Long-done notes leave the done list; the recent one stays.
        let done: Vec<&str> = live.done_notes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(done, ["done-recently"]);
    }

    #[test]
    fn nothing_old_nothing_archived() {
        assert!(keep_mask(&sample(), at(-1)).iter().all(|k| *k));
        assert!(keep_mask(&[], at(0)).is_empty());
    }
}
