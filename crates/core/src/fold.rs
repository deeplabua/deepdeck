//! Events → board. Pure functions; the only input is the log.

use crate::event::{Event, EventBody, TaskStatus};
use crate::model::{
    AgentState, AgentView, Board, Note, ProjectView, SavedCommand, SessionSummary, Task,
};
use crate::project::ProjectMeta;

/// How many recent prompts an agent keeps for `ddeck retract`.
pub const RECENT_PROMPTS: usize = 20;

/// Fold every project's events into a sorted board.
pub fn fold(projects: impl IntoIterator<Item = (ProjectMeta, Vec<Event>)>) -> Board {
    Board::from_projects(
        projects
            .into_iter()
            .map(|(meta, events)| fold_project(meta, &events))
            .collect(),
    )
}

/// How the board lists projects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    /// By name: projects stay where they are.
    #[default]
    Name,
    /// Waiting for you → working → idle → ended, then most recent first.
    Attention,
}

impl Board {
    /// Re-sort in place; the fold itself returns [`Order::Attention`].
    pub fn sort(&mut self, order: Order) {
        match order {
            Order::Attention => sort_by_attention(&mut self.projects),
            Order::Name => self.projects.sort_by(|a, b| {
                a.meta
                    .name
                    .to_lowercase()
                    .cmp(&b.meta.name.to_lowercase())
                    .then(a.meta.root.cmp(&b.meta.root))
            }),
        }
    }

    /// Board from already folded projects (the TUI re-folds only what changed).
    /// Sorted: waiting for you → working → idle/done → ended, then most recent first.
    pub fn from_projects(mut projects: Vec<ProjectView>) -> Board {
        sort_by_attention(&mut projects);
        Board { projects }
    }
}

fn sort_by_attention(projects: &mut [ProjectView]) {
    projects.sort_by(|a, b| {
        let rank = |p: &ProjectView| p.headline().map_or(u8::MAX, |a| a.state.rank());
        rank(a)
            .cmp(&rank(b))
            .then(b.last_activity.cmp(&a.last_activity))
            .then(a.meta.name.cmp(&b.meta.name))
    });
}

/// Fold one project's log (in log order).
pub fn fold_project(meta: ProjectMeta, events: &[Event]) -> ProjectView {
    let mut agents: Vec<AgentView> = Vec::new();
    let mut notes: Vec<Note> = Vec::new();
    let mut done_notes: Vec<Note> = Vec::new();
    let mut roadmap: Vec<Task> = Vec::new();
    let mut commands: Vec<SavedCommand> = Vec::new();
    let mut last_activity = None;

    for e in events {
        last_activity = last_activity.max(Some(e.ts));
        match &e.body {
            EventBody::NoteAdd {
                id,
                note_type,
                text,
                run,
            } => {
                notes.retain(|n| &n.id != id);
                done_notes.retain(|n| &n.id != id);
                notes.push(Note {
                    id: id.clone(),
                    note_type: *note_type,
                    text: text.clone(),
                    created: e.ts,
                    done_at: None,
                    run: run.clone(),
                });
            }
            EventBody::NoteUpdate { id, text } => {
                if let Some(n) = notes.iter_mut().find(|n| &n.id == id) {
                    n.text = text.clone();
                }
            }
            EventBody::NoteDone { id } => {
                if let Some(i) = notes.iter().position(|n| &n.id == id) {
                    let mut n = notes.remove(i);
                    n.done_at = Some(e.ts);
                    done_notes.insert(0, n);
                }
            }
            EventBody::NoteReopen { id } => {
                if let Some(i) = done_notes.iter().position(|n| &n.id == id) {
                    let mut n = done_notes.remove(i);
                    n.done_at = None;
                    // Back in creation order.
                    let at = notes.partition_point(|o| o.created <= n.created);
                    notes.insert(at, n);
                }
            }
            EventBody::NoteDelete { id } => {
                notes.retain(|n| &n.id != id);
                done_notes.retain(|n| &n.id != id);
            }
            EventBody::RoadmapSet { steps } => {
                roadmap = steps
                    .iter()
                    .enumerate()
                    .map(|(i, s)| Task {
                        id: (i + 1).to_string(),
                        subject: s.subject.clone(),
                        status: s.status,
                    })
                    .collect();
            }
            EventBody::RoadmapStep { n, status } => {
                if let Some(t) = n.checked_sub(1).and_then(|i| roadmap.get_mut(i)) {
                    t.status = *status;
                }
            }
            EventBody::CmdSet { name, command } => {
                let cmd = SavedCommand {
                    name: name.clone(),
                    command: command.clone(),
                    at: e.ts,
                };
                match commands.iter_mut().find(|c| &c.name == name) {
                    Some(c) => *c = cmd,
                    None => commands.push(cmd),
                }
            }
            EventBody::CmdDelete { name } => commands.retain(|c| &c.name != name),
            body => {
                let Some(session) = &e.session else { continue };
                let agent = agent_mut(&mut agents, session, e);
                apply_agent(agent, body, e);
            }
        }
    }

    agents.sort_by_key(|a| std::cmp::Reverse(a.last_activity));
    let last_summary = events
        .iter()
        .rev()
        .find_map(|e| match (&e.body, &e.session) {
            (EventBody::SessionSummary { text }, Some(session)) => Some(SessionSummary {
                session: session.clone(),
                at: e.ts,
                text: text.clone(),
            }),
            _ => None,
        });
    ProjectView {
        meta,
        agents,
        notes,
        done_notes,
        last_summary,
        roadmap,
        commands,
        last_activity,
    }
}

fn agent_mut<'a>(agents: &'a mut Vec<AgentView>, session: &str, e: &Event) -> &'a mut AgentView {
    if let Some(i) = agents.iter().position(|a| a.session == session) {
        return &mut agents[i];
    }
    agents.push(AgentView {
        session: session.to_owned(),
        state: AgentState::Idle { since: e.ts },
        tasks: Vec::new(),
        last_prompt: None,
        recent_prompts: Vec::new(),
        last_recap: None,
        summary: None,
        herdr_pane: None,
        last_activity: e.ts,
    });
    agents.last_mut().expect("just pushed")
}

fn apply_agent(agent: &mut AgentView, body: &EventBody, e: &Event) {
    agent.last_activity = agent.last_activity.max(e.ts);
    let ts = e.ts;
    match body {
        EventBody::AgentStarted { herdr_pane, .. } => {
            agent.state = AgentState::Idle { since: ts };
            if herdr_pane.is_some() {
                agent.herdr_pane = herdr_pane.clone();
            }
        }
        // Logs written before the hook filtered these may still contain them.
        EventBody::UserPrompt { text } if crate::hooks::is_system_message(text) => {}
        EventBody::UserPrompt { text } => {
            let text = crate::hooks::clean_prompt(text);
            agent.recent_prompts.push((ts, text.clone()));
            if agent.recent_prompts.len() > RECENT_PROMPTS {
                agent.recent_prompts.remove(0);
            }
            agent.last_prompt = Some(text);
            agent.state = AgentState::Working { since: ts };
        }
        EventBody::TaskUpsert {
            task_id,
            subject,
            status,
        } => {
            upsert_task(&mut agent.tasks, task_id, subject.as_deref(), *status);
            resume(agent, ts);
        }
        EventBody::PlanSnapshot { tasks } => {
            agent.tasks = tasks
                .iter()
                .enumerate()
                .map(|(i, t)| Task {
                    id: (i + 1).to_string(),
                    subject: t.subject.clone(),
                    status: t.status,
                })
                .collect();
            resume(agent, ts);
        }
        EventBody::AgentWaiting { text, .. } => {
            if !matches!(agent.state, AgentState::Ended { .. }) {
                agent.state = AgentState::Waiting {
                    since: ts,
                    text: text.clone(),
                };
            }
        }
        EventBody::AgentStopped { recap } => {
            if recap.is_some() {
                agent.last_recap = recap.clone();
            }
            agent.state = AgentState::Stopped { at: ts };
        }
        EventBody::AgentEnded { .. } => agent.state = AgentState::Ended { at: ts },
        EventBody::SessionSummary { text } => agent.summary = Some(text.clone()),
        EventBody::PromptRetract { at, .. } => {
            agent.recent_prompts.retain(|(t, _)| t != at);
            agent.last_prompt = agent.recent_prompts.last().map(|(_, p)| p.clone());
        }
        EventBody::NoteAdd { .. }
        | EventBody::NoteUpdate { .. }
        | EventBody::NoteDone { .. }
        | EventBody::NoteDelete { .. }
        | EventBody::NoteReopen { .. }
        | EventBody::RoadmapSet { .. }
        | EventBody::RoadmapStep { .. }
        | EventBody::CmdSet { .. }
        | EventBody::CmdDelete { .. } => {}
    }
}

/// A tool call means the agent is running again (e.g. after a permission prompt).
fn resume(agent: &mut AgentView, ts: jiff::Timestamp) {
    if matches!(
        agent.state,
        AgentState::Waiting { .. } | AgentState::Idle { .. }
    ) {
        agent.state = AgentState::Working { since: ts };
    }
}

fn upsert_task(tasks: &mut Vec<Task>, id: &str, subject: Option<&str>, status: Option<TaskStatus>) {
    if status == Some(TaskStatus::Deleted) {
        tasks.retain(|t| t.id != id);
        return;
    }
    match tasks.iter_mut().find(|t| t.id == id) {
        Some(t) => {
            if let Some(s) = subject {
                t.subject = s.to_owned();
            }
            if let Some(st) = status {
                t.status = st;
            }
        }
        None => tasks.push(Task {
            id: id.to_owned(),
            // An update for a task created before the hooks were installed.
            subject: subject.map_or_else(|| format!("task #{id}"), str::to_owned),
            status: status.unwrap_or(TaskStatus::Pending),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::event::{NoteType, PlanItem, Source};

    fn at(min: i64) -> jiff::Timestamp {
        jiff::Timestamp::from_second(1_793_430_000 + min * 60).unwrap()
    }

    fn ev(min: i64, session: &str, body: EventBody) -> Event {
        Event::new(at(min), Source::Claude, Some(session.into()), body)
    }

    fn up(id: &str, subject: Option<&str>, status: TaskStatus) -> EventBody {
        EventBody::TaskUpsert {
            task_id: id.into(),
            subject: subject.map(Into::into),
            status: Some(status),
        }
    }

    fn meta(name: &str) -> ProjectMeta {
        ProjectMeta::for_root(&Path::new("/p").join(name))
    }

    #[test]
    fn plan_progress_and_states() {
        use TaskStatus::*;
        let events = vec![
            ev(
                0,
                "a",
                EventBody::AgentStarted {
                    source: "startup".into(),
                    herdr_pane: None,
                },
            ),
            ev(
                1,
                "a",
                EventBody::UserPrompt {
                    text: "import attachments".into(),
                },
            ),
            ev(2, "a", up("1", Some("Adapter"), Pending)),
            ev(2, "a", up("2", Some("Progress UI"), Pending)),
            ev(2, "a", up("3", Some("Tests"), Pending)),
            ev(3, "a", up("1", None, InProgress)),
            ev(
                4,
                "a",
                EventBody::AgentWaiting {
                    text: "Claude needs your permission".into(),
                    reason: None,
                },
            ),
            ev(5, "a", up("1", None, Completed)),
            ev(5, "a", up("2", None, InProgress)),
            ev(
                6,
                "a",
                EventBody::AgentStopped {
                    recap: Some("Adapter done.".into()),
                },
            ),
        ];
        let p = fold_project(meta("DeepVault"), &events);
        let a = &p.agents[0];
        assert_eq!(a.progress(), (1, 3));
        assert_eq!(a.current_task().unwrap().subject, "Progress UI");
        assert_eq!(a.state, AgentState::Stopped { at: at(6) });
        assert_eq!(a.last_recap.as_deref(), Some("Adapter done."));
        assert_eq!(a.last_prompt.as_deref(), Some("import attachments"));
        assert_eq!(p.last_activity, Some(at(6)));
    }

    #[test]
    fn tool_use_after_permission_prompt_resumes_work() {
        let events = vec![
            ev(0, "a", EventBody::UserPrompt { text: "go".into() }),
            ev(
                1,
                "a",
                EventBody::AgentWaiting {
                    text: "perm".into(),
                    reason: None,
                },
            ),
            ev(2, "a", up("1", Some("x"), TaskStatus::InProgress)),
        ];
        let p = fold_project(meta("x"), &events);
        assert_eq!(p.agents[0].state, AgentState::Working { since: at(2) });
    }

    #[test]
    fn deleted_and_orphan_tasks() {
        let events = vec![
            ev(0, "a", up("7", None, TaskStatus::InProgress)),
            ev(1, "a", up("8", Some("keep"), TaskStatus::Pending)),
            ev(2, "a", up("8", None, TaskStatus::Deleted)),
        ];
        let p = fold_project(meta("x"), &events);
        assert_eq!(p.agents[0].tasks.len(), 1);
        assert_eq!(p.agents[0].tasks[0].subject, "task #7");
    }

    #[test]
    fn snapshot_replaces_plan() {
        let snap = |subjects: &[&str]| EventBody::PlanSnapshot {
            tasks: subjects
                .iter()
                .map(|s| PlanItem {
                    subject: (*s).into(),
                    status: TaskStatus::Pending,
                })
                .collect(),
        };
        let events = vec![ev(0, "a", snap(&["a", "b", "c"])), ev(1, "a", snap(&["d"]))];
        let p = fold_project(meta("x"), &events);
        assert_eq!(p.agents[0].tasks.len(), 1);
    }

    #[test]
    fn retract_rolls_back_the_last_prompt() {
        let events = vec![
            ev(
                0,
                "a",
                EventBody::UserPrompt {
                    text: "real question".into(),
                },
            ),
            ev(
                1,
                "a",
                EventBody::UserPrompt {
                    text: "chips No l1O0 tooltip?".into(),
                },
            ),
            ev(
                2,
                "a",
                EventBody::UserPrompt {
                    text: "oops, wrong session".into(),
                },
            ),
            ev(
                3,
                "a",
                EventBody::PromptRetract {
                    at: at(1),
                    to: Some("DeepVault".into()),
                },
            ),
            // A retract in another session does not touch this one.
            ev(
                4,
                "b",
                EventBody::PromptRetract {
                    at: at(2),
                    to: None,
                },
            ),
        ];
        let p = fold_project(meta("x"), &events);
        let a = p.agents.iter().find(|a| a.session == "a").unwrap();
        assert_eq!(a.last_prompt.as_deref(), Some("oops, wrong session"));
        assert_eq!(a.recent_prompts.len(), 2);
        let events = vec![
            ev(
                0,
                "a",
                EventBody::UserPrompt {
                    text: "real".into(),
                },
            ),
            ev(
                1,
                "a",
                EventBody::UserPrompt {
                    text: "misdirected".into(),
                },
            ),
            ev(
                2,
                "a",
                EventBody::PromptRetract {
                    at: at(1),
                    to: None,
                },
            ),
        ];
        let p = fold_project(meta("x"), &events);
        assert_eq!(p.agents[0].last_prompt.as_deref(), Some("real"));
    }

    #[test]
    fn summaries_per_session_and_project() {
        let events = vec![
            ev(
                0,
                "old",
                EventBody::UserPrompt {
                    text: "fix tray".into(),
                },
            ),
            ev(
                1,
                "old",
                EventBody::SessionSummary {
                    text: "Tray icon fixed; release notes left.".into(),
                },
            ),
            ev(2, "old", EventBody::AgentEnded { reason: None }),
            ev(
                3,
                "new",
                EventBody::AgentStarted {
                    source: "startup".into(),
                    herdr_pane: None,
                },
            ),
        ];
        let p = fold_project(meta("x"), &events);
        let s = p.last_summary.as_ref().unwrap();
        assert_eq!(
            (s.session.as_str(), s.text.as_str()),
            ("old", "Tray icon fixed; release notes left.")
        );
        let new = p.agents.iter().find(|a| a.session == "new").unwrap();
        assert_eq!(new.summary, None);
        let old = p.agents.iter().find(|a| a.session == "old").unwrap();
        assert_eq!(old.digest(), "1 prompt");
    }

    #[test]
    fn roadmap_survives_sessions() {
        let set = EventBody::RoadmapSet {
            steps: ["Restart agents", "Release v0.2"]
                .iter()
                .map(|s| PlanItem {
                    subject: (*s).into(),
                    status: TaskStatus::Pending,
                })
                .collect(),
        };
        let events = vec![
            ev(0, "a", set),
            ev(1, "a", EventBody::AgentEnded { reason: None }),
            ev(
                2,
                "b",
                EventBody::RoadmapStep {
                    n: 1,
                    status: TaskStatus::Completed,
                },
            ),
            ev(
                3,
                "b",
                EventBody::RoadmapStep {
                    n: 9,
                    status: TaskStatus::Completed,
                },
            ),
        ];
        let p = fold_project(meta("x"), &events);
        assert_eq!(p.roadmap.len(), 2);
        assert_eq!(p.roadmap[0].status, TaskStatus::Completed);
        assert_eq!(p.roadmap[1].status, TaskStatus::Pending);
    }

    #[test]
    fn notes_lifecycle() {
        let user = |min, body| Event::new(at(min), Source::User, None, body);
        let events = vec![
            user(
                0,
                EventBody::NoteAdd {
                    id: "n1".into(),
                    note_type: NoteType::Idea,
                    text: "a".into(),
                    run: None,
                },
            ),
            user(
                1,
                EventBody::NoteAdd {
                    id: "n2".into(),
                    note_type: NoteType::Question,
                    text: "b".into(),
                    run: None,
                },
            ),
            user(
                2,
                EventBody::NoteUpdate {
                    id: "n1".into(),
                    text: "a2".into(),
                },
            ),
            user(3, EventBody::NoteDone { id: "n2".into() }),
        ];
        let p = fold_project(meta("x"), &events);
        assert!(p.agents.is_empty());
        assert_eq!(p.notes.len(), 1);
        assert_eq!(p.notes[0].text, "a2");
        // Done is not gone: it moves to done_notes and can come back.
        assert_eq!(p.done_notes.len(), 1);
        assert_eq!(p.done_notes[0].id, "n2");
        let mut events = events;
        events.push(user(4, EventBody::NoteReopen { id: "n2".into() }));
        let p = fold_project(meta("x"), &events);
        let ids: Vec<_> = p.notes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["n1", "n2"], "reopened note returns to creation order");
        assert!(p.done_notes.is_empty());
        events.push(user(5, EventBody::NoteDone { id: "n2".into() }));
        events.push(user(6, EventBody::NoteDelete { id: "n2".into() }));
        let p = fold_project(meta("x"), &events);
        assert!(p.done_notes.is_empty() && p.notes.len() == 1);
    }

    #[test]
    fn board_puts_waiting_first() {
        let waiting = vec![ev(
            0,
            "w",
            EventBody::AgentWaiting {
                text: "?".into(),
                reason: None,
            },
        )];
        let working = vec![ev(5, "k", EventBody::UserPrompt { text: "go".into() })];
        let ended = vec![ev(9, "e", EventBody::AgentEnded { reason: None })];
        let board = fold([
            (meta("ended"), ended),
            (meta("working"), working),
            (meta("waiting"), waiting),
        ]);
        let names: Vec<_> = board
            .projects
            .iter()
            .map(|p| p.meta.name.as_str())
            .collect();
        assert_eq!(names, ["waiting", "working", "ended"]);
        assert_eq!(board.waiting_count(), 1);
    }

    #[test]
    fn two_sessions_in_one_project() {
        let events = vec![
            ev(0, "old", EventBody::AgentEnded { reason: None }),
            ev(1, "new", EventBody::UserPrompt { text: "hi".into() }),
        ];
        let p = fold_project(meta("x"), &events);
        assert_eq!(p.agents.len(), 2);
        assert_eq!(p.agents[0].session, "new");
        assert_eq!(p.headline().unwrap().session, "new");
    }
}
