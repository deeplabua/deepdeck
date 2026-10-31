//! Keeps the board in sync with `~/.deepdeck/` by polling sizes: a stat per
//! project every tick is cheaper than a watcher dependency, and only new lines
//! are read. Only projects whose log grew are re-folded.

use std::collections::BTreeMap;
use std::io;
use std::time::SystemTime;

use deepdeck_core::store::{self, Paths, Registry};
use deepdeck_core::{Board, Event, ProjectId, ProjectView, fold_project};
use jiff::{SignedDuration, Timestamp};

type Stamp = (u64, Option<SystemTime>);

struct Entry {
    events: Vec<Event>,
    offset: u64,
    view: ProjectView,
}

pub struct Source {
    paths: Paths,
    registry: Option<Stamp>,
    projects: BTreeMap<ProjectId, Entry>,
}

fn stamp(path: &std::path::Path) -> Option<Stamp> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.len(), m.modified().ok()))
}

impl Source {
    pub fn new(paths: Paths) -> Self {
        Source {
            paths,
            registry: None,
            projects: BTreeMap::new(),
        }
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    /// Pick up new projects and new lines. Returns whether anything changed.
    pub fn poll(&mut self) -> io::Result<bool> {
        let mut changed = false;
        let reg_path = self.paths.registry();
        let reg_stamp = stamp(&reg_path);
        if reg_stamp != self.registry {
            self.registry = reg_stamp;
            for meta in Registry::load(&reg_path)?.projects {
                if let Some(e) = self.projects.get_mut(&meta.id) {
                    if e.view.meta != meta {
                        e.view = fold_project(meta, &e.events);
                        changed = true;
                    }
                } else {
                    self.projects.insert(
                        meta.id.clone(),
                        Entry {
                            events: Vec::new(),
                            offset: 0,
                            view: fold_project(meta, &[]),
                        },
                    );
                    changed = true;
                }
            }
        }
        for (id, e) in &mut self.projects {
            let path = self.paths.events(id);
            let len = stamp(&path).map_or(0, |s| s.0);
            if len == e.offset {
                continue;
            }
            let reset = len < e.offset;
            if reset {
                // The log was archived or replaced: start over.
                e.events.clear();
                e.offset = 0;
            }
            // A writer may hold the lock (mandatory on Windows): try next tick.
            let Ok(chunk) = store::read_from(&path, e.offset) else {
                continue;
            };
            e.offset = chunk.next_offset;
            if reset || !chunk.events.is_empty() {
                e.events.extend(chunk.events);
                e.view = fold_project(e.view.meta.clone(), &e.events);
                changed = true;
            }
        }
        Ok(changed)
    }

    /// Every project with its full log (for the Timeline).
    pub fn logs(&self) -> Vec<(&deepdeck_core::ProjectMeta, &[Event])> {
        self.projects
            .values()
            .map(|e| (&e.view.meta, e.events.as_slice()))
            .collect()
    }

    /// Projects active within `hours`, plus any project with open notes.
    pub fn board(&self, now: Timestamp, hours: i64) -> Board {
        let cutoff = now - SignedDuration::from_hours(hours);
        Board::from_projects(
            self.projects
                .values()
                .map(|e| &e.view)
                .filter(|v| !v.notes.is_empty() || v.last_activity.is_some_and(|t| t >= cutoff))
                .cloned()
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use deepdeck_core::{EventBody, ProjectMeta, Source as Src};

    use super::*;

    #[test]
    fn picks_up_new_projects_and_lines() {
        let dir = std::env::temp_dir().join(format!("ddeck-source-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = Paths::new(&dir);
        let mut src = Source::new(paths.clone());
        assert!(!src.poll().unwrap());

        let now = Timestamp::now();
        let meta = ProjectMeta::for_root(Path::new("/p/DeepVault"));
        store::ensure_registered(&paths, &meta).unwrap();
        let ev = |t: &str| {
            Event::new(
                now,
                Src::Claude,
                Some("s".into()),
                EventBody::UserPrompt { text: t.into() },
            )
        };
        store::append(&paths.events(&meta.id), &ev("one")).unwrap();
        assert!(src.poll().unwrap());
        assert!(!src.poll().unwrap());
        assert_eq!(src.board(now, 12).projects.len(), 1);

        store::append(&paths.events(&meta.id), &ev("two")).unwrap();
        assert!(src.poll().unwrap());
        let board = src.board(now, 12);
        assert_eq!(
            board.projects[0].agents[0].last_prompt.as_deref(),
            Some("two")
        );
        assert_eq!(
            src.board(now + SignedDuration::from_hours(13), 12)
                .projects
                .len(),
            0
        );

        // Archived from under a running board: the view follows the shrink.
        let later = now + SignedDuration::from_hours(1);
        assert_eq!(
            store::rotate(&paths.events(&meta.id), later)
                .unwrap()
                .archived,
            2
        );
        assert!(src.poll().unwrap());
        assert!(
            src.board(now, 12)
                .projects
                .iter()
                .all(|p| p.agents.is_empty())
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
