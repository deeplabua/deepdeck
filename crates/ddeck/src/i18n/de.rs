//! Deutsch.

use super::Strings;
use crate::update::Notice;

pub const DE: Strings = Strings {
    app: "DeepDeck",
    tab_projects: "Projekte",
    tab_timeline: "Verlauf",
    tab_inbox: "Eingang",

    state_waiting: "wartet auf dich",
    state_working: "arbeitet",
    state_done: "fertig",
    state_idle: "inaktiv",
    state_ended: "beendet",

    plan: "Plan",
    you: "du:",
    recap: "kurz:",
    summary: "Fazit:",
    notes_questions: "Fragen",
    notes_ideas: "Ideen",
    notes_tasks: "Aufgaben",
    last_session: "letzte Sitzung:",
    quiet: "inaktiv",
    more_done: |n| format!("+ {n} erledigt"),
    more: |n| format!("+ {n} weitere"),
    more_sessions: |n| {
        if n == 1 {
            "+1 Sitzung".to_owned()
        } else {
            format!("+{n} Sitzungen")
        }
    },
    ago: |secs| match secs.max(0) {
        0..60 => "jetzt".to_owned(),
        s @ 60..3600 => format!("{} Min.", s / 60),
        s @ 3600..86_400 => format!("{} Std.", s / 3600),
        s => format!("{} T.", s / 86_400),
    },

    kind_task: "Aufgabe",
    kind_question: "Frage",
    kind_idea: "Idee",
    input_hint: "Tab Projekt · ↵ speichern · Esc abbrechen",
    edit_label: "bearbeiten",
    copied: |how| format!("kopiert · {how}"),
    sorted: |order| match order {
        deepdeck_core::Order::Name => "Projekte nach Name — s: nach Dringlichkeit".to_owned(),
        deepdeck_core::Order::Attention => "Projekte nach Dringlichkeit — s: nach Name".to_owned(),
    },
    notice: |n| match n {
        Notice::NewVersion(v, cmd) => format!("↑ DeepDeck {v} ist da — {cmd}"),
        Notice::Setup { installed: Some(v) } => format!(
            "Agenten-Anweisungen sind v{v}, dieses ddeck kennt v{} — ddeck setup ausführen",
            crate::setup::BLOCK_VERSION
        ),
        Notice::Setup { installed: None } => {
            "Agenten haben keine DeepDeck-Anweisungen — ddeck setup ausführen".to_owned()
        }
    },
    notice_copy: "  (u: kopieren)",
    herdr_outside: "o springt zum Agenten in herdr — starte ddeck in einem herdr-Bereich",

    keys: &[
        ("o", "Agent"),
        ("y", "kopieren"),
        ("s", "sortieren"),
        ("f", "Filter"),
        ("a", "Aufgabe"),
        ("?", "Frage"),
        ("i", "Idee"),
        ("↵", "öffnen"),
        ("␣", "erledigt"),
        ("d", "löschen"),
        ("e", "bearbeiten"),
        ("q", "beenden"),
    ],
    keys_timeline: &[
        ("j/k", "scrollen"),
        ("f", "dieses Projekt / alle"),
        ("q", "beenden"),
    ],
    keys_inbox: &[
        ("o", "Agent"),
        ("y", "kopieren"),
        ("f", "Filter"),
        ("↵", "öffnen"),
        ("␣", "gelöst"),
        ("m", "→ Aufgabe"),
        ("e", "bearbeiten"),
        ("?", "Frage"),
        ("i", "Idee"),
        ("q", "beenden"),
    ],

    tl_plan: |n| {
        if n == 1 {
            "Plan: 1 Schritt".to_owned()
        } else {
            format!("Plan: {n} Schritte")
        }
    },
    tl_roadmap: |n| {
        if n == 1 {
            "Roadmap: 1 Schritt".to_owned()
        } else {
            format!("Roadmap: {n} Schritte")
        }
    },
    tl_cmd: |name| format!("Befehl gespeichert: {name}"),
    commands: "Befehle",
    roadmap: "Roadmap",
    tl_you_were_in: |ago, project| format!("vor {ago} warst du in {project}"),
    tl_empty: "Heute noch nichts.",
    tl_retracted: "fehlgeleitete Nachricht entfernt",
    tl_moved_to: |project| format!("Nachricht verschoben nach {project}"),
    only: |name| format!("nur {name}"),

    ib_waiting: "WARTET AUF DICH",
    ib_questions: "OFFENE FRAGEN",
    ib_ideas: "IDEEN FÜR SPÄTER",
    ib_empty: "Eingang leer. Nichts wartet auf dich.",

    empty_no_hooks: &[
        "DeepDeck ist noch nicht eingerichtet.",
        "",
        "Starte:  ddeck setup --dry-run   — Änderungen ansehen",
        "         ddeck setup             — Änderungen anwenden",
        "",
        "Fügt sechs Hooks in ~/.claude/settings.json und einen kurzen",
        "Block in ~/.claude/CLAUDE.md hinzu, damit Agenten Pläne posten.",
        "Zuerst Backups; andere Hooks und Texte bleiben unverändert.",
    ],
    empty_no_events: &["Starte eine Claude-Code-Sitzung — sie erscheint hier."],

    st_none: "Noch keine Aktivität. Starte eine Claude-Code-Sitzung — sie erscheint hier.",
    st_projects: |n| {
        if n == 1 {
            "DeepDeck · 1 Projekt".to_owned()
        } else {
            format!("DeepDeck · {n} Projekte")
        }
    },
    st_waiting: |n| format!("{n} warten auf dich"),
    st_notes_only: "nur Notizen",
    st_sessions: |n| format!("({n} Sitzungen)"),

    dr_binary: "Programm",
    dr_data: "Daten",
    dr_claude: "claude",
    dr_claude_missing: "nicht im PATH gefunden",
    dr_hooks_in: |p| format!("Hooks in {p}"),
    dr_hook_missing: "fehlt — `ddeck setup` ausführen",
    dr_not_this_binary: "(nicht dieses Programm)",
    dr_instructions: "Anweisungen",
    dr_instr_old: |p, v, cur| format!("{p} hat v{v}, aktuell ist v{cur} — `ddeck setup` ausführen"),
    dr_instr_none: |p| {
        format!("nicht in {p} — `ddeck setup` ausführen (Agenten posten sonst keine Pläne)")
    },
    dr_version: "Version",
    dr_version_new: |cur, v, cmd| format!("{cur} installiert, {v} ist da — {cmd}"),
    dr_version_ok: |cur| format!("{cur} (die neueste, die brew und cargo kennen)"),
    dr_events: "Ereignisse",
    dr_events_total: |p, n| format!("{p} Projekte, {n} Ereignisse"),
    dr_last: |ago| format!("zuletzt vor {ago}"),
    dr_hook_errors: |n, p| format!("Hook-Fehler ({n} in {p}), zuletzt:"),
    dr_hook_errors_none: "Hook-Fehler: keine",

    su_dry_run: "(Probelauf — nichts geschrieben)",
    su_dry_remove: |p| format!("(Probelauf — {p} würde entfernt)"),
    su_removed: |p| format!("{p} entfernt"),
    su_wrote: |p| format!("{p} geschrieben"),
    su_wrote_backup: |p, b| format!("{p} geschrieben (Backup: {b})"),
    su_instr_current: |p| format!("DeepDeck-Anweisungen in {p} sind aktuell"),
    su_no_instr: |p| format!("Keine DeepDeck-Anweisungen in {p}"),
    su_hooks_present: |p| format!("DeepDeck-Hooks sind bereits in {p} installiert"),
    su_no_hooks: |p| format!("Keine DeepDeck-Hooks in {p}"),
    su_restart: "Starte laufende Claude-Code-Sitzungen neu (oder öffne /hooks), damit die Änderung greift.",
    su_done: "Fertig. Neue Claude-Code-Sitzungen posten ihre Pläne auf das Board; öffne es mit `ddeck`.",
    su_target_warning: |p| {
        format!(
            "Warnung: Hooks zeigen auf einen Build in target/ ({p}); `cargo install --path crates/ddeck` ausführen und erneut starten, für einen festen Pfad"
        )
    },

    lang_now: |name, code| format!("Sprache: {name} ({code})"),
    lang_set: |name| format!("Sprache: {name}"),
    lang_auto: "Sprache: wieder vom System",
};
