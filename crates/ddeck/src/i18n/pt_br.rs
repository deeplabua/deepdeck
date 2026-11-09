//! Português (Brasil).

use super::Strings;
use crate::update::Notice;

pub const PT_BR: Strings = Strings {
    app: "DeepDeck",
    tab_projects: "Projetos",
    tab_timeline: "Histórico",
    tab_inbox: "Entrada",

    state_waiting: "esperando você",
    state_working: "trabalhando",
    state_done: "pronto",
    state_idle: "ocioso",
    state_ended: "encerrado",

    plan: "plano",
    you: "você:",
    recap: "resumo:",
    summary: "balanço:",
    notes_questions: "perguntas",
    notes_ideas: "ideias",
    notes_tasks: "tarefas",
    last_session: "última sessão:",
    quiet: "ociosos",
    more_done: |n| format!("+ {n} concluídas"),
    more: |n| format!("+ {n} a mais"),
    more_sessions: |n| {
        if n == 1 {
            "+1 sessão".to_owned()
        } else {
            format!("+{n} sessões")
        }
    },
    ago: |secs| match secs.max(0) {
        0..60 => "agora".to_owned(),
        s @ 60..3600 => format!("{} min", s / 60),
        s @ 3600..86_400 => format!("{} h", s / 3600),
        s => format!("{} d", s / 86_400),
    },

    kind_task: "tarefa",
    kind_question: "pergunta",
    kind_idea: "ideia",
    input_hint: "tab projeto · ↵ salvar · esc cancelar",
    edit_label: "editar",
    copied: |how| format!("copiado · {how}"),
    sorted: |order| match order {
        deepdeck_core::Order::Name => "projetos por nome — s: por urgência".to_owned(),
        deepdeck_core::Order::Attention => "projetos por urgência — s: por nome".to_owned(),
    },
    notice: |n| match n {
        Notice::NewVersion(v, cmd) => format!("↑ Saiu o DeepDeck {v} — {cmd}"),
        Notice::Setup { installed: Some(v) } => format!(
            "as instruções dos agentes são v{v}, este ddeck conhece v{} — rode ddeck setup",
            crate::setup::BLOCK_VERSION
        ),
        Notice::Setup { installed: None } => {
            "os agentes não têm instruções do DeepDeck — rode ddeck setup".to_owned()
        }
    },
    notice_copy: "  (u: copiar)",
    herdr_outside: "o vai para o agente no herdr — rode o ddeck em um painel do herdr",

    keys: &[
        ("o", "agente"),
        ("y", "copiar"),
        ("s", "ordem"),
        ("f", "filtro"),
        ("a", "tarefa"),
        ("?", "pergunta"),
        ("i", "ideia"),
        ("↵", "abrir"),
        ("␣", "feito"),
        ("d", "excluir"),
        ("e", "editar"),
        ("q", "sair"),
    ],
    keys_timeline: &[
        ("j/k", "rolar"),
        ("f", "este projeto / todos"),
        ("q", "sair"),
    ],
    keys_inbox: &[
        ("o", "agente"),
        ("y", "copiar"),
        ("f", "filtro"),
        ("↵", "abrir"),
        ("␣", "resolvido"),
        ("m", "→ tarefa"),
        ("e", "editar"),
        ("?", "pergunta"),
        ("i", "ideia"),
        ("q", "sair"),
    ],

    tl_plan: |n| {
        if n == 1 {
            "plano: 1 passo".to_owned()
        } else {
            format!("plano: {n} passos")
        }
    },
    tl_roadmap: |n| {
        if n == 1 {
            "roteiro: 1 passo".to_owned()
        } else {
            format!("roteiro: {n} passos")
        }
    },
    tl_cmd: |name| format!("comando salvo: {name}"),
    commands: "comandos",
    roadmap: "roteiro",
    tl_you_were_in: |ago, project| format!("há {ago} você estava em {project}"),
    tl_empty: "Nada ainda hoje.",
    tl_retracted: "mensagem enviada por engano removida",
    tl_moved_to: |project| format!("mensagem movida para {project}"),
    only: |name| format!("só {name}"),

    ib_waiting: "ESPERANDO VOCÊ",
    ib_questions: "PERGUNTAS ABERTAS",
    ib_ideas: "IDEIAS PARA DEPOIS",
    ib_empty: "Entrada vazia. Nada está esperando você.",

    empty_no_hooks: &[
        "O DeepDeck ainda não está configurado.",
        "",
        "Rode:  ddeck setup --dry-run   — ver as mudanças",
        "       ddeck setup             — aplicá-las",
        "",
        "Adiciona seis hooks ao ~/.claude/settings.json e um bloco",
        "curto ao ~/.claude/CLAUDE.md para os agentes publicarem planos.",
        "Backups primeiro; outros hooks e textos não mudam.",
    ],
    empty_no_events: &["Inicie uma sessão do Claude Code — ela aparecerá aqui."],

    st_none: "Nenhuma atividade ainda. Inicie uma sessão do Claude Code — ela aparecerá aqui.",
    st_projects: |n| {
        if n == 1 {
            "DeepDeck · 1 projeto".to_owned()
        } else {
            format!("DeepDeck · {n} projetos")
        }
    },
    st_waiting: |n| format!("{n} esperando você"),
    st_notes_only: "só notas",
    st_sessions: |n| format!("({n} sessões)"),

    dr_binary: "binário",
    dr_data: "dados",
    dr_claude: "claude",
    dr_claude_missing: "não encontrado no PATH",
    dr_hooks_in: |p| format!("hooks em {p}"),
    dr_hook_missing: "ausente — rode `ddeck setup`",
    dr_not_this_binary: "(não é este binário)",
    dr_instructions: "instruções",
    dr_instr_old: |p, v, cur| format!("{p} tem v{v}, a atual é v{cur} — rode `ddeck setup`"),
    dr_instr_none: |p| {
        format!("ausentes em {p} — rode `ddeck setup` (os agentes não publicarão planos)")
    },
    dr_version: "versão",
    dr_version_new: |cur, v, cmd| format!("{cur} instalada, saiu a {v} — {cmd}"),
    dr_version_ok: |cur| format!("{cur} (a mais nova que brew e cargo conhecem)"),
    dr_events: "eventos",
    dr_events_total: |p, n| format!("{p} projetos, {n} eventos"),
    dr_last: |ago| format!("último há {ago}"),
    dr_hook_errors: |n, p| format!("erros de hooks ({n} em {p}), últimos:"),
    dr_hook_errors_none: "erros de hooks: nenhum",

    su_dry_run: "(simulação — nada foi gravado)",
    su_dry_remove: |p| format!("(simulação — {p} seria removido)"),
    su_removed: |p| format!("removido {p}"),
    su_wrote: |p| format!("gravado {p}"),
    su_wrote_backup: |p, b| format!("gravado {p} (backup: {b})"),
    su_instr_current: |p| format!("As instruções do DeepDeck em {p} estão atualizadas"),
    su_no_instr: |p| format!("Nenhuma instrução do DeepDeck em {p}"),
    su_hooks_present: |p| format!("Os hooks do DeepDeck já estão instalados em {p}"),
    su_no_hooks: |p| format!("Nenhum hook do DeepDeck em {p}"),
    su_restart: "Reinicie as sessões abertas do Claude Code (ou abra /hooks) para aplicar a mudança.",
    su_done: "Pronto. Novas sessões do Claude Code publicarão os planos no quadro; abra-o com `ddeck`.",
    su_target_warning: |p| {
        format!(
            "aviso: os hooks apontam para um build em target/ ({p}); rode `cargo install --path crates/ddeck` e repita para um caminho estável"
        )
    },

    lang_now: |name, code| format!("idioma: {name} ({code})"),
    lang_set: |name| format!("idioma: {name}"),
    lang_auto: "idioma: de novo o do sistema",
};
