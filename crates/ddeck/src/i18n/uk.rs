//! Українська.

use super::{Strings, slavic};
use crate::update::Notice;

pub const UK: Strings = Strings {
    app: "DeepDeck",
    tab_projects: "Проєкти",
    tab_timeline: "Стрічка",
    tab_inbox: "Вхідні",

    state_waiting: "чекає на вас",
    state_working: "працює",
    state_done: "готово",
    state_idle: "простоює",
    state_ended: "завершено",

    plan: "план",
    you: "ви:",
    recap: "коротко:",
    summary: "підсумок:",
    notes_questions: "питання",
    notes_ideas: "ідеї",
    notes_tasks: "завдання",
    last_session: "минула сесія:",
    quiet: "простоюють",
    more_done: |n| format!("+ {n} готово"),
    more: |n| format!("+ ще {n}"),
    more_sessions: |n| format!("+{}", slavic(n, "сесія", "сесії", "сесій")),
    ago: |secs| match secs.max(0) {
        0..60 => "зараз".to_owned(),
        s @ 60..3600 => format!("{} хв", s / 60),
        s @ 3600..86_400 => format!("{} год", s / 3600),
        s => format!("{} д", s / 86_400),
    },

    kind_task: "завдання",
    kind_question: "питання",
    kind_idea: "ідея",
    input_hint: "tab проєкт · ↵ зберегти · esc скасувати",
    edit_label: "редагування",
    copied: |how| format!("скопійовано · {how}"),
    sorted: |order| match order {
        deepdeck_core::Order::Name => "проєкти за назвою — s: за важливістю".to_owned(),
        deepdeck_core::Order::Attention => "проєкти за важливістю — s: за назвою".to_owned(),
    },
    notice: |n| match n {
        Notice::NewVersion(v, cmd) => format!("↑ Вийшов DeepDeck {v} — {cmd}"),
        Notice::Setup { installed: Some(v) } => format!(
            "інструкції агентів v{v}, цей ddeck знає v{} — запустіть ddeck setup",
            crate::setup::BLOCK_VERSION
        ),
        Notice::Setup { installed: None } => {
            "агенти не мають інструкцій DeepDeck — запустіть ddeck setup".to_owned()
        }
    },
    notice_copy: "  (u: копіювати)",
    herdr_outside: "o перемикає на агента в herdr — запустіть ddeck у панелі herdr",

    keys: &[
        ("o", "агент"),
        ("y", "копія"),
        ("s", "порядок"),
        ("f", "фільтр"),
        ("a", "завдання"),
        ("?", "питання"),
        ("i", "ідея"),
        ("↵", "відкрити"),
        ("␣", "готово"),
        ("d", "видалити"),
        ("e", "змінити"),
        ("q", "вихід"),
    ],
    keys_timeline: &[
        ("j/k", "прокрутка"),
        ("f", "цей проєкт / усі"),
        ("q", "вихід"),
    ],
    keys_inbox: &[
        ("o", "агент"),
        ("y", "копія"),
        ("f", "фільтр"),
        ("↵", "відкрити"),
        ("␣", "вирішено"),
        ("m", "→ завдання"),
        ("e", "змінити"),
        ("?", "питання"),
        ("i", "ідея"),
        ("q", "вихід"),
    ],

    tl_plan: |n| format!("план: {}", slavic(n, "крок", "кроки", "кроків")),
    tl_roadmap: |n| format!("дорожня карта: {}", slavic(n, "крок", "кроки", "кроків")),
    tl_cmd: |name| format!("збережено команду: {name}"),
    commands: "команди",
    roadmap: "дорожня карта",
    tl_you_were_in: |ago, project| format!("{ago} тому ви були в {project}"),
    tl_empty: "Сьогодні поки нічого.",
    tl_retracted: "повідомлення не за адресою видалено",
    tl_moved_to: |project| format!("повідомлення перенесено в {project}"),
    only: |name| format!("лише {name}"),

    ib_waiting: "ЧЕКАЮТЬ НА ВАС",
    ib_questions: "ВІДКРИТІ ПИТАННЯ",
    ib_ideas: "ІДЕЇ НА ПОТІМ",
    ib_empty: "Вхідні порожні. На вас нічого не чекає.",

    empty_no_hooks: &[
        "DeepDeck ще не налаштовано.",
        "",
        "Запустіть:  ddeck setup --dry-run   — переглянути зміни",
        "            ddeck setup             — застосувати їх",
        "",
        "Додасть шість хуків у ~/.claude/settings.json і короткий",
        "блок у ~/.claude/CLAUDE.md, щоб агенти публікували плани.",
        "Спершу резервні копії; чужі хуки й текст не змінюються.",
    ],
    empty_no_events: &["Запустіть сесію Claude Code — вона з’явиться тут."],

    st_none: "Поки нічого. Запустіть сесію Claude Code — вона з’явиться тут.",
    st_projects: |n| format!("DeepDeck · {}", slavic(n, "проєкт", "проєкти", "проєктів")),
    st_waiting: |n| format!("чекають на вас: {n}"),
    st_notes_only: "лише нотатки",
    st_sessions: |n| format!("({})", slavic(n, "сесія", "сесії", "сесій")),

    dr_binary: "бінарник",
    dr_data: "дані",
    dr_claude: "claude",
    dr_claude_missing: "не знайдено в PATH",
    dr_hooks_in: |p| format!("хуки в {p}"),
    dr_hook_missing: "немає — запустіть `ddeck setup`",
    dr_not_this_binary: "(не цей бінарник)",
    dr_instructions: "інструкції",
    dr_instr_old: |p, v, cur| format!("{p}: v{v}, актуальна v{cur} — запустіть `ddeck setup`"),
    dr_instr_none: |p| {
        format!("немає в {p} — запустіть `ddeck setup` (агенти не публікуватимуть плани)")
    },
    dr_version: "версія",
    dr_version_new: |cur, v, cmd| format!("встановлено {cur}, вийшла {v} — {cmd}"),
    dr_version_ok: |cur| format!("{cur} (найновіша з тих, що бачили brew і cargo)"),
    dr_events: "події",
    dr_events_total: |p, n| format!("проєктів: {p}, подій: {n}"),
    dr_last: |ago| format!("остання {ago} тому"),
    dr_hook_errors: |n, p| format!("помилки хуків ({n} у {p}), останні:"),
    dr_hook_errors_none: "помилки хуків: немає",

    su_dry_run: "(пробний запуск — нічого не записано)",
    su_dry_remove: |p| format!("(пробний запуск — {p} було б видалено)"),
    su_removed: |p| format!("видалено {p}"),
    su_wrote: |p| format!("записано {p}"),
    su_wrote_backup: |p, b| format!("записано {p} (резервна копія: {b})"),
    su_instr_current: |p| format!("Інструкції DeepDeck у {p} актуальні"),
    su_no_instr: |p| format!("У {p} немає інструкцій DeepDeck"),
    su_hooks_present: |p| format!("Хуки DeepDeck уже встановлено в {p}"),
    su_no_hooks: |p| format!("У {p} немає хуків DeepDeck"),
    su_restart: "Перезапустіть відкриті сесії Claude Code (або відкрийте /hooks), щоб вони підхопили зміни.",
    su_done: "Готово. Нові сесії Claude Code публікуватимуть плани на дошку; відкрийте її командою `ddeck`.",
    su_target_warning: |p| {
        format!(
            "увага: хуки вказують на збірку в target/ ({p}); виконайте `cargo install --path crates/ddeck` і запустіть знову, щоб шлях був сталим"
        )
    },

    lang_now: |name, code| format!("мова: {name} ({code})"),
    lang_set: |name| format!("мова: {name}"),
    lang_auto: "мова: знову як у системі",
};
