//! English — the source language.

use super::Strings;

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
    notes_handoffs: "your part",
    last_session: "last session:",
    quiet: "idle",
    more_done: |n| format!("+ {n} done"),
    more: |n| format!("+ {n} more"),
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
    kind_handoff: "your part",
    input_hint: "tab project · ↵ save · esc cancel",
    edit_label: "edit",
    copied: |how| format!("copied · {how}"),
    sorted: |order| match order {
        deepdeck_core::Order::Name => "projects by name — s: by attention".to_owned(),
        deepdeck_core::Order::Attention => "projects by attention — s: by name".to_owned(),
    },
    notice: |n| match n {
        crate::update::Notice::NewVersion(v, cmd) => {
            format!("↑ DeepDeck {v} is out — {cmd}")
        }
        crate::update::Notice::Setup { installed: Some(v) } => format!(
            "agent instructions are v{v}, this ddeck has v{} — run ddeck setup",
            crate::setup::BLOCK_VERSION
        ),
        crate::update::Notice::Setup { installed: None } => {
            "agents have no DeepDeck instructions — run ddeck setup".to_owned()
        }
    },
    notice_copy: "  (u: copy)",
    herdr_outside: "o jumps to the agent in herdr — run ddeck in a herdr pane",

    keys: &[
        ("o", "agent"),
        ("y", "copy"),
        ("/", "find"),
        ("s", "sort"),
        ("f", "only"),
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
        ("/", "find"),
        ("f", "this project / all"),
        ("q", "quit"),
    ],
    keys_inbox: &[
        ("o", "agent"),
        ("y", "copy"),
        ("/", "find"),
        ("f", "only"),
        ("↵", "open"),
        ("␣", "resolved"),
        ("m", "→ task"),
        ("e", "edit"),
        ("?", "question"),
        ("i", "idea"),
        ("q", "quit"),
    ],
    keys_search: &[
        ("↵", "project"),
        ("o", "agent"),
        ("y", "resume"),
        ("/", "edit"),
        ("esc", "close"),
    ],
    search_hint: "words · ↵ results · esc close",
    search_start: "Search what you and the agents said, in every project.",
    search_none: "Nothing found.",
    search_ended: "ended",
    search_board: "board",
    search_found: |m, s| {
        format!(
            "{m} {} in {s} {}",
            if m == 1 { "match" } else { "matches" },
            if s == 1 { "place" } else { "places" }
        )
    },

    tl_plan: |n| {
        if n == 1 {
            "plan: 1 step".to_owned()
        } else {
            format!("plan: {n} steps")
        }
    },
    tl_cmd: |name| format!("saved command: {name}"),
    commands: "commands",
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
    only: |name| format!("only {name}"),

    ib_waiting: "WAITING FOR YOU",
    ib_questions: "OPEN QUESTIONS",
    ib_ideas: "IDEAS FOR LATER",
    ib_handoffs: "YOUR PART",
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

    st_none: "No activity yet. Start a Claude Code session — it will show up here.",
    st_projects: |n| {
        if n == 1 {
            "DeepDeck · 1 project".to_owned()
        } else {
            format!("DeepDeck · {n} projects")
        }
    },
    st_waiting: |n| format!("{n} waiting for you"),
    st_notes_only: "notes only",
    st_sessions: |n| format!("({n} sessions)"),

    dr_binary: "binary",
    dr_data: "data",
    dr_claude: "claude",
    dr_claude_missing: "not found in PATH",
    dr_hooks_in: |p| format!("hooks in {p}"),
    dr_hook_missing: "missing — run `ddeck setup`",
    dr_not_this_binary: "(not this binary)",
    dr_instructions: "instructions",
    dr_instr_old: |p, v, cur| format!("{p} has v{v}, current is v{cur} — run `ddeck setup`"),
    dr_instr_none: |p| format!("not in {p} — run `ddeck setup` (agents won't post plans)"),
    dr_version: "version",
    dr_version_new: |cur, v, cmd| format!("{cur} installed, {v} is out — {cmd}"),
    dr_version_ok: |cur| format!("{cur} (newest that brew and cargo have seen)"),
    dr_events: "events",
    dr_events_total: |p, n| format!("{p} projects, {n} events"),
    dr_last: |ago| format!("last {ago} ago"),
    dr_hook_errors: |n, p| format!("hook errors ({n} in {p}), last:"),
    dr_hook_errors_none: "hook errors: none",

    su_dry_run: "(dry run — nothing written)",
    su_dry_remove: |p| format!("(dry run — {p} would be removed)"),
    su_removed: |p| format!("removed {p}"),
    su_wrote: |p| format!("wrote {p}"),
    su_wrote_backup: |p, b| format!("wrote {p} (backup: {b})"),
    su_instr_current: |p| format!("DeepDeck instructions are up to date in {p}"),
    su_no_instr: |p| format!("No DeepDeck instructions in {p}"),
    su_hooks_present: |p| format!("DeepDeck hooks are already installed in {p}"),
    su_no_hooks: |p| format!("No DeepDeck hooks in {p}"),
    su_restart: "Restart running Claude Code sessions (or open /hooks) to pick the change up.",
    su_done: "Done. New Claude Code sessions will post plans to the board; open it with `ddeck`.",
    su_target_warning: |p| {
        format!(
            "warning: hooks point to a build in target/ ({p}); `cargo install --path crates/ddeck` and re-run for a stable path"
        )
    },

    lang_now: |name, code| format!("language: {name} ({code})"),
    lang_set: |name| format!("language set: {name}"),
    lang_auto: "language: from the system again",
};
