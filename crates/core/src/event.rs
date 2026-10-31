//! Events — one JSON line each in `events.jsonl`. The log is the source of truth;
//! the board is a fold over it (see [`crate::fold`]).
//!
//! Compatibility rule: the meaning of a field never changes within a `v`. New
//! optional fields are fine; anything else bumps [`EVENT_VERSION`] and the reader
//! keeps understanding the old shape.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

/// Current event schema version, written into every event as `v`.
pub const EVENT_VERSION: u32 = 1;

/// Who produced the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// A Claude Code hook.
    Claude,
    /// The person at the keyboard (TUI or CLI).
    User,
    /// An agent calling `ddeck plan|step|ask|idea…` from its shell.
    Agent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub v: u32,
    pub ts: Timestamp,
    pub src: Source,
    /// Claude Code `session_id`; absent for manual notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(flatten)]
    pub body: EventBody,
}

impl Event {
    pub fn new(ts: Timestamp, src: Source, session: Option<String>, body: EventBody) -> Self {
        Self {
            v: EVENT_VERSION,
            ts,
            src,
            session,
            body,
        }
    }

    /// The `kind` string as written in the log.
    pub fn kind(&self) -> &'static str {
        self.body.kind()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum EventBody {
    /// `SessionStart`. `source` is `startup | resume | clear | compact`.
    #[serde(rename = "agent.started")]
    AgentStarted {
        source: String,
        /// `HERDR_PANE_ID` of the agent's pane, when it runs inside herdr
        /// (added after v1 shipped; optional, so old lines still parse).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        herdr_pane: Option<String>,
    },

    /// `UserPromptSubmit`, text already truncated.
    #[serde(rename = "user.prompt")]
    UserPrompt { text: String },

    /// `TaskCreate` / `TaskUpdate`: a point change to one task of the agent's plan.
    /// Fields that the tool call did not touch are `None`.
    #[serde(rename = "task.upsert")]
    TaskUpsert {
        task_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subject: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<TaskStatus>,
    },

    /// `TodoWrite`: the whole plan at once (older Claude Code versions).
    #[serde(rename = "plan.snapshot")]
    PlanSnapshot { tasks: Vec<PlanItem> },

    /// The user confirmed that the prompt sent at `at` (same session) was meant
    /// for another session: hide it and the turn that answered it.
    #[serde(rename = "prompt.retract")]
    PromptRetract {
        at: Timestamp,
        /// Project it was meant for, if the user said.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<String>,
    },

    /// The agent's own summary of its session so far: what was done, decided,
    /// left open. The latest one is what the next session reads first.
    #[serde(rename = "session.summary")]
    SessionSummary { text: String },

    /// The project's roadmap — what comes after the current request (later
    /// sessions, steps for the user). Project-level: survives sessions.
    #[serde(rename = "roadmap.set")]
    RoadmapSet { steps: Vec<PlanItem> },

    /// Progress on roadmap step `n` (1-based).
    #[serde(rename = "roadmap.step")]
    RoadmapStep { n: usize, status: TaskStatus },

    /// `Notification`: the agent is blocked on the user.
    #[serde(rename = "agent.waiting")]
    AgentWaiting {
        text: String,
        /// Claude Code `notification_type`, e.g. `permission_prompt`, `idle_prompt`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    /// `Stop`: the agent finished its turn.
    #[serde(rename = "agent.stopped")]
    AgentStopped {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recap: Option<String>,
    },

    /// `SessionEnd`.
    #[serde(rename = "agent.ended")]
    AgentEnded {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    #[serde(rename = "note.add")]
    NoteAdd {
        id: String,
        #[serde(rename = "type")]
        note_type: NoteType,
        text: String,
    },

    #[serde(rename = "note.update")]
    NoteUpdate { id: String, text: String },

    #[serde(rename = "note.done")]
    NoteDone { id: String },

    /// Undo of `note.done`: the note is open again.
    #[serde(rename = "note.reopen")]
    NoteReopen { id: String },

    #[serde(rename = "note.delete")]
    NoteDelete { id: String },

    /// A saved project command (runbook entry): set or replace by `name`.
    /// References to secrets only (`dv run --item …`), never their values.
    #[serde(rename = "cmd.set")]
    CmdSet { name: String, command: String },

    #[serde(rename = "cmd.delete")]
    CmdDelete { name: String },
}

impl EventBody {
    pub fn kind(&self) -> &'static str {
        match self {
            EventBody::AgentStarted { .. } => "agent.started",
            EventBody::UserPrompt { .. } => "user.prompt",
            EventBody::TaskUpsert { .. } => "task.upsert",
            EventBody::PlanSnapshot { .. } => "plan.snapshot",
            EventBody::PromptRetract { .. } => "prompt.retract",
            EventBody::SessionSummary { .. } => "session.summary",
            EventBody::RoadmapSet { .. } => "roadmap.set",
            EventBody::RoadmapStep { .. } => "roadmap.step",
            EventBody::AgentWaiting { .. } => "agent.waiting",
            EventBody::AgentStopped { .. } => "agent.stopped",
            EventBody::AgentEnded { .. } => "agent.ended",
            EventBody::NoteAdd { .. } => "note.add",
            EventBody::NoteUpdate { .. } => "note.update",
            EventBody::NoteDone { .. } => "note.done",
            EventBody::NoteDelete { .. } => "note.delete",
            EventBody::NoteReopen { .. } => "note.reopen",
            EventBody::CmdSet { .. } => "cmd.set",
            EventBody::CmdDelete { .. } => "cmd.delete",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    /// `TaskUpdate` with `status: "deleted"` removes the task.
    Deleted,
    /// Anything a future Claude Code version invents.
    #[serde(other)]
    Unknown,
}

impl TaskStatus {
    /// Lenient parse of a status string from a hook payload.
    pub fn parse(s: &str) -> Self {
        match s {
            "pending" => TaskStatus::Pending,
            "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "deleted" => TaskStatus::Deleted,
            _ => TaskStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanItem {
    pub subject: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Task,
    Question,
    Idea,
}

/// Session id used for agent CLI calls made outside a Claude Code session
/// (no `CLAUDE_CODE_SESSION_ID`), e.g. by hand from a shell.
pub const CLI_SESSION: &str = "cli";

/// `n_` + nanoseconds since the epoch in base 36: unique per keystroke and
/// sortable within the same era.
pub fn note_id(now: Timestamp) -> String {
    let mut n = now.as_nanosecond().unsigned_abs();
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while n > 0 {
        out.push(digits[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    format!("n_{}", String::from_utf8(out).expect("ascii"))
}

/// Parse one log line. Unknown kinds and broken lines yield `None` so that an
/// older reader never fails on a newer log.
pub fn parse_line(line: &str) -> Option<Event> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    serde_json::from_str(line).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts() -> Timestamp {
        "2026-10-31T06:22:04Z".parse().unwrap()
    }

    #[test]
    fn serializes_as_documented() {
        let e = Event::new(
            ts(),
            Source::Claude,
            Some("8f3".into()),
            EventBody::AgentWaiting {
                text: "Claude needs your permission".into(),
                reason: Some("permission_prompt".into()),
            },
        );
        let s = serde_json::to_string(&e).unwrap();
        assert_eq!(
            s,
            r#"{"v":1,"ts":"2026-10-31T06:22:04Z","src":"claude","session":"8f3","kind":"agent.waiting","data":{"text":"Claude needs your permission","reason":"permission_prompt"}}"#
        );
        assert_eq!(parse_line(&s), Some(e));
    }

    #[test]
    fn note_round_trip() {
        let e = Event::new(
            ts(),
            Source::User,
            None,
            EventBody::NoteAdd {
                id: "n_1".into(),
                note_type: NoteType::Idea,
                text: "export to Bitwarden JSON".into(),
            },
        );
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains(r#""kind":"note.add","data":{"id":"n_1","type":"idea""#));
        assert_eq!(parse_line(&s), Some(e));
    }

    #[test]
    fn unknown_kind_and_garbage_are_skipped() {
        assert_eq!(
            parse_line(
                r#"{"v":2,"ts":"2026-10-31T06:22:04Z","src":"claude","kind":"agent.teleported","data":{}}"#
            ),
            None
        );
        assert_eq!(parse_line("{not json"), None);
        assert_eq!(parse_line("   "), None);
    }

    #[test]
    fn unknown_task_status_is_tolerated() {
        let line = r#"{"v":1,"ts":"2026-10-31T06:22:04Z","src":"claude","session":"s","kind":"task.upsert","data":{"task_id":"1","status":"blocked"}}"#;
        let e = parse_line(line).unwrap();
        assert_eq!(
            e.body,
            EventBody::TaskUpsert {
                task_id: "1".into(),
                subject: None,
                status: Some(TaskStatus::Unknown)
            }
        );
    }
}
