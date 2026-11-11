//! Русский.

use super::{Strings, slavic};
use crate::update::Notice;

pub const RU: Strings = Strings {
    app: "DeepDeck",
    tab_projects: "Проекты",
    tab_timeline: "Лента",
    tab_inbox: "Входящие",

    state_waiting: "ждёт вас",
    state_working: "работает",
    state_done: "готово",
    state_idle: "простаивает",
    state_ended: "завершён",

    plan: "план",
    you: "вы:",
    recap: "кратко:",
    summary: "итог:",
    notes_questions: "вопросы",
    notes_ideas: "идеи",
    notes_tasks: "задачи",
    notes_handoffs: "за вами",
    last_session: "прошлая сессия:",
    quiet: "простаивают",
    more_done: |n| format!("+ {n} готово"),
    more: |n| format!("+ ещё {n}"),
    more_sessions: |n| format!("+{}", slavic(n, "сессия", "сессии", "сессий")),
    ago: |secs| match secs.max(0) {
        0..60 => "сейчас".to_owned(),
        s @ 60..3600 => format!("{} мин", s / 60),
        s @ 3600..86_400 => format!("{} ч", s / 3600),
        s => format!("{} д", s / 86_400),
    },

    kind_task: "задача",
    kind_question: "вопрос",
    kind_idea: "идея",
    kind_handoff: "за вами",
    input_hint: "tab проект · ↵ сохранить · esc отмена",
    edit_label: "правка",
    copied: |how| format!("скопировано · {how}"),
    sorted: |order| match order {
        deepdeck_core::Order::Name => "проекты по имени — s: по важности".to_owned(),
        deepdeck_core::Order::Attention => "проекты по важности — s: по имени".to_owned(),
    },
    notice: |n| match n {
        Notice::NewVersion(v, cmd) => format!("↑ Вышел DeepDeck {v} — {cmd}"),
        Notice::Setup { installed: Some(v) } => format!(
            "инструкции агентов v{v}, этот ddeck знает v{} — запустите ddeck setup",
            crate::setup::BLOCK_VERSION
        ),
        Notice::Setup { installed: None } => {
            "у агентов нет инструкций DeepDeck — запустите ddeck setup".to_owned()
        }
    },
    notice_copy: "  (u: копировать)",
    herdr_outside: "o переключает на агента в herdr — запустите ddeck в панели herdr",

    keys: &[
        ("o", "агент"),
        ("y", "копия"),
        ("s", "порядок"),
        ("f", "фильтр"),
        ("a", "задача"),
        ("?", "вопрос"),
        ("i", "идея"),
        ("↵", "открыть"),
        ("␣", "готово"),
        ("d", "удалить"),
        ("e", "правка"),
        ("q", "выход"),
    ],
    keys_timeline: &[
        ("j/k", "прокрутка"),
        ("f", "этот проект / все"),
        ("q", "выход"),
    ],
    keys_inbox: &[
        ("o", "агент"),
        ("y", "копия"),
        ("f", "фильтр"),
        ("↵", "открыть"),
        ("␣", "решено"),
        ("m", "→ задача"),
        ("e", "правка"),
        ("?", "вопрос"),
        ("i", "идея"),
        ("q", "выход"),
    ],

    tl_plan: |n| format!("план: {}", slavic(n, "шаг", "шага", "шагов")),
    tl_roadmap: |n| format!("дорожная карта: {}", slavic(n, "шаг", "шага", "шагов")),
    tl_cmd: |name| format!("сохранена команда: {name}"),
    commands: "команды",
    roadmap: "дорожная карта",
    tl_you_were_in: |ago, project| format!("{ago} назад вы были в {project}"),
    tl_empty: "Сегодня пока ничего.",
    tl_retracted: "сообщение не по адресу удалено",
    tl_moved_to: |project| format!("сообщение перенесено в {project}"),
    only: |name| format!("только {name}"),

    ib_waiting: "ЖДУТ ВАС",
    ib_questions: "ОТКРЫТЫЕ ВОПРОСЫ",
    ib_ideas: "ИДЕИ НА ПОТОМ",
    ib_handoffs: "ЗА ВАМИ",
    ib_empty: "Входящие пусты. Вас ничего не ждёт.",

    empty_no_hooks: &[
        "DeepDeck ещё не настроен.",
        "",
        "Запустите:  ddeck setup --dry-run   — посмотреть изменения",
        "            ddeck setup             — применить их",
        "",
        "Добавит шесть хуков в ~/.claude/settings.json и короткий",
        "блок в ~/.claude/CLAUDE.md, чтобы агенты публиковали планы.",
        "Сначала резервные копии; чужие хуки и текст не меняются.",
    ],
    empty_no_events: &["Запустите сессию Claude Code — она появится здесь."],

    st_none: "Пока ничего. Запустите сессию Claude Code — она появится здесь.",
    st_projects: |n| format!("DeepDeck · {}", slavic(n, "проект", "проекта", "проектов")),
    st_waiting: |n| format!("ждут вас: {n}"),
    st_notes_only: "только заметки",
    st_sessions: |n| format!("({})", slavic(n, "сессия", "сессии", "сессий")),

    dr_binary: "бинарник",
    dr_data: "данные",
    dr_claude: "claude",
    dr_claude_missing: "не найден в PATH",
    dr_hooks_in: |p| format!("хуки в {p}"),
    dr_hook_missing: "нет — запустите `ddeck setup`",
    dr_not_this_binary: "(не этот бинарник)",
    dr_instructions: "инструкции",
    dr_instr_old: |p, v, cur| format!("{p}: v{v}, актуальная v{cur} — запустите `ddeck setup`"),
    dr_instr_none: |p| {
        format!("нет в {p} — запустите `ddeck setup` (агенты не будут публиковать планы)")
    },
    dr_version: "версия",
    dr_version_new: |cur, v, cmd| format!("установлена {cur}, вышла {v} — {cmd}"),
    dr_version_ok: |cur| format!("{cur} (новейшая из тех, что видели brew и cargo)"),
    dr_events: "события",
    dr_events_total: |p, n| format!("проектов: {p}, событий: {n}"),
    dr_last: |ago| format!("последнее {ago} назад"),
    dr_hook_errors: |n, p| format!("ошибки хуков ({n} в {p}), последние:"),
    dr_hook_errors_none: "ошибки хуков: нет",

    su_dry_run: "(пробный запуск — ничего не записано)",
    su_dry_remove: |p| format!("(пробный запуск — {p} был бы удалён)"),
    su_removed: |p| format!("удалён {p}"),
    su_wrote: |p| format!("записан {p}"),
    su_wrote_backup: |p, b| format!("записан {p} (резервная копия: {b})"),
    su_instr_current: |p| format!("Инструкции DeepDeck в {p} актуальны"),
    su_no_instr: |p| format!("В {p} нет инструкций DeepDeck"),
    su_hooks_present: |p| format!("Хуки DeepDeck уже установлены в {p}"),
    su_no_hooks: |p| format!("В {p} нет хуков DeepDeck"),
    su_restart: "Перезапустите открытые сессии Claude Code (или откройте /hooks), чтобы они подхватили изменения.",
    su_done: "Готово. Новые сессии Claude Code будут публиковать планы на доску; откройте её командой `ddeck`.",
    su_target_warning: |p| {
        format!(
            "внимание: хуки указывают на сборку в target/ ({p}); выполните `cargo install --path crates/ddeck` и запустите снова, чтобы путь был постоянным"
        )
    },

    lang_now: |name, code| format!("язык: {name} ({code})"),
    lang_set: |name| format!("язык: {name}"),
    lang_auto: "язык: снова как в системе",
};
