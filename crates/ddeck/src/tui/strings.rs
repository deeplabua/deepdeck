//! Every user-visible TUI string lives here, so that a translation is one more
//! `const` of this type. Layout code never contains literals.

/// UI strings for one language. Plurals and durations are functions because
/// languages differ in more than a trailing "s".
pub struct Strings {
    pub app: &'static str,
    pub tab_projects: &'static str,
    pub tab_timeline: &'static str,
    pub tab_inbox: &'static str,

    pub state_waiting: &'static str,
    pub state_working: &'static str,
    pub state_done: &'static str,
    pub state_idle: &'static str,
    pub state_ended: &'static str,

    pub plan: &'static str,
    pub you: &'static str,
    pub recap: &'static str,
    pub summary: &'static str,
    /// Note group headers on the Projects screen.
    pub notes_questions: &'static str,
    pub notes_ideas: &'static str,
    pub notes_tasks: &'static str,
    pub last_session: &'static str,
    /// Label of the single line listing projects with nothing to show.
    pub quiet: &'static str,
    /// "+ 7 done" — collapsed completed tasks.
    pub more_done: fn(usize) -> String,
    /// "+1 session" — other sessions in the same project.
    pub more_sessions: fn(usize) -> String,
    /// Time since an event, compact: "now", "5m", "2h", "3d".
    pub ago: fn(i64) -> String,

    pub kind_task: &'static str,
    pub kind_question: &'static str,
    pub kind_idea: &'static str,
    pub input_hint: &'static str,
    pub edit_label: &'static str,

    /// Footer: (key, action) pairs.
    pub keys: &'static [(&'static str, &'static str)],

    /// Footer on the Timeline and Inbox screens.
    pub keys_timeline: &'static [(&'static str, &'static str)],
    pub keys_inbox: &'static [(&'static str, &'static str)],

    /// "plan: 3 steps".
    pub tl_plan: fn(usize) -> String,
    /// "roadmap: 4 steps".
    pub tl_roadmap: fn(usize) -> String,
    /// Section label on the Projects screen.
    pub roadmap: &'static str,
    /// Divider where the conversation switched projects: (ago, project).
    pub tl_you_were_in: fn(&str, &str) -> String,
    pub tl_empty: &'static str,
    /// In place of a retracted prompt and its answer.
    pub tl_retracted: &'static str,
    pub tl_moved_to: fn(&str) -> String,
    pub tl_filter: &'static str,

    pub ib_waiting: &'static str,
    pub ib_questions: &'static str,
    pub ib_ideas: &'static str,
    pub ib_empty: &'static str,

    pub empty_no_hooks: &'static [&'static str],
    pub empty_no_events: &'static [&'static str],
}

pub const EN: Strings = Strings {
    app: "DeepDeck",
    tab_projects: "Projects",
    tab_timeline: "Timeline",
    tab_inbox: "Inbox",

    state_waiting: "waiting for you",
    state_working: "working",
    state_done: "done",
    state_idle: "idle",
    state_ended: "ended",

    plan: "plan",
    you: "you:",
    recap: "recap:",
    summary: "summary:",
    notes_questions: "questions",
    notes_ideas: "ideas",
    notes_tasks: "tasks",
    last_session: "last session:",
    quiet: "idle",
    more_done: |n| format!("+ {n} done"),
    more_sessions: |n| {
        if n == 1 {
            "+1 session".to_owned()
        } else {
            format!("+{n} sessions")
        }
    },
    ago: |secs| match secs.max(0) {
        0..60 => "now".to_owned(),
        s @ 60..3600 => format!("{}m", s / 60),
        s @ 3600..86_400 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86_400),
    },

    kind_task: "task",
    kind_question: "question",
    kind_idea: "idea",
    input_hint: "tab project · ↵ save · esc cancel",
    edit_label: "edit",

    keys: &[
        ("a", "task"),
        ("?", "question"),
        ("i", "idea"),
        ("↵", "open"),
        ("␣", "done"),
        ("d", "delete"),
        ("e", "edit"),
        ("q", "quit"),
    ],

    keys_timeline: &[
        ("j/k", "scroll"),
        ("f", "this project / all"),
        ("q", "quit"),
    ],
    keys_inbox: &[
        ("↵", "open"),
        ("␣", "resolved"),
        ("m", "→ task"),
        ("e", "edit"),
        ("?", "question"),
        ("i", "idea"),
        ("q", "quit"),
    ],

    tl_plan: |n| {
        if n == 1 {
            "plan: 1 step".to_owned()
        } else {
            format!("plan: {n} steps")
        }
    },
    tl_roadmap: |n| {
        if n == 1 {
            "roadmap: 1 step".to_owned()
        } else {
            format!("roadmap: {n} steps")
        }
    },
    roadmap: "roadmap",
    tl_you_were_in: |ago, project| format!("{ago} ago you were in {project}"),
    tl_empty: "Nothing today yet.",
    tl_retracted: "misdirected message removed",
    tl_moved_to: |project| format!("message moved to {project}"),
    tl_filter: "only",

    ib_waiting: "WAITING FOR YOU",
    ib_questions: "OPEN QUESTIONS",
    ib_ideas: "IDEAS FOR LATER",
    ib_empty: "Inbox zero. Nothing is waiting for you.",

    empty_no_hooks: &[
        "DeepDeck is not set up yet.",
        "",
        "Run:  ddeck setup --dry-run   to see the changes",
        "      ddeck setup             to apply them",
        "",
        "It adds six hooks to ~/.claude/settings.json and a short",
        "block to ~/.claude/CLAUDE.md so agents post their plans.",
        "Backups first; other hooks and text stay as they are.",
    ],
    empty_no_events: &["Start a Claude Code session — it will show up here."],
};
