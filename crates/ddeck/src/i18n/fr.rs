//! Français. Singulier pour 0 et 1.

use super::Strings;
use crate::update::Notice;

pub const FR: Strings = Strings {
    app: "DeepDeck",
    tab_projects: "Projets",
    tab_timeline: "Fil",
    tab_inbox: "Boîte",

    state_waiting: "vous attend",
    state_working: "au travail",
    state_done: "terminé",
    state_idle: "inactif",
    state_ended: "fermé",

    plan: "plan",
    you: "vous :",
    recap: "en bref :",
    summary: "bilan :",
    notes_questions: "questions",
    notes_ideas: "idées",
    notes_tasks: "tâches",
    notes_handoffs: "à vous",
    last_session: "dernière session :",
    quiet: "inactifs",
    more_done: |n| {
        if n <= 1 {
            format!("+ {n} terminée")
        } else {
            format!("+ {n} terminées")
        }
    },
    more: |n| format!("+ {n} de plus"),
    more_sessions: |n| {
        if n <= 1 {
            format!("+{n} session")
        } else {
            format!("+{n} sessions")
        }
    },
    ago: |secs| match secs.max(0) {
        0..60 => "maintenant".to_owned(),
        s @ 60..3600 => format!("{} min", s / 60),
        s @ 3600..86_400 => format!("{} h", s / 3600),
        s => format!("{} j", s / 86_400),
    },

    kind_task: "tâche",
    kind_question: "question",
    kind_idea: "idée",
    kind_handoff: "à vous",
    input_hint: "tab projet · ↵ enregistrer · échap annuler",
    edit_label: "modifier",
    copied: |how| format!("copié · {how}"),
    sorted: |order| match order {
        deepdeck_core::Order::Name => "projets par nom — s : par urgence".to_owned(),
        deepdeck_core::Order::Attention => "projets par urgence — s : par nom".to_owned(),
    },
    notice: |n| match n {
        Notice::NewVersion(v, cmd) => format!("↑ DeepDeck {v} est sorti — {cmd}"),
        Notice::Setup { installed: Some(v) } => format!(
            "les instructions des agents sont en v{v}, ce ddeck connaît la v{} — lancez ddeck setup",
            crate::setup::BLOCK_VERSION
        ),
        Notice::Setup { installed: None } => {
            "les agents n’ont pas d’instructions DeepDeck — lancez ddeck setup".to_owned()
        }
    },
    notice_copy: "  (u : copier)",
    herdr_outside: "o bascule vers l’agent dans herdr — lancez ddeck dans un panneau herdr",

    keys: &[
        ("o", "agent"),
        ("y", "copier"),
        ("/", "chercher"),
        ("s", "tri"),
        ("f", "filtre"),
        ("a", "tâche"),
        ("?", "question"),
        ("i", "idée"),
        ("↵", "ouvrir"),
        ("␣", "fait"),
        ("d", "supprimer"),
        ("e", "modifier"),
        ("q", "quitter"),
    ],
    keys_timeline: &[
        ("j/k", "défiler"),
        ("/", "chercher"),
        ("f", "ce projet / tous"),
        ("q", "quitter"),
    ],
    keys_inbox: &[
        ("o", "agent"),
        ("y", "copier"),
        ("/", "chercher"),
        ("f", "filtre"),
        ("↵", "ouvrir"),
        ("␣", "résolu"),
        ("m", "→ tâche"),
        ("e", "modifier"),
        ("?", "question"),
        ("i", "idée"),
        ("q", "quitter"),
    ],
    keys_search: &[
        ("↵", "projet"),
        ("o", "agent"),
        ("y", "resume"),
        ("/", "modifier"),
        ("esc", "fermer"),
    ],
    search_hint: "mots · ↵ résultats · esc fermer",
    search_start: "Cherche ce que vous et les agents avez dit, dans tous les projets.",
    search_none: "Aucun résultat.",
    search_ended: "terminée",
    search_board: "tableau",
    search_found: |m, s| {
        format!(
            "{m} {} dans {s} {}",
            if m == 1 { "résultat" } else { "résultats" },
            if s == 1 { "endroit" } else { "endroits" }
        )
    },

    tl_plan: |n| {
        if n <= 1 {
            format!("plan : {n} étape")
        } else {
            format!("plan : {n} étapes")
        }
    },
    tl_roadmap: |n| {
        if n <= 1 {
            format!("feuille de route : {n} étape")
        } else {
            format!("feuille de route : {n} étapes")
        }
    },
    tl_cmd: |name| format!("commande enregistrée : {name}"),
    commands: "commandes",
    roadmap: "feuille de route",
    tl_you_were_in: |ago, project| format!("il y a {ago}, vous étiez dans {project}"),
    tl_empty: "Rien encore aujourd’hui.",
    tl_retracted: "message mal adressé supprimé",
    tl_moved_to: |project| format!("message déplacé vers {project}"),
    only: |name| format!("seulement {name}"),

    ib_waiting: "VOUS ATTENDENT",
    ib_questions: "QUESTIONS OUVERTES",
    ib_ideas: "IDÉES POUR PLUS TARD",
    ib_handoffs: "À VOUS",
    ib_empty: "Boîte vide. Rien ne vous attend.",

    empty_no_hooks: &[
        "DeepDeck n’est pas encore configuré.",
        "",
        "Lancez :  ddeck setup --dry-run   — voir les changements",
        "          ddeck setup             — les appliquer",
        "",
        "Ajoute six hooks à ~/.claude/settings.json et un court bloc",
        "à ~/.claude/CLAUDE.md pour que les agents publient leurs plans.",
        "Sauvegardes d’abord ; les autres hooks et textes restent intacts.",
    ],
    empty_no_events: &["Lancez une session Claude Code — elle apparaîtra ici."],

    st_none: "Aucune activité pour l’instant. Lancez une session Claude Code — elle apparaîtra ici.",
    st_projects: |n| {
        if n <= 1 {
            format!("DeepDeck · {n} projet")
        } else {
            format!("DeepDeck · {n} projets")
        }
    },
    st_waiting: |n| format!("{n} vous attendent"),
    st_notes_only: "notes seulement",
    st_sessions: |n| format!("({n} sessions)"),

    dr_binary: "binaire",
    dr_data: "données",
    dr_claude: "claude",
    dr_claude_missing: "introuvable dans le PATH",
    dr_hooks_in: |p| format!("hooks dans {p}"),
    dr_hook_missing: "absent — lancez `ddeck setup`",
    dr_not_this_binary: "(pas ce binaire)",
    dr_instructions: "instructions",
    dr_instr_old: |p, v, cur| {
        format!("{p} contient la v{v}, la version actuelle est la v{cur} — lancez `ddeck setup`")
    },
    dr_instr_none: |p| {
        format!(
            "absentes de {p} — lancez `ddeck setup` (sinon les agents ne publient pas de plans)"
        )
    },
    dr_version: "version",
    dr_version_new: |cur, v, cmd| format!("{cur} installée, {v} est sortie — {cmd}"),
    dr_version_ok: |cur| format!("{cur} (la plus récente connue de brew et cargo)"),
    dr_events: "événements",
    dr_events_total: |p, n| format!("{p} projets, {n} événements"),
    dr_last: |ago| format!("dernier il y a {ago}"),
    dr_hook_errors: |n, p| format!("erreurs des hooks ({n} dans {p}), dernières :"),
    dr_hook_errors_none: "erreurs des hooks : aucune",

    su_dry_run: "(simulation — rien n’a été écrit)",
    su_dry_remove: |p| format!("(simulation — {p} serait supprimé)"),
    su_removed: |p| format!("{p} supprimé"),
    su_wrote: |p| format!("{p} écrit"),
    su_wrote_backup: |p, b| format!("{p} écrit (sauvegarde : {b})"),
    su_instr_current: |p| format!("Les instructions DeepDeck de {p} sont à jour"),
    su_no_instr: |p| format!("Pas d’instructions DeepDeck dans {p}"),
    su_hooks_present: |p| format!("Les hooks DeepDeck sont déjà installés dans {p}"),
    su_no_hooks: |p| format!("Pas de hooks DeepDeck dans {p}"),
    su_restart: "Redémarrez les sessions Claude Code ouvertes (ou ouvrez /hooks) pour prendre en compte le changement.",
    su_done: "Terminé. Les nouvelles sessions Claude Code publieront leurs plans sur le tableau ; ouvrez-le avec `ddeck`.",
    su_target_warning: |p| {
        format!(
            "attention : les hooks pointent vers une compilation dans target/ ({p}) ; lancez `cargo install --path crates/ddeck` puis recommencez pour un chemin stable"
        )
    },

    lang_now: |name, code| format!("langue : {name} ({code})"),
    lang_set: |name| format!("langue : {name}"),
    lang_auto: "langue : de nouveau celle du système",
};
