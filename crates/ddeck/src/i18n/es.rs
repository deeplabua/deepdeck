//! Español.

use super::Strings;
use crate::update::Notice;

pub const ES: Strings = Strings {
    app: "DeepDeck",
    tab_projects: "Proyectos",
    tab_timeline: "Historial",
    tab_inbox: "Bandeja",

    state_waiting: "te espera",
    state_working: "trabajando",
    state_done: "listo",
    state_idle: "inactivo",
    state_ended: "terminado",

    plan: "plan",
    you: "tú:",
    recap: "resumen:",
    summary: "balance:",
    notes_questions: "preguntas",
    notes_ideas: "ideas",
    notes_tasks: "tareas",
    last_session: "última sesión:",
    quiet: "inactivos",
    more_done: |n| format!("+ {n} hechas"),
    more: |n| format!("+ {n} más"),
    more_sessions: |n| {
        if n == 1 {
            "+1 sesión".to_owned()
        } else {
            format!("+{n} sesiones")
        }
    },
    ago: |secs| match secs.max(0) {
        0..60 => "ahora".to_owned(),
        s @ 60..3600 => format!("{} min", s / 60),
        s @ 3600..86_400 => format!("{} h", s / 3600),
        s => format!("{} d", s / 86_400),
    },

    kind_task: "tarea",
    kind_question: "pregunta",
    kind_idea: "idea",
    input_hint: "tab proyecto · ↵ guardar · esc cancelar",
    edit_label: "editar",
    copied: |how| format!("copiado · {how}"),
    sorted: |order| match order {
        deepdeck_core::Order::Name => "proyectos por nombre — s: por urgencia".to_owned(),
        deepdeck_core::Order::Attention => "proyectos por urgencia — s: por nombre".to_owned(),
    },
    notice: |n| match n {
        Notice::NewVersion(v, cmd) => format!("↑ Ya salió DeepDeck {v} — {cmd}"),
        Notice::Setup { installed: Some(v) } => format!(
            "las instrucciones de los agentes son v{v}, este ddeck conoce v{} — ejecuta ddeck setup",
            crate::setup::BLOCK_VERSION
        ),
        Notice::Setup { installed: None } => {
            "los agentes no tienen instrucciones de DeepDeck — ejecuta ddeck setup".to_owned()
        }
    },
    notice_copy: "  (u: copiar)",
    herdr_outside: "o salta al agente en herdr — ejecuta ddeck en un panel de herdr",

    keys: &[
        ("o", "agente"),
        ("y", "copiar"),
        ("s", "orden"),
        ("f", "filtro"),
        ("a", "tarea"),
        ("?", "pregunta"),
        ("i", "idea"),
        ("↵", "abrir"),
        ("␣", "hecho"),
        ("d", "borrar"),
        ("e", "editar"),
        ("q", "salir"),
    ],
    keys_timeline: &[
        ("j/k", "desplazar"),
        ("f", "este proyecto / todos"),
        ("q", "salir"),
    ],
    keys_inbox: &[
        ("o", "agente"),
        ("y", "copiar"),
        ("f", "filtro"),
        ("↵", "abrir"),
        ("␣", "resuelto"),
        ("m", "→ tarea"),
        ("e", "editar"),
        ("?", "pregunta"),
        ("i", "idea"),
        ("q", "salir"),
    ],

    tl_plan: |n| {
        if n == 1 {
            "plan: 1 paso".to_owned()
        } else {
            format!("plan: {n} pasos")
        }
    },
    tl_roadmap: |n| {
        if n == 1 {
            "hoja de ruta: 1 paso".to_owned()
        } else {
            format!("hoja de ruta: {n} pasos")
        }
    },
    tl_cmd: |name| format!("comando guardado: {name}"),
    commands: "comandos",
    roadmap: "hoja de ruta",
    tl_you_were_in: |ago, project| format!("hace {ago} estabas en {project}"),
    tl_empty: "Nada todavía hoy.",
    tl_retracted: "mensaje enviado por error eliminado",
    tl_moved_to: |project| format!("mensaje movido a {project}"),
    only: |name| format!("solo {name}"),

    ib_waiting: "TE ESPERAN",
    ib_questions: "PREGUNTAS ABIERTAS",
    ib_ideas: "IDEAS PARA DESPUÉS",
    ib_empty: "Bandeja vacía. Nada te espera.",

    empty_no_hooks: &[
        "DeepDeck aún no está configurado.",
        "",
        "Ejecuta:  ddeck setup --dry-run   — ver los cambios",
        "          ddeck setup             — aplicarlos",
        "",
        "Añade seis hooks a ~/.claude/settings.json y un bloque",
        "breve a ~/.claude/CLAUDE.md para que los agentes publiquen sus planes.",
        "Primero copias de seguridad; otros hooks y textos no cambian.",
    ],
    empty_no_events: &["Inicia una sesión de Claude Code — aparecerá aquí."],

    st_none: "Sin actividad todavía. Inicia una sesión de Claude Code — aparecerá aquí.",
    st_projects: |n| {
        if n == 1 {
            "DeepDeck · 1 proyecto".to_owned()
        } else {
            format!("DeepDeck · {n} proyectos")
        }
    },
    st_waiting: |n| format!("{n} te esperan"),
    st_notes_only: "solo notas",
    st_sessions: |n| format!("({n} sesiones)"),

    dr_binary: "binario",
    dr_data: "datos",
    dr_claude: "claude",
    dr_claude_missing: "no está en el PATH",
    dr_hooks_in: |p| format!("hooks en {p}"),
    dr_hook_missing: "falta — ejecuta `ddeck setup`",
    dr_not_this_binary: "(no es este binario)",
    dr_instructions: "instrucciones",
    dr_instr_old: |p, v, cur| {
        format!("{p} tiene v{v}, la actual es v{cur} — ejecuta `ddeck setup`")
    },
    dr_instr_none: |p| {
        format!("no están en {p} — ejecuta `ddeck setup` (los agentes no publicarán planes)")
    },
    dr_version: "versión",
    dr_version_new: |cur, v, cmd| format!("{cur} instalada, ya salió {v} — {cmd}"),
    dr_version_ok: |cur| format!("{cur} (la más nueva que conocen brew y cargo)"),
    dr_events: "eventos",
    dr_events_total: |p, n| format!("{p} proyectos, {n} eventos"),
    dr_last: |ago| format!("último hace {ago}"),
    dr_hook_errors: |n, p| format!("errores de hooks ({n} en {p}), últimos:"),
    dr_hook_errors_none: "errores de hooks: ninguno",

    su_dry_run: "(simulación — no se escribió nada)",
    su_dry_remove: |p| format!("(simulación — se eliminaría {p})"),
    su_removed: |p| format!("eliminado {p}"),
    su_wrote: |p| format!("escrito {p}"),
    su_wrote_backup: |p, b| format!("escrito {p} (copia: {b})"),
    su_instr_current: |p| format!("Las instrucciones de DeepDeck en {p} están al día"),
    su_no_instr: |p| format!("No hay instrucciones de DeepDeck en {p}"),
    su_hooks_present: |p| format!("Los hooks de DeepDeck ya están instalados en {p}"),
    su_no_hooks: |p| format!("No hay hooks de DeepDeck en {p}"),
    su_restart: "Reinicia las sesiones de Claude Code abiertas (o abre /hooks) para aplicar el cambio.",
    su_done: "Listo. Las nuevas sesiones de Claude Code publicarán sus planes en el tablero; ábrelo con `ddeck`.",
    su_target_warning: |p| {
        format!(
            "aviso: los hooks apuntan a una compilación en target/ ({p}); ejecuta `cargo install --path crates/ddeck` y repite para una ruta estable"
        )
    },

    lang_now: |name, code| format!("idioma: {name} ({code})"),
    lang_set: |name| format!("idioma: {name}"),
    lang_auto: "idioma: de nuevo el del sistema",
};
