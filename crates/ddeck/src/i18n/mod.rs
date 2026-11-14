//! Interface languages. Every user-visible string of the board and of the
//! screens people read (`status`, `doctor`, `setup`) lives in a [`Strings`]
//! per language; layout code never contains literals.
//!
//! Not translated on purpose: what agents read (`ddeck list`, the one-line
//! answers of agent commands, the CLAUDE.md block) and `--help`.
//!
//! The language: `DEEPDECK_LANG`, else the one saved with `ddeck lang`, else
//! `LC_ALL` / `LC_MESSAGES` / `LANG`, else (macOS) the first system language,
//! else English.

use std::path::Path;
use std::sync::OnceLock;

mod de;
mod en;
mod es;
mod fr;
mod it;
mod ja;
mod ko;
mod pt_br;
mod ru;
mod uk;
mod zh_hans;

pub use en::EN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    De,
    Es,
    Fr,
    It,
    Ja,
    Ko,
    PtBr,
    Ru,
    Uk,
    ZhHans,
}

impl Lang {
    pub const ALL: [Lang; 11] = [
        Lang::En,
        Lang::De,
        Lang::Es,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Ko,
        Lang::PtBr,
        Lang::Ru,
        Lang::Uk,
        Lang::ZhHans,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::De => "de",
            Lang::Es => "es",
            Lang::Fr => "fr",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::Ko => "ko",
            Lang::PtBr => "pt-BR",
            Lang::Ru => "ru",
            Lang::Uk => "uk",
            Lang::ZhHans => "zh-Hans",
        }
    }

    /// The language's own name.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::De => "Deutsch",
            Lang::Es => "Español",
            Lang::Fr => "Français",
            Lang::It => "Italiano",
            Lang::Ja => "日本語",
            Lang::Ko => "한국어",
            Lang::PtBr => "Português (Brasil)",
            Lang::Ru => "Русский",
            Lang::Uk => "Українська",
            Lang::ZhHans => "简体中文",
        }
    }

    /// A locale or language tag: `ru`, `ru_RU.UTF-8`, `pt-BR`, `zh-Hans-CN`,
    /// `zh_CN`, `uk-UA`. Unknown, `C` and `POSIX` give `None`.
    pub fn parse(tag: &str) -> Option<Lang> {
        let tag = tag.split(['.', '@']).next()?.replace('_', "-");
        let mut parts = tag.split('-');
        let lang = parts.next()?.to_ascii_lowercase();
        let rest: Vec<String> = parts.map(str::to_ascii_lowercase).collect();
        Some(match lang.as_str() {
            "en" => Lang::En,
            "de" => Lang::De,
            "es" => Lang::Es,
            "fr" => Lang::Fr,
            "it" => Lang::It,
            "ja" => Lang::Ja,
            "ko" => Lang::Ko,
            // Portuguese: only the Brazilian translation exists.
            "pt" => Lang::PtBr,
            "ru" => Lang::Ru,
            "uk" => Lang::Uk,
            // Simplified Chinese; Traditional (Hant, TW, HK, MO) is not translated.
            "zh" if !rest
                .iter()
                .any(|p| ["hant", "tw", "hk", "mo"].contains(&p.as_str())) =>
            {
                Lang::ZhHans
            }
            _ => return None,
        })
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Lang::En => &EN,
            Lang::De => &de::DE,
            Lang::Es => &es::ES,
            Lang::Fr => &fr::FR,
            Lang::It => &it::IT,
            Lang::Ja => &ja::JA,
            Lang::Ko => &ko::KO,
            Lang::PtBr => &pt_br::PT_BR,
            Lang::Ru => &ru::RU,
            Lang::Uk => &uk::UK,
            Lang::ZhHans => &zh_hans::ZH_HANS,
        }
    }
}

/// Where the language came from, for `ddeck lang`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Env,
    Saved,
    Locale,
    System,
    Default,
}

/// The language of this process, decided once.
pub fn lang() -> Lang {
    static LANG: OnceLock<Lang> = OnceLock::new();
    *LANG.get_or_init(|| detect().0)
}

/// The strings of this process's language.
pub fn t() -> &'static Strings {
    lang().strings()
}

pub fn detect() -> (Lang, Source) {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    if let Some(l) = env("DEEPDECK_LANG").and_then(|v| Lang::parse(&v)) {
        return (l, Source::Env);
    }
    if let Some(l) = deepdeck_core::store::Paths::from_env().and_then(|p| saved(&p.tui_prefs())) {
        return (l, Source::Saved);
    }
    // The first locale variable that is set decides, as in POSIX.
    if let Some(l) = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|k| env(k))
        .and_then(|v| Lang::parse(&v))
    {
        return (l, Source::Locale);
    }
    if let Some(l) = system_language() {
        return (l, Source::System);
    }
    (Lang::En, Source::Default)
}

/// The language saved with `ddeck lang` (`"lang"` in `tui.json`).
pub fn saved(prefs: &Path) -> Option<Lang> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(prefs).ok()?).ok()?;
    Lang::parse(v["lang"].as_str()?)
}

/// Set one key of `tui.json`, keeping the others (`None` removes it).
pub fn set_pref(prefs: &Path, key: &str, value: Option<serde_json::Value>) -> std::io::Result<()> {
    let mut v: serde_json::Value = std::fs::read(prefs)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    let map = v.as_object_mut().expect("an object");
    match value {
        Some(x) => {
            map.insert(key.to_owned(), x);
        }
        None => {
            map.remove(key);
        }
    }
    if let Some(dir) = prefs.parent() {
        deepdeck_core::store::ensure_private_dir(dir)?;
    }
    std::fs::write(
        prefs,
        serde_json::to_vec(&v).map_err(std::io::Error::other)?,
    )
}

/// macOS: the first of the user's preferred languages that we have.
fn system_language() -> Option<Lang> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let out = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
        .ok()?;
    apple_languages(&String::from_utf8_lossy(&out.stdout))
}

/// `("en-GB", "ru-UA")` → the first language we have.
pub fn apple_languages(text: &str) -> Option<Lang> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
        .filter(|s| !s.is_empty())
        .find_map(Lang::parse)
}

/// Russian and Ukrainian plural forms: 1 шаг, 2 шага, 5 шагов (21 шаг, 11 шагов).
pub fn slavic(n: usize, one: &str, few: &str, many: &str) -> String {
    let form = match (n % 10, n % 100) {
        (1, r) if r != 11 => one,
        (2..=4, r) if !(12..=14).contains(&r) => few,
        _ => many,
    };
    format!("{n} {form}")
}

/// Every user-visible string of the human screens, for one language. Plurals
/// and durations are functions because languages differ in more than a trailing "s".
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
    /// Project screen group of handoffs (lowercase, like the other groups).
    pub notes_handoffs: &'static str,
    pub last_session: &'static str,
    /// Label of the single line listing projects with nothing to show.
    pub quiet: &'static str,
    /// "+ 7 done" — collapsed completed tasks.
    pub more_done: fn(usize) -> String,
    /// "+ 2 more" — folded saved commands.
    pub more: fn(usize) -> String,
    /// "+1 session" — other sessions in the same project.
    pub more_sessions: fn(usize) -> String,
    /// Time since an event, compact: "now", "5m", "2h", "3d".
    pub ago: fn(i64) -> String,

    pub kind_task: &'static str,
    pub kind_question: &'static str,
    pub kind_idea: &'static str,
    pub kind_handoff: &'static str,
    pub input_hint: &'static str,
    pub edit_label: &'static str,
    /// After `y`: "copied · pbcopy".
    pub copied: fn(&str) -> String,
    /// After `s`: the new project order.
    pub sorted: fn(deepdeck_core::Order) -> String,
    /// Above the footer: a newer release, old agent instructions.
    pub notice: fn(&crate::update::Notice) -> String,
    /// After a notice on the board.
    pub notice_copy: &'static str,
    /// `o` pressed outside herdr.
    pub herdr_outside: &'static str,

    /// Footer: (key, action) pairs. Keys are the same in every language.
    pub keys: &'static [(&'static str, &'static str)],
    pub keys_timeline: &'static [(&'static str, &'static str)],
    pub keys_inbox: &'static [(&'static str, &'static str)],
    /// Footer of the search screen (`/`) once the cursor is in the results.
    pub keys_search: &'static [(&'static str, &'static str)],
    /// Footer while typing the search words.
    pub search_hint: &'static str,
    /// Search screen before anything is typed.
    pub search_start: &'static str,
    pub search_none: &'static str,
    /// A session that has ended (search results).
    pub search_ended: &'static str,
    /// Hits not tied to a session: notes and commands posted from a terminal.
    pub search_board: &'static str,
    /// "4 matches in 2 places".
    pub search_found: fn(usize, usize) -> String,

    /// "plan: 3 steps".
    pub tl_plan: fn(usize) -> String,
    /// "roadmap: 4 steps".
    pub tl_roadmap: fn(usize) -> String,
    /// "saved command: release".
    pub tl_cmd: fn(&str) -> String,
    /// Section label for saved commands on the Projects screen.
    pub commands: &'static str,
    /// Section label on the Projects screen.
    pub roadmap: &'static str,
    /// Divider where the conversation switched projects: (ago, project).
    pub tl_you_were_in: fn(&str, &str) -> String,
    pub tl_empty: &'static str,
    /// In place of a retracted prompt and its answer.
    pub tl_retracted: &'static str,
    pub tl_moved_to: fn(&str) -> String,
    /// Footer label while `f` shows one project: "only DeepDeck".
    pub only: fn(&str) -> String,

    pub ib_waiting: &'static str,
    pub ib_questions: &'static str,
    pub ib_ideas: &'static str,
    /// Inbox section: what only the user can do or check.
    pub ib_handoffs: &'static str,
    pub ib_empty: &'static str,

    pub empty_no_hooks: &'static [&'static str],
    pub empty_no_events: &'static [&'static str],

    // ── ddeck status ──
    pub st_none: &'static str,
    /// "DeepDeck · 3 projects".
    pub st_projects: fn(usize) -> String,
    /// "2 waiting for you".
    pub st_waiting: fn(usize) -> String,
    pub st_notes_only: &'static str,
    /// "(3 sessions)".
    pub st_sessions: fn(usize) -> String,

    // ── ddeck doctor ── (labels are padded by the caller)
    pub dr_binary: &'static str,
    pub dr_data: &'static str,
    pub dr_claude: &'static str,
    pub dr_claude_missing: &'static str,
    /// "hooks in <path>".
    pub dr_hooks_in: fn(&str) -> String,
    pub dr_hook_missing: &'static str,
    pub dr_not_this_binary: &'static str,
    pub dr_instructions: &'static str,
    /// (path, installed, current).
    pub dr_instr_old: fn(&str, u32, u32) -> String,
    pub dr_instr_none: fn(&str) -> String,
    pub dr_version: &'static str,
    /// (installed, newer, command).
    pub dr_version_new: fn(&str, &str, &str) -> String,
    pub dr_version_ok: fn(&str) -> String,
    pub dr_events: &'static str,
    /// (projects, events).
    pub dr_events_total: fn(usize, usize) -> String,
    /// "last 5m ago".
    pub dr_last: fn(&str) -> String,
    /// (count, path).
    pub dr_hook_errors: fn(usize, &str) -> String,
    pub dr_hook_errors_none: &'static str,

    // ── ddeck setup / uninstall ──
    pub su_dry_run: &'static str,
    pub su_dry_remove: fn(&str) -> String,
    pub su_removed: fn(&str) -> String,
    pub su_wrote: fn(&str) -> String,
    /// (path, backup).
    pub su_wrote_backup: fn(&str, &str) -> String,
    pub su_instr_current: fn(&str) -> String,
    pub su_no_instr: fn(&str) -> String,
    pub su_hooks_present: fn(&str) -> String,
    pub su_no_hooks: fn(&str) -> String,
    pub su_restart: &'static str,
    pub su_done: &'static str,
    pub su_target_warning: fn(&str) -> String,

    // ── ddeck lang ──
    /// (language name, code).
    pub lang_now: fn(&str, &str) -> String,
    pub lang_set: fn(&str) -> String,
    pub lang_auto: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tags_and_locales() {
        assert_eq!(Lang::parse("ru_RU.UTF-8"), Some(Lang::Ru));
        assert_eq!(Lang::parse("uk-UA"), Some(Lang::Uk));
        assert_eq!(Lang::parse("pt_PT"), Some(Lang::PtBr));
        assert_eq!(Lang::parse("zh-Hans-CN"), Some(Lang::ZhHans));
        assert_eq!(Lang::parse("zh_CN.UTF-8"), Some(Lang::ZhHans));
        assert_eq!(Lang::parse("zh-Hant-TW"), None);
        assert_eq!(Lang::parse("de_DE@euro"), Some(Lang::De));
        assert_eq!(Lang::parse("C"), None);
        assert_eq!(Lang::parse("POSIX"), None);
        for l in Lang::ALL {
            assert_eq!(Lang::parse(l.code()), Some(l), "{}", l.code());
        }
    }

    #[test]
    fn reads_apple_languages() {
        let out = "(\n    \"en-GB\",\n    \"ru-UA\"\n)\n";
        assert_eq!(apple_languages(out), Some(Lang::En));
        assert_eq!(apple_languages("(\"he-IL\", \"uk-UA\")"), Some(Lang::Uk));
        assert_eq!(apple_languages("()"), None);
    }

    #[test]
    fn slavic_plurals() {
        let f = |n| slavic(n, "шаг", "шага", "шагов");
        assert_eq!(f(1), "1 шаг");
        assert_eq!(f(3), "3 шага");
        assert_eq!(f(5), "5 шагов");
        assert_eq!(f(11), "11 шагов");
        assert_eq!(f(12), "12 шагов");
        assert_eq!(f(21), "21 шаг");
        assert_eq!(f(22), "22 шага");
        assert_eq!(f(0), "0 шагов");
    }

    /// Footer keys are the same letters in every language (code matches on them).
    #[test]
    fn every_language_has_the_same_keys() {
        for l in Lang::ALL {
            let s = l.strings();
            fn keys(k: &[(&'static str, &'static str)]) -> Vec<&'static str> {
                k.iter().map(|(k, _)| *k).collect()
            }
            assert_eq!(keys(s.keys), keys(EN.keys), "{}", l.code());
            assert_eq!(
                keys(s.keys_timeline),
                keys(EN.keys_timeline),
                "{}",
                l.code()
            );
            assert_eq!(keys(s.keys_inbox), keys(EN.keys_inbox), "{}", l.code());
            assert_eq!(
                s.empty_no_hooks.len(),
                EN.empty_no_hooks.len(),
                "{}",
                l.code()
            );
            // Functions run and say something in every language.
            for n in [0, 1, 2, 5, 11, 21] {
                assert!((s.tl_plan)(n).contains(&n.to_string()));
                assert!((s.more_done)(n).contains(&n.to_string()));
                assert!((s.st_projects)(n).contains(&n.to_string()));
            }
            for secs in [0, 59, 60, 3600, 86_400 * 3] {
                assert!(!(s.ago)(secs).is_empty());
            }
        }
    }
}
