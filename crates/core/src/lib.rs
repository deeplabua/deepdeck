//! DeepDeck core: the event log, Claude Code hook parsing and the board fold.
//!
//! Everything except [`store`] is pure and tested on fixtures; [`store`] is the
//! thin file-system edge.

pub mod archive;
pub mod event;
pub mod fold;
pub mod hooks;
pub mod model;
pub mod project;
pub mod runbook;
pub mod store;

pub use event::{Event, EventBody, NoteType, Source, TaskStatus};
pub use fold::{Order, fold, fold_project};
pub use hooks::{HookEvent, parse_hook};
pub use model::{
    AgentState, AgentView, Board, Note, ProjectView, SavedCommand, SessionSummary, Task,
};
pub use project::{ProjectId, ProjectMeta};
