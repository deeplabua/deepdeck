//! The board: what the screens show. Built only by [`crate::fold`].

use jiff::Timestamp;

use crate::event::{NoteType, TaskStatus};
use crate::project::ProjectMeta;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Board {
    /// Sorted: waiting for you → working → idle/stopped → ended, then most recent first.
    pub projects: Vec<ProjectView>,
}

impl Board {
    /// Number of agents blocked on the user, across all projects.
    pub fn waiting_count(&self) -> usize {
        self.projects
            .iter()
            .flat_map(|p| &p.agents)
            .filter(|a| matches!(a.state, AgentState::Waiting { .. }))
            .count()
    }
}

/// A runbook entry: a command worth keeping for this project.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedCommand {
    pub name: String,
    pub command: String,
    pub at: Timestamp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectView {
    pub meta: ProjectMeta,
    /// One per Claude Code session, most recently active first.
    pub agents: Vec<AgentView>,
    /// Open manual notes (tasks, questions, ideas) in creation order.
    pub notes: Vec<Note>,
    /// Notes marked done, most recently done first. They stay visible (and can
    /// be reopened); only `note.delete` removes a note.
    pub done_notes: Vec<Note>,
    /// The latest session summary in the project, from any session.
    pub last_summary: Option<SessionSummary>,
    /// Project roadmap (ids "1".."n"), independent of sessions.
    pub roadmap: Vec<Task>,
    /// Saved project commands (the runbook), in the order first added.
    pub commands: Vec<SavedCommand>,
    pub last_activity: Option<Timestamp>,
}

impl ProjectView {
    /// The state that represents the project as a whole: its most urgent agent.
    pub fn headline(&self) -> Option<&AgentView> {
        self.agents.iter().min_by_key(|a| a.state.rank())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentView {
    pub session: String,
    pub state: AgentState,
    /// The agent's plan, in creation order.
    pub tasks: Vec<Task>,
    /// Last thing the user said to this agent (not retracted).
    pub last_prompt: Option<String>,
    /// Recent prompts with their timestamps, oldest first (for `ddeck retract`).
    pub recent_prompts: Vec<(Timestamp, String)>,
    /// Recap of the last finished turn, kept across later states.
    pub last_recap: Option<String>,
    /// The agent's latest summary of this session.
    pub summary: Option<String>,
    /// herdr pane the agent runs in, for "jump to agent" (v0.3).
    pub herdr_pane: Option<String>,
    pub last_activity: Timestamp,
}

impl AgentView {
    /// `(completed, total)` over the plan.
    pub fn progress(&self) -> (usize, usize) {
        let done = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        (done, self.tasks.len())
    }

    /// The task in progress, else the first pending one.
    pub fn current_task(&self) -> Option<&Task> {
        self.tasks
            .iter()
            .find(|t| t.status == TaskStatus::InProgress)
            .or_else(|| self.tasks.iter().find(|t| t.status == TaskStatus::Pending))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    /// Session started, nothing asked yet.
    Idle {
        since: Timestamp,
    },
    Working {
        since: Timestamp,
    },
    Waiting {
        since: Timestamp,
        text: String,
    },
    Stopped {
        at: Timestamp,
    },
    Ended {
        at: Timestamp,
    },
}

impl AgentState {
    /// Sort key: lower is more urgent.
    pub fn rank(&self) -> u8 {
        match self {
            AgentState::Waiting { .. } => 0,
            AgentState::Working { .. } => 1,
            AgentState::Stopped { .. } | AgentState::Idle { .. } => 2,
            AgentState::Ended { .. } => 3,
        }
    }

    pub fn since(&self) -> Timestamp {
        match self {
            AgentState::Idle { since }
            | AgentState::Working { since }
            | AgentState::Waiting { since, .. } => *since,
            AgentState::Stopped { at } | AgentState::Ended { at } => *at,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionSummary {
    pub session: String,
    pub at: Timestamp,
    pub text: String,
}

impl AgentView {
    /// What the session did, for when the agent wrote no summary: built from
    /// its own events, e.g. "4 prompts · plan 3/4 · last: Tray icon fixed."
    pub fn digest(&self) -> String {
        let mut parts = Vec::new();
        let n = self.recent_prompts.len();
        if n > 0 {
            parts.push(format!("{n} prompt{}", if n == 1 { "" } else { "s" }));
        }
        let (done, total) = self.progress();
        if total > 0 {
            parts.push(format!("plan {done}/{total}"));
        }
        if let Some(r) = &self.last_recap {
            parts.push(format!("last: {r}"));
        }
        parts.join(" · ")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub id: String,
    pub subject: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub id: String,
    pub note_type: NoteType,
    pub text: String,
    pub created: Timestamp,
    /// When it was marked done (only for notes in `done_notes`).
    pub done_at: Option<Timestamp>,
}
