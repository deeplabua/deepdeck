//! Search over what the board remembers: the user's prompts, the agent's
//! recaps and summaries, notes, plans, roadmaps and saved commands.
//!
//! Pure: takes events, returns hits. Reading the logs (archive included) is the
//! caller's job.

use std::collections::{HashMap, HashSet};

use jiff::Timestamp;

use crate::event::{Event, EventBody, NoteType};

/// Where a hit's text came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    /// The user's message (as the hook saved it, cut to 200 characters).
    Prompt,
    /// The agent's recap of a finished turn.
    Reply,
    Summary,
    Note(NoteType),
    Plan,
    Roadmap,
    Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub ts: Timestamp,
    pub session: Option<String>,
    pub kind: HitKind,
    pub text: String,
}

/// A term as it is looked for: lowercased, with a short ending dropped from
/// longer words so that inflected forms match ("ключевые" finds "ключевых",
/// "keywords" finds "keyword").
pub fn stem(term: &str) -> String {
    let lower = term.to_lowercase();
    let n = lower.chars().count();
    let drop = match n {
        0..=4 => 0,
        5 => 1,
        _ => 2,
    };
    lower.chars().take(n - drop).collect()
}

/// Every term (already [`stem`]med) occurs in `text`, ignoring case.
pub fn matches(text: &str, terms: &[String]) -> bool {
    let lower = text.to_lowercase();
    !terms.is_empty() && terms.iter().all(|t| lower.contains(t.as_str()))
}

/// Events of one project in log order (archive first) → hits, oldest first.
/// Retracted prompts, deleted notes and repeats of the same text in a session
/// (plan snapshots, re-posted roadmaps) are left out; a repeat keeps its latest time.
pub fn search(events: &[Event], terms: &[String]) -> Vec<Hit> {
    let retracted: HashSet<(Option<&str>, Timestamp)> = events
        .iter()
        .filter_map(|e| match &e.body {
            EventBody::PromptRetract { at, .. } => Some((e.session.as_deref(), *at)),
            _ => None,
        })
        .collect();
    let deleted: HashSet<&str> = events
        .iter()
        .filter_map(|e| match &e.body {
            EventBody::NoteDelete { id } => Some(id.as_str()),
            _ => None,
        })
        .collect();
    let mut note_types: HashMap<&str, NoteType> = HashMap::new();

    let mut hits: Vec<Hit> = Vec::new();
    let mut seen: HashMap<(Option<String>, String), usize> = HashMap::new();
    let mut push = |hits: &mut Vec<Hit>, e: &Event, kind: HitKind, text: &str| {
        if !matches(text, terms) {
            return;
        }
        let key = (e.session.clone(), text.to_owned());
        if let Some(&i) = seen.get(&key) {
            hits[i].ts = e.ts;
            return;
        }
        seen.insert(key, hits.len());
        hits.push(Hit {
            ts: e.ts,
            session: e.session.clone(),
            kind,
            text: text.to_owned(),
        });
    };

    for e in events {
        match &e.body {
            EventBody::UserPrompt { text } => {
                if !retracted.contains(&(e.session.as_deref(), e.ts)) {
                    push(&mut hits, e, HitKind::Prompt, text);
                }
            }
            EventBody::AgentStopped { recap: Some(text) } => {
                push(&mut hits, e, HitKind::Reply, text);
            }
            EventBody::SessionSummary { text } => push(&mut hits, e, HitKind::Summary, text),
            EventBody::NoteAdd {
                id,
                note_type,
                text,
                run,
            } => {
                note_types.insert(id, *note_type);
                if deleted.contains(id.as_str()) {
                    continue;
                }
                let full = match run {
                    Some(run) => format!("{text} — {run}"),
                    None => text.clone(),
                };
                push(&mut hits, e, HitKind::Note(*note_type), &full);
            }
            EventBody::NoteUpdate { id, text } => {
                if !deleted.contains(id.as_str()) {
                    let ty = note_types
                        .get(id.as_str())
                        .copied()
                        .unwrap_or(NoteType::Task);
                    push(&mut hits, e, HitKind::Note(ty), text);
                }
            }
            EventBody::PlanSnapshot { tasks } => {
                for t in tasks {
                    push(&mut hits, e, HitKind::Plan, &t.subject);
                }
            }
            EventBody::TaskUpsert {
                subject: Some(s), ..
            } => push(&mut hits, e, HitKind::Plan, s),
            EventBody::RoadmapSet { steps } => {
                for s in steps {
                    push(&mut hits, e, HitKind::Roadmap, &s.subject);
                }
            }
            EventBody::CmdSet { name, command } => {
                push(
                    &mut hits,
                    e,
                    HitKind::Command,
                    &format!("{name}: {command}"),
                );
            }
            _ => {}
        }
    }
    hits
}

/// A piece of `text` about `max` characters long around the first term, with
/// `…` where it was cut; line breaks become spaces.
pub fn snippet(text: &str, terms: &[String], max: usize) -> String {
    let flat: Vec<char> = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .collect();
    if flat.len() <= max {
        return flat.into_iter().collect();
    }
    let lower: Vec<char> = flat.iter().flat_map(|c| c.to_lowercase()).collect();
    // to_lowercase can change the length (rare); fall back to the start then.
    let at = if lower.len() == flat.len() {
        terms
            .iter()
            .filter_map(|t| {
                let t: Vec<char> = t.chars().collect();
                lower
                    .windows(t.len().max(1))
                    .position(|w| w == t.as_slice())
            })
            .min()
            .unwrap_or(0)
    } else {
        0
    };
    let mut start = at.saturating_sub(max / 4).min(flat.len() - max);
    let mut end = start + max;
    // Cut between words when one is near: after a space at the start, before one at the end.
    if start > 0
        && flat[start - 1] != ' '
        && let Some(i) = flat[start..at.max(start)]
            .iter()
            .take(12)
            .position(|c| *c == ' ')
    {
        start += i + 1;
    }
    if end < flat.len()
        && flat[end] != ' '
        && let Some(i) = flat[start..end]
            .iter()
            .rev()
            .take(12)
            .position(|c| *c == ' ')
    {
        end -= i + 1;
    }
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.extend(&flat[start..end]);
    if end < flat.len() {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{PlanItem, Source, TaskStatus};

    fn at(min: i64) -> Timestamp {
        Timestamp::from_second(1_793_430_000 + min * 60).unwrap()
    }

    fn ev(min: i64, session: &str, body: EventBody) -> Event {
        Event::new(at(min), Source::Claude, Some(session.into()), body)
    }

    fn terms(q: &str) -> Vec<String> {
        q.split_whitespace().map(stem).collect()
    }

    #[test]
    fn stems_catch_inflected_forms() {
        assert_eq!(stem("Ключевые"), "ключев");
        assert_eq!(stem("keywords"), "keywor");
        assert_eq!(stem("aso"), "aso");
        assert!(matches("по ключевым словам", &terms("ключевые слова")));
        assert!(!matches("по ключевым", &terms("ключевые слова")));
        assert!(!matches("anything", &[]));
    }

    #[test]
    fn finds_prompts_replies_notes_and_dedupes_plans() {
        let plan = |s: &str| PlanItem {
            subject: s.into(),
            status: TaskStatus::Pending,
        };
        let events = vec![
            ev(
                0,
                "a",
                EventBody::UserPrompt {
                    text: "развить ASO инструмент в админке".into(),
                },
            ),
            ev(
                1,
                "a",
                EventBody::PlanSnapshot {
                    tasks: vec![plan("ASO funnel"), plan("other")],
                },
            ),
            ev(
                2,
                "a",
                EventBody::PlanSnapshot {
                    tasks: vec![plan("ASO funnel")],
                },
            ),
            ev(
                3,
                "a",
                EventBody::AgentStopped {
                    recap: Some("Раздел ASO готов".into()),
                },
            ),
            ev(
                4,
                "a",
                EventBody::NoteAdd {
                    id: "n1".into(),
                    note_type: NoteType::Handoff,
                    text: "Put the ASC key".into(),
                    run: Some("ssh deploy aso".into()),
                },
            ),
            ev(
                5,
                "a",
                EventBody::NoteAdd {
                    id: "n2".into(),
                    note_type: NoteType::Idea,
                    text: "ASO deleted idea".into(),
                    run: None,
                },
            ),
            ev(6, "a", EventBody::NoteDelete { id: "n2".into() }),
            ev(
                7,
                "b",
                EventBody::UserPrompt {
                    text: "aso for the wrong session".into(),
                },
            ),
            ev(
                8,
                "b",
                EventBody::PromptRetract {
                    at: at(7),
                    to: None,
                },
            ),
        ];
        let hits = search(&events, &terms("aso"));
        let got: Vec<(HitKind, &str, Timestamp)> = hits
            .iter()
            .map(|h| (h.kind, h.text.as_str(), h.ts))
            .collect();
        assert_eq!(
            got,
            [
                (HitKind::Prompt, "развить ASO инструмент в админке", at(0)),
                (HitKind::Plan, "ASO funnel", at(2)),
                (HitKind::Reply, "Раздел ASO готов", at(3)),
                (
                    HitKind::Note(NoteType::Handoff),
                    "Put the ASC key — ssh deploy aso",
                    at(4)
                ),
            ]
        );
    }

    #[test]
    fn snippet_centres_on_the_match() {
        let t = terms("funnel");
        assert_eq!(snippet("short\ntext", &t, 40), "short text");
        let long = "a ".repeat(40) + "the funnel is here " + &"b ".repeat(40);
        let s = snippet(&long, &t, 30);
        assert!(s.starts_with('…') && s.ends_with('…'), "{s}");
        assert!(s.contains("funnel"), "{s}");
        assert!(s.chars().count() <= 32, "{s}");
        // Cut between words: no half word at either edge.
        assert!(s.starts_with("…a ") && s.ends_with(" b…"), "{s}");
    }
}
