//! `ddeck find`: which session talked about something, across all projects.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use deepdeck_core::event::CLI_SESSION;
use deepdeck_core::search::{self, Hit, HitKind};
use deepdeck_core::store::{self, Paths};
use deepdeck_core::{Event, EventBody, NoteType, ProjectMeta};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::json;

/// Sessions shown without `--all`, and hits per session.
const SESSIONS: usize = 8;
const HITS: usize = 4;
const SNIPPET: usize = 110;

/// Hits of one session (or of the board itself: notes and commands posted
/// from a terminal), newest first.
#[derive(Debug)]
pub struct Group {
    pub meta: ProjectMeta,
    pub session: Option<String>,
    pub ended: bool,
    pub hits: Vec<Hit>,
}

impl Group {
    fn last(&self) -> Timestamp {
        self.hits[0].ts
    }
}

/// Group a project's hits by session, newest hit first in each.
pub fn group(meta: &ProjectMeta, events: &[Event], terms: &[String]) -> Vec<Group> {
    let mut ended: HashMap<&str, bool> = HashMap::new();
    for e in events {
        if let Some(s) = e.session.as_deref() {
            ended.insert(s, matches!(e.body, EventBody::AgentEnded { .. }));
        }
    }
    let mut groups: Vec<Group> = Vec::new();
    for hit in search::search(events, terms) {
        let session = hit.session.clone().filter(|s| s != CLI_SESSION);
        let g = match groups.iter_mut().find(|g| g.session == session) {
            Some(g) => g,
            None => {
                groups.push(Group {
                    meta: meta.clone(),
                    ended: session
                        .as_deref()
                        .is_some_and(|s| ended.get(s).copied().unwrap_or(false)),
                    session,
                    hits: Vec::new(),
                });
                groups.last_mut().expect("just pushed")
            }
        };
        g.hits.push(hit);
    }
    for g in &mut groups {
        g.hits.sort_by_key(|h| std::cmp::Reverse(h.ts));
    }
    groups
}

pub fn find(
    paths: &Paths,
    query: &[String],
    project: Option<&str>,
    all: bool,
    as_json: bool,
) -> Result<String> {
    let terms: Vec<String> = query
        .iter()
        .flat_map(|q| q.split_whitespace())
        .map(search::stem)
        .filter(|t| !t.is_empty())
        .collect();
    anyhow::ensure!(!terms.is_empty(), "nothing to find");
    let reg = store::Registry::load(&paths.registry())?;
    let metas: Vec<ProjectMeta> = match project {
        Some(q) => vec![crate::cli::resolve(&reg, q)?.clone()],
        None => reg.projects.clone(),
    };
    let mut groups = Vec::new();
    for (meta, events) in load(paths, metas)? {
        groups.extend(group(&meta, &events, &terms));
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.last()));
    if as_json {
        return Ok(to_json(&groups));
    }
    Ok(render(
        &groups,
        &terms,
        all,
        &TimeZone::system(),
        home().as_deref(),
    ))
}

/// These projects' events, archive included.
pub fn load(paths: &Paths, metas: Vec<ProjectMeta>) -> Result<Vec<(ProjectMeta, Vec<Event>)>> {
    metas
        .into_iter()
        .map(|meta| {
            let events = store::load_with_archive(paths, &meta.id)?;
            Ok((meta, events))
        })
        .collect()
}

/// Every registered project's events, archive included (the board's search).
pub fn load_all(paths: &Paths) -> Result<Vec<(ProjectMeta, Vec<Event>)>> {
    load(paths, store::Registry::load(&paths.registry())?.projects)
}

fn home() -> Option<String> {
    std::env::var("HOME").ok().filter(|h| !h.is_empty())
}

pub fn kind_label(kind: HitKind) -> &'static str {
    match kind {
        HitKind::Prompt => "you",
        HitKind::Reply => "agent",
        HitKind::Summary => "summary",
        HitKind::Note(NoteType::Task) => "task",
        HitKind::Note(NoteType::Question) => "question",
        HitKind::Note(NoteType::Idea) => "idea",
        HitKind::Note(NoteType::Handoff) => "handoff",
        HitKind::Plan => "plan",
        HitKind::Roadmap => "roadmap",
        HitKind::Command => "command",
    }
}

fn tilde(path: &Path, home: Option<&str>) -> String {
    let p = path.display().to_string();
    match home {
        Some(h) if p.starts_with(h) => format!("~{}", &p[h.len()..]),
        _ => p,
    }
}

/// Plain text: a header per session, its newest hits, how to get back to it.
pub fn render(
    groups: &[Group],
    terms: &[String],
    all: bool,
    tz: &TimeZone,
    home: Option<&str>,
) -> String {
    if groups.is_empty() {
        return "nothing found\n".to_owned();
    }
    let total: usize = groups.iter().map(|g| g.hits.len()).sum();
    let mut out = format!(
        "{total} {} in {} {}\n",
        if total == 1 { "match" } else { "matches" },
        groups.len(),
        if groups.len() == 1 { "place" } else { "places" },
    );
    let shown = if all { groups.len() } else { SESSIONS };
    for g in groups.iter().take(shown) {
        let when = |ts: Timestamp| ts.to_zoned(tz.clone()).strftime("%m-%d %H:%M").to_string();
        let first = g.hits.last().map_or(g.last(), |h| h.ts);
        let span = if when(first)[..5] == when(g.last())[..5] {
            when(g.last())
        } else {
            format!("{} – {}", &when(first)[..5], when(g.last()))
        };
        let who = match &g.session {
            Some(s) => format!("session {}", &s[..s.len().min(8)]),
            None => "board".to_owned(),
        };
        let state = if g.ended { " · ended" } else { "" };
        out.push_str(&format!("\n{} · {who} · {span}{state}\n", g.meta.name));
        let n = if all { g.hits.len() } else { HITS };
        for h in g.hits.iter().take(n) {
            out.push_str(&format!(
                "  {}  {:<8} {}\n",
                when(h.ts),
                kind_label(h.kind),
                search::snippet(&h.text, terms, SNIPPET)
            ));
        }
        if g.hits.len() > n {
            out.push_str(&format!("  + {} more\n", g.hits.len() - n));
        }
        if let Some(s) = &g.session {
            out.push_str(&format!(
                "  resume: cd {} && claude --resume {s}\n",
                tilde(&g.meta.root, home)
            ));
        }
    }
    if groups.len() > shown {
        out.push_str(&format!(
            "\n+ {} more places (--all shows everything)\n",
            groups.len() - shown
        ));
    }
    out
}

fn to_json(groups: &[Group]) -> String {
    let v: Vec<_> = groups
        .iter()
        .map(|g| {
            json!({
                "project": g.meta.name,
                "root": g.meta.root,
                "session": g.session,
                "ended": g.ended,
                "hits": g.hits.iter().map(|h| json!({
                    "ts": h.ts.to_string(),
                    "kind": kind_label(h.kind),
                    "text": h.text,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::to_string_pretty(&v).unwrap_or_default() + "\n"
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use deepdeck_core::Source;

    use super::*;

    fn at(min: i64) -> Timestamp {
        Timestamp::from_second(1_793_430_000 + min * 60).unwrap()
    }

    fn ev(min: i64, session: &str, body: EventBody) -> Event {
        Event::new(at(min), Source::Claude, Some(session.into()), body)
    }

    #[test]
    fn groups_by_session_newest_first_with_resume() {
        let meta = ProjectMeta::for_root(Path::new("/home/u/Projects/DeepShrink"));
        let events = vec![
            ev(
                0,
                "76baef01-9fbe",
                EventBody::UserPrompt {
                    text: "по каким ключевым словам юзеры попали к нам?".into(),
                },
            ),
            ev(
                5,
                "76baef01-9fbe",
                EventBody::AgentStopped {
                    recap: Some("Заменил ключевые слова en-US".into()),
                },
            ),
            ev(9, "76baef01-9fbe", EventBody::AgentEnded { reason: None }),
            ev(
                20,
                "cli",
                EventBody::NoteAdd {
                    id: "n1".into(),
                    note_type: NoteType::Idea,
                    text: "Mac keywords: drop repeats".into(),
                    run: None,
                },
            ),
            ev(
                30,
                "other",
                EventBody::UserPrompt {
                    text: "nothing here".into(),
                },
            ),
        ];
        let terms: Vec<String> = ["ключевые", "keyword"]
            .iter()
            .map(|t| search::stem(t))
            .collect();
        // Either term per hit is not enough: both must match.
        assert!(group(&meta, &events, &terms).is_empty());

        let terms = vec![search::stem("ключевые")];
        let groups = group(&meta, &events, &terms);
        assert_eq!(groups.len(), 1);
        let out = render(&groups, &terms, false, &TimeZone::UTC, Some("/home/u"));
        assert_eq!(
            out,
            "2 matches in 1 place\n\
             \n\
             DeepShrink · session 76baef01 · 10-31 07:05 · ended\n\
             \x20 10-31 07:05  agent    Заменил ключевые слова en-US\n\
             \x20 10-31 07:00  you      по каким ключевым словам юзеры попали к нам?\n\
             \x20 resume: cd ~/Projects/DeepShrink && claude --resume 76baef01-9fbe\n"
        );

        let terms = vec![search::stem("keywords")];
        let groups = group(&meta, &events, &terms);
        let out = render(&groups, &terms, false, &TimeZone::UTC, None);
        assert!(out.contains("DeepShrink · board · "), "{out}");
        assert!(out.contains("idea     Mac keywords: drop repeats"), "{out}");
        assert!(!out.contains("resume:"), "{out}");
    }
}
