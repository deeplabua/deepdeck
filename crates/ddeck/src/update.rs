//! What the board should tell about this install: a newer DeepDeck that brew or
//! cargo already knows about, and agent instructions older than this binary.
//!
//! No network: only files the package managers keep on disk are read — the
//! Homebrew tap's formula (refreshed by any `brew update`, `install` or
//! `upgrade`) and cargo's crates.io index cache (refreshed when cargo resolves
//! the crate). A newer release shows up once one of them has seen it.

use std::path::{Path, PathBuf};

/// How this binary was installed, which decides the upgrade command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Install {
    Brew,
    /// `cargo install deepdeck` from crates.io.
    Cargo,
    /// cargo-dist's shell or PowerShell installer.
    Installer,
    /// Built from a source checkout (`cargo install --path <dir>`).
    Source(String),
}

impl Install {
    pub fn upgrade_command(&self) -> String {
        match self {
            Install::Brew => "brew upgrade deepdeck".to_owned(),
            Install::Cargo => "cargo install deepdeck --locked".to_owned(),
            Install::Installer if cfg!(windows) => "powershell -c \"irm https://github.com/deeplabua/deepdeck/releases/latest/download/deepdeck-installer.ps1 | iex\"".to_owned(),
            Install::Installer => "curl -LsSf https://github.com/deeplabua/deepdeck/releases/latest/download/deepdeck-installer.sh | sh".to_owned(),
            // Pull first: a released version newer than this build means the checkout is behind.
            Install::Source(dir) => format!("cargo install --path {dir} --locked"),
        }
    }
}

/// A notice for the board's footer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// A newer release is known: (version, upgrade command).
    NewVersion(String, String),
    /// The agent instructions are missing or older: `ddeck setup`.
    Setup { installed: Option<u32> },
}

/// Everything worth telling right now. Reads a few small files; call it at
/// start and now and then, never from a hook.
pub fn notices(hooks_installed: bool) -> Vec<Notice> {
    let mut out = Vec::new();
    let install = how_installed();
    if let Some(v) = known_versions()
        .into_iter()
        .filter(|v| newer(v, env!("CARGO_PKG_VERSION")))
        .max_by(|a, b| cmp_versions(a, b))
    {
        out.push(Notice::NewVersion(v, install.upgrade_command()));
    }
    // Without hooks the board already says to run `ddeck setup`.
    if hooks_installed {
        let installed = crate::setup::global_claude_md()
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| crate::setup::installed_version(&t));
        if installed.is_none_or(|v| v < crate::setup::BLOCK_VERSION) {
            out.push(Notice::Setup { installed });
        }
    }
    out
}

fn how_installed() -> Install {
    let exe = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .unwrap_or_default();
    let cargo_home = cargo_home();
    let crates2 = cargo_home
        .as_ref()
        .and_then(|h| std::fs::read_to_string(h.join(".crates2.json")).ok());
    let receipt = receipt_path().is_some_and(|p| p.exists());
    classify(&exe, crates2.as_deref(), receipt)
}

/// Pure part of [`how_installed`].
pub fn classify(exe: &Path, crates2: Option<&str>, installer_receipt: bool) -> Install {
    let exe = exe.to_string_lossy().replace('\\', "/");
    if exe.contains("/Cellar/deepdeck/") {
        return Install::Brew;
    }
    // `.crates2.json` keys: "deepdeck 0.4.0 (registry+https://…)" or "(path+file://…)".
    let key = crates2.and_then(|t| {
        let at = t.find("\"deepdeck ")?;
        t[at..].split('"').nth(1).map(str::to_owned)
    });
    match key {
        Some(k) if k.contains("(path+") => Install::Source(source_dir(&k)),
        Some(k) if k.contains("(registry+") || k.contains("(sparse+") => Install::Cargo,
        _ if installer_receipt => Install::Installer,
        // Unknown: the installer works everywhere.
        _ => Install::Installer,
    }
}

/// The directory in "deepdeck 0.4.0 (path+file:///Users/me/DeepDeck/crates/ddeck)",
/// with the home directory as `~` to keep the command short.
fn source_dir(key: &str) -> String {
    let dir = key
        .split_once("(path+file://")
        .map_or("", |(_, rest)| rest.trim_end_matches(')'));
    match std::env::home_dir().and_then(|h| h.to_str().map(str::to_owned)) {
        Some(h) if !h.is_empty() && dir.starts_with(&format!("{h}/")) => {
            format!("~{}", &dir[h.len()..])
        }
        _ => dir.to_owned(),
    }
}

/// For `ddeck list` and `ddeck status` run by a person: the notices after the
/// output. Agents run these commands with a pipe, not a terminal — they never
/// see them and so never act on them.
pub fn for_terminal() -> String {
    use std::io::IsTerminal as _;
    if !std::io::stdout().is_terminal() {
        return String::new();
    }
    let style = crate::status::Style::for_stdout();
    notices(true)
        .iter()
        .map(|n| {
            format!(
                "\n{}\n",
                style.paint("33", &(crate::tui::strings::EN.notice)(n))
            )
        })
        .collect()
}

/// The text to copy for a notice: the command it names.
pub fn command(n: &Notice) -> String {
    match n {
        Notice::NewVersion(_, cmd) => cmd.clone(),
        Notice::Setup { .. } => "ddeck setup".to_owned(),
    }
}

fn cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|h| h.join(".cargo")))
}

/// Where cargo-dist's installer leaves its receipt.
fn receipt_path() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::home_dir().map(|h| h.join(".config")))?
    };
    Some(base.join("deepdeck").join("deepdeck-receipt.json"))
}

/// Versions of DeepDeck that brew and cargo have seen.
fn known_versions() -> Vec<String> {
    let mut out = Vec::new();
    for repo in brew_repositories() {
        let f = repo.join("Library/Taps/deeplabua/homebrew-tap/Formula/deepdeck.rb");
        if let Some(v) = std::fs::read_to_string(f)
            .ok()
            .and_then(|t| formula_version(&t))
        {
            out.push(v);
        }
    }
    if let Some(index) = cargo_home().map(|h| h.join("registry/index"))
        && let Ok(dirs) = std::fs::read_dir(index)
    {
        for d in dirs.flatten() {
            if let Ok(bytes) = std::fs::read(d.path().join(".cache/de/ep/deepdeck")) {
                out.extend(index_cache_versions(&bytes));
            }
        }
    }
    out
}

fn brew_repositories() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::env::var_os("HOMEBREW_REPOSITORY")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for p in [
        "/opt/homebrew",
        "/usr/local/Homebrew",
        "/home/linuxbrew/.linuxbrew/Homebrew",
    ] {
        let p = PathBuf::from(p);
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// `version "0.4.0"` in a Homebrew formula.
pub fn formula_version(text: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let rest = l.trim().strip_prefix("version ")?;
        Some(rest.trim().trim_matches('"').to_owned()).filter(|v| !v.is_empty())
    })
}

/// Versions in cargo's sparse index cache: a header, then for each version its
/// number and its JSON line, NUL-separated. Yanked versions are left out.
pub fn index_cache_versions(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|b| *b == 0)
        .filter_map(|chunk| serde_json::from_slice::<serde_json::Value>(chunk).ok())
        .filter(|v| v["yanked"] != true)
        .filter_map(|v| v["vers"].as_str().map(str::to_owned))
        .collect()
}

/// Release numbers only (`1.2.3`); pre-releases never count as newer.
fn parts(v: &str) -> Option<[u64; 3]> {
    let mut it = v.split('.').map(|p| p.parse::<u64>().ok());
    let out = [it.next()??, it.next()??, it.next()??];
    it.next().is_none().then_some(out)
}

fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    parts(a).cmp(&parts(b))
}

pub fn newer(candidate: &str, current: &str) -> bool {
    match (parts(candidate), parts(current)) {
        (Some(c), Some(cur)) => c > cur,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_formula_version() {
        let rb =
            "class Deepdeck < Formula\n  desc \"Task board\"\n  version \"0.4.0\"\n  on_macos do\n";
        assert_eq!(formula_version(rb).as_deref(), Some("0.4.0"));
        assert_eq!(formula_version("class X < Formula\nend\n"), None);
    }

    #[test]
    fn reads_the_index_cache() {
        let mut b = b"\x03\x02\0\0\0etag: \"x\"\x000.2.0\0".to_vec();
        b.extend_from_slice(br#"{"name":"deepdeck","vers":"0.2.0","yanked":false}"#);
        b.extend_from_slice(b"\x000.3.0\0");
        b.extend_from_slice(br#"{"name":"deepdeck","vers":"0.3.0","yanked":true}"#);
        b.extend_from_slice(b"\x000.4.0\0");
        b.extend_from_slice(br#"{"name":"deepdeck","vers":"0.4.0","yanked":false}"#);
        assert_eq!(index_cache_versions(&b), ["0.2.0", "0.4.0"]);
    }

    #[test]
    fn compares_release_numbers() {
        assert!(newer("0.4.1", "0.4.0"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(!newer("0.4.0", "0.4.0"));
        assert!(!newer("0.3.9", "0.4.0"));
        assert!(!newer("0.5.0-rc.1", "0.4.0"));
        assert!(!newer("garbage", "0.4.0"));
    }

    #[test]
    fn tells_how_it_was_installed() {
        let brew = Path::new("/opt/homebrew/Cellar/deepdeck/0.4.0/bin/ddeck");
        assert_eq!(classify(brew, None, false), Install::Brew);
        let bin = Path::new("/Users/me/.cargo/bin/ddeck");
        let registry = r#"{"installs":{"deepdeck 0.4.0 (registry+https://github.com/rust-lang/crates.io-index)":{}}}"#;
        assert_eq!(classify(bin, Some(registry), false), Install::Cargo);
        let path =
            r#"{"installs":{"deepdeck 0.4.0 (path+file:///Users/me/DeepDeck/crates/ddeck)":{}}}"#;
        let Install::Source(dir) = classify(bin, Some(path), false) else {
            panic!("a path install is a source build");
        };
        assert!(dir.ends_with("/DeepDeck/crates/ddeck"));
        assert_eq!(classify(bin, None, true), Install::Installer);
        assert_eq!(
            Install::Source("~/DeepDeck/crates/ddeck".into()).upgrade_command(),
            "cargo install --path ~/DeepDeck/crates/ddeck --locked"
        );
    }
}
