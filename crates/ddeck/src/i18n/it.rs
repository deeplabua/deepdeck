//! Italiano.

use super::Strings;
use crate::update::Notice;

pub const IT: Strings = Strings {
    app: "DeepDeck",
    tab_projects: "Progetti",
    tab_timeline: "Cronologia",
    tab_inbox: "In arrivo",

    state_waiting: "ti aspetta",
    state_working: "al lavoro",
    state_done: "fatto",
    state_idle: "inattivo",
    state_ended: "chiuso",

    plan: "piano",
    you: "tu:",
    recap: "in breve:",
    summary: "bilancio:",
    notes_questions: "domande",
    notes_ideas: "idee",
    notes_tasks: "attività",
    notes_handoffs: "tocca a te",
    last_session: "ultima sessione:",
    quiet: "inattivi",
    more_done: |n| format!("+ {n} fatte"),
    more: |n| format!("+ altri {n}"),
    more_sessions: |n| {
        if n == 1 {
            "+1 sessione".to_owned()
        } else {
            format!("+{n} sessioni")
        }
    },
    ago: |secs| match secs.max(0) {
        0..60 => "ora".to_owned(),
        s @ 60..3600 => format!("{} min", s / 60),
        s @ 3600..86_400 => format!("{} h", s / 3600),
        s => format!("{} g", s / 86_400),
    },

    kind_task: "attività",
    kind_question: "domanda",
    kind_idea: "idea",
    kind_handoff: "tocca a te",
    input_hint: "tab progetto · ↵ salva · esc annulla",
    edit_label: "modifica",
    copied: |how| format!("copiato · {how}"),
    sorted: |order| match order {
        deepdeck_core::Order::Name => "progetti per nome — s: per urgenza".to_owned(),
        deepdeck_core::Order::Attention => "progetti per urgenza — s: per nome".to_owned(),
    },
    notice: |n| match n {
        Notice::NewVersion(v, cmd) => format!("↑ È uscito DeepDeck {v} — {cmd}"),
        Notice::Setup { installed: Some(v) } => format!(
            "le istruzioni degli agenti sono v{v}, questo ddeck conosce v{} — esegui ddeck setup",
            crate::setup::BLOCK_VERSION
        ),
        Notice::Setup { installed: None } => {
            "gli agenti non hanno istruzioni DeepDeck — esegui ddeck setup".to_owned()
        }
    },
    notice_copy: "  (u: copia)",
    herdr_outside: "o passa all’agente in herdr — avvia ddeck in un pannello di herdr",

    keys: &[
        ("o", "agente"),
        ("y", "copia"),
        ("/", "cerca"),
        ("s", "ordine"),
        ("f", "filtro"),
        ("a", "attività"),
        ("?", "domanda"),
        ("i", "idea"),
        ("↵", "apri"),
        ("␣", "fatto"),
        ("d", "elimina"),
        ("e", "modifica"),
        ("q", "esci"),
    ],
    keys_timeline: &[
        ("j/k", "scorri"),
        ("/", "cerca"),
        ("f", "questo progetto / tutti"),
        ("q", "esci"),
    ],
    keys_inbox: &[
        ("o", "agente"),
        ("y", "copia"),
        ("/", "cerca"),
        ("f", "filtro"),
        ("↵", "apri"),
        ("␣", "risolto"),
        ("m", "→ attività"),
        ("e", "modifica"),
        ("?", "domanda"),
        ("i", "idea"),
        ("q", "esci"),
    ],
    keys_search: &[
        ("↵", "progetto"),
        ("o", "agente"),
        ("y", "resume"),
        ("/", "modifica"),
        ("esc", "chiudi"),
    ],
    search_hint: "parole · ↵ risultati · esc chiudi",
    search_start: "Cerca ciò che avete detto tu e gli agenti, in tutti i progetti.",
    search_none: "Nessun risultato.",
    search_ended: "terminata",
    search_board: "bacheca",
    search_found: |m, s| {
        format!(
            "{m} {} in {s} {}",
            if m == 1 { "risultato" } else { "risultati" },
            if s == 1 { "posto" } else { "posti" }
        )
    },

    tl_plan: |n| {
        if n == 1 {
            "piano: 1 passo".to_owned()
        } else {
            format!("piano: {n} passi")
        }
    },
    tl_roadmap: |n| {
        if n == 1 {
            "roadmap: 1 passo".to_owned()
        } else {
            format!("roadmap: {n} passi")
        }
    },
    tl_cmd: |name| format!("comando salvato: {name}"),
    commands: "comandi",
    roadmap: "roadmap",
    tl_you_were_in: |ago, project| format!("{ago} fa eri in {project}"),
    tl_empty: "Ancora niente oggi.",
    tl_retracted: "messaggio inviato per errore rimosso",
    tl_moved_to: |project| format!("messaggio spostato in {project}"),
    only: |name| format!("solo {name}"),

    ib_waiting: "TI ASPETTANO",
    ib_questions: "DOMANDE APERTE",
    ib_ideas: "IDEE PER DOPO",
    ib_handoffs: "TOCCA A TE",
    ib_empty: "Nessun elemento. Niente ti aspetta.",

    empty_no_hooks: &[
        "DeepDeck non è ancora configurato.",
        "",
        "Esegui:  ddeck setup --dry-run   — vedi le modifiche",
        "         ddeck setup             — applicale",
        "",
        "Aggiunge sei hook a ~/.claude/settings.json e un breve blocco",
        "a ~/.claude/CLAUDE.md perché gli agenti pubblichino i piani.",
        "Prima i backup; altri hook e testi restano invariati.",
    ],
    empty_no_events: &["Avvia una sessione di Claude Code — apparirà qui."],

    st_none: "Ancora nessuna attività. Avvia una sessione di Claude Code — apparirà qui.",
    st_projects: |n| {
        if n == 1 {
            "DeepDeck · 1 progetto".to_owned()
        } else {
            format!("DeepDeck · {n} progetti")
        }
    },
    st_waiting: |n| format!("{n} ti aspettano"),
    st_notes_only: "solo note",
    st_sessions: |n| format!("({n} sessioni)"),

    dr_binary: "binario",
    dr_data: "dati",
    dr_claude: "claude",
    dr_claude_missing: "non trovato nel PATH",
    dr_hooks_in: |p| format!("hook in {p}"),
    dr_hook_missing: "manca — esegui `ddeck setup`",
    dr_not_this_binary: "(non questo binario)",
    dr_instructions: "istruzioni",
    dr_instr_old: |p, v, cur| format!("{p} ha v{v}, l’attuale è v{cur} — esegui `ddeck setup`"),
    dr_instr_none: |p| {
        format!("non presenti in {p} — esegui `ddeck setup` (gli agenti non pubblicheranno piani)")
    },
    dr_version: "versione",
    dr_version_new: |cur, v, cmd| format!("{cur} installata, è uscita {v} — {cmd}"),
    dr_version_ok: |cur| format!("{cur} (la più recente nota a brew e cargo)"),
    dr_events: "eventi",
    dr_events_total: |p, n| format!("{p} progetti, {n} eventi"),
    dr_last: |ago| format!("ultimo {ago} fa"),
    dr_hook_errors: |n, p| format!("errori degli hook ({n} in {p}), ultimi:"),
    dr_hook_errors_none: "errori degli hook: nessuno",

    su_dry_run: "(prova — niente scritto)",
    su_dry_remove: |p| format!("(prova — {p} verrebbe rimosso)"),
    su_removed: |p| format!("rimosso {p}"),
    su_wrote: |p| format!("scritto {p}"),
    su_wrote_backup: |p, b| format!("scritto {p} (backup: {b})"),
    su_instr_current: |p| format!("Le istruzioni DeepDeck in {p} sono aggiornate"),
    su_no_instr: |p| format!("Nessuna istruzione DeepDeck in {p}"),
    su_hooks_present: |p| format!("Gli hook DeepDeck sono già installati in {p}"),
    su_no_hooks: |p| format!("Nessun hook DeepDeck in {p}"),
    su_restart: "Riavvia le sessioni di Claude Code aperte (o apri /hooks) per applicare la modifica.",
    su_done: "Fatto. Le nuove sessioni di Claude Code pubblicheranno i piani sulla bacheca; aprila con `ddeck`.",
    su_target_warning: |p| {
        format!(
            "attenzione: gli hook puntano a una build in target/ ({p}); esegui `cargo install --path crates/ddeck` e ripeti per un percorso stabile"
        )
    },

    lang_now: |name, code| format!("lingua: {name} ({code})"),
    lang_set: |name| format!("lingua: {name}"),
    lang_auto: "lingua: di nuovo quella del sistema",
};
