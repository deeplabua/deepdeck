//! `ddeck install` / `uninstall`: merge our hooks into Claude Code's settings.json
//! without touching anyone else's (herdr and friends). Our entries are recognised
//! by their command: `<path>/ddeck hook`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

/// Hook events we subscribe to, with their matcher.
pub const HOOK_EVENTS: [(&str, Option<&str>); 6] = [
    ("SessionStart", None),
    ("UserPromptSubmit", None),
    ("PostToolUse", Some("TaskCreate|TaskUpdate|TodoWrite")),
    ("Notification", None),
    ("Stop", None),
    ("SessionEnd", None),
];

/// Seconds Claude Code waits for our hook. It normally takes a few ms.
const HOOK_TIMEOUT_SECS: u64 = 5;

/// The binary hooks should call: this one, by a path that survives upgrades.
pub fn hook_exe() -> std::io::Result<PathBuf> {
    Ok(stable_path(&std::env::current_exe()?.canonicalize()?))
}

/// Homebrew keeps each version in `<prefix>/Cellar/deepdeck/<version>/` and
/// removes it on upgrade; `<prefix>/opt/deepdeck` always points at the current one.
pub fn stable_path(exe: &Path) -> PathBuf {
    let parts: Vec<_> = exe.components().collect();
    if let Some(i) = parts
        .windows(2)
        .position(|w| w[0].as_os_str() == "Cellar" && w[1].as_os_str() == "deepdeck")
        && parts.len() > i + 3
    {
        let mut p: PathBuf = parts[..i].iter().collect();
        p.push("opt/deepdeck");
        p.extend(&parts[i + 3..]);
        return p;
    }
    exe.to_path_buf()
}

/// The command string for a binary path, quoted if it has spaces.
pub fn hook_command(exe: &Path) -> String {
    command_for(&exe.display().to_string(), cfg!(windows))
}

/// Claude Code runs hook commands through a shell — Git Bash on Windows, where
/// `\` is an escape: Windows paths get forward slashes (cmd accepts them too)
/// and double quotes when they contain spaces.
fn command_for(path: &str, windows: bool) -> String {
    if windows {
        let p = path.replace('\\', "/");
        return if p.contains(char::is_whitespace) {
            format!("\"{p}\" hook")
        } else {
            format!("{p} hook")
        };
    }
    if path.contains(|c: char| c.is_whitespace() || c == '\'') {
        format!("'{}' hook", path.replace('\'', r"'\''"))
    } else {
        format!("{path} hook")
    }
}

/// True for `ddeck hook`, `/abs/ddeck hook --dump`, `'/a b/ddeck' hook`,
/// `"C:/a b/ddeck.exe" hook`.
pub fn is_our_command(cmd: &str) -> bool {
    let cmd = cmd.trim();
    let (prog, rest) = match cmd.chars().next() {
        Some(q @ ('\'' | '"')) => match cmd[1..].find(q) {
            Some(end) => (&cmd[1..=end], &cmd[end + 2..]),
            None => return false,
        },
        _ => cmd.split_once(char::is_whitespace).unwrap_or((cmd, "")),
    };
    let name = prog.rsplit(['/', '\\']).next().unwrap_or(prog);
    matches!(name, "ddeck" | "ddeck.exe") && rest.split_whitespace().next() == Some("hook")
}

fn hooks_obj(settings: &mut Value) -> Result<&mut Map<String, Value>> {
    let root = settings
        .as_object_mut()
        .context("settings.json is not a JSON object")?;
    let hooks = root.entry("hooks").or_insert_with(|| json!({}));
    hooks
        .as_object_mut()
        .context("`hooks` in settings.json is not an object")
}

fn group_has_ours(group: &Value) -> bool {
    group["hooks"].as_array().is_some_and(|hs| {
        hs.iter()
            .any(|h| h["command"].as_str().is_some_and(is_our_command))
    })
}

/// Add or update our hooks. Returns whether anything changed.
pub fn merge(settings: &mut Value, command: &str) -> Result<bool> {
    let hooks = hooks_obj(settings)?;
    let mut changed = false;
    for (event, matcher) in HOOK_EVENTS {
        let groups = hooks.entry(event).or_insert_with(|| json!([]));
        let Some(groups) = groups.as_array_mut() else {
            bail!("`hooks.{event}` in settings.json is not an array");
        };
        let mut found = false;
        for group in groups.iter_mut().filter(|g| group_has_ours(g)) {
            found = true;
            let only_ours = group["hooks"].as_array().is_some_and(|hs| hs.len() == 1);
            if only_ours && group.get("matcher").and_then(Value::as_str) != matcher {
                match matcher {
                    Some(m) => group["matcher"] = json!(m),
                    None => {
                        group.as_object_mut().map(|g| g.remove("matcher"));
                    }
                }
                changed = true;
            }
            for h in group["hooks"].as_array_mut().into_iter().flatten() {
                if h["command"].as_str().is_some_and(is_our_command) && h["command"] != command {
                    h["command"] = json!(command);
                    changed = true;
                }
            }
        }
        if !found {
            let mut group = Map::new();
            if let Some(m) = matcher {
                group.insert("matcher".into(), json!(m));
            }
            group.insert(
                "hooks".into(),
                json!([{ "type": "command", "command": command, "timeout": HOOK_TIMEOUT_SECS }]),
            );
            groups.push(Value::Object(group));
            changed = true;
        }
    }
    Ok(changed)
}

/// Remove our hooks only. Returns whether anything changed.
pub fn remove(settings: &mut Value) -> Result<bool> {
    let Some(root) = settings.as_object_mut() else {
        bail!("settings.json is not a JSON object");
    };
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return Ok(false);
    };
    let mut changed = false;
    for groups in hooks.values_mut().filter_map(Value::as_array_mut) {
        for group in groups.iter_mut() {
            if let Some(hs) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                let before = hs.len();
                hs.retain(|h| !h["command"].as_str().is_some_and(is_our_command));
                changed |= hs.len() != before;
            }
        }
        groups.retain(|g| !g["hooks"].as_array().is_some_and(Vec::is_empty));
    }
    if changed {
        hooks.retain(|_, v| !v.as_array().is_some_and(Vec::is_empty));
        if hooks.is_empty() {
            root.remove("hooks");
        }
    }
    Ok(changed)
}

/// Which of our events are present, and with what command.
pub fn installed(settings: &Value) -> Vec<(&'static str, Option<String>)> {
    HOOK_EVENTS
        .iter()
        .map(|(event, _)| {
            let cmd = settings["hooks"][event]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|g| g["hooks"].as_array().into_iter().flatten())
                .filter_map(|h| h["command"].as_str())
                .find(|c| is_our_command(c))
                .map(str::to_owned);
            (*event, cmd)
        })
        .collect()
}

// ── file edge ───────────────────────────────────────────────────────────────

/// `$CLAUDE_CONFIG_DIR/settings.json`, else `~/.claude/settings.json`.
pub fn default_settings_path() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(dir).join("settings.json"));
    }
    let home = std::env::home_dir().context("cannot find the home directory")?;
    Ok(home.join(".claude").join("settings.json"))
}

/// Original text (empty if missing) and parsed value. Broken JSON is an error:
/// we never overwrite a file we could not read.
pub fn load_settings(path: &Path) -> Result<(String, Value)> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok((text, json!({}))),
        Ok(text) => {
            let v = serde_json::from_str(&text).with_context(|| {
                format!(
                    "{} is not valid JSON — fix it first, nothing was changed",
                    path.display()
                )
            })?;
            Ok((text, v))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok((String::new(), json!({}))),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

pub fn render(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("a Value always serializes");
    s.push('\n');
    s
}

pub fn diff(path: &Path, old: &str, new: &str) -> String {
    similar::TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(3)
        .header(
            &path.display().to_string(),
            &format!("{} (new)", path.display()),
        )
        .to_string()
}

/// Back up (if the file exists) and write atomically. Returns the backup path.
pub fn write_with_backup(path: &Path, text: &str) -> Result<Option<PathBuf>> {
    write_with_backup_in(path, text, None)
}

/// Like [`write_with_backup`], with the backup in `backup_dir` instead of next
/// to the file (keeps backups out of a repository's working tree).
pub fn write_with_backup_in(
    path: &Path,
    text: &str,
    backup_dir: Option<&Path>,
) -> Result<Option<PathBuf>> {
    let backup = if path.exists() {
        let stamp = jiff::Zoned::now().strftime("%Y%m%d-%H%M%S").to_string();
        let name = format!(
            "{}.ddeck-backup-{stamp}",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("file")
        );
        let backup = match backup_dir {
            Some(dir) => {
                deepdeck_core::store::ensure_private_dir(dir)?;
                dir.join(&name)
            }
            None => path.with_file_name(&name),
        };
        // Two writes within a second must not overwrite the first backup.
        let mut backup = backup;
        let mut n = 2;
        while backup.exists() {
            backup.set_file_name(format!("{name}-{n}"));
            n += 1;
        }
        std::fs::copy(path, &backup)
            .with_context(|| format!("backing up to {}", backup.display()))?;
        Some(backup)
    } else {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        None
    };
    let tmp = path.with_file_name(format!(
        "{}.ddeck-tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("file")
    ));
    std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    if let Ok(meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CMD: &str = "/Users/dev/.cargo/bin/ddeck hook";

    #[test]
    fn brew_installs_hook_the_stable_path() {
        assert_eq!(
            stable_path(Path::new("/opt/homebrew/Cellar/deepdeck/0.4.1/bin/ddeck")),
            Path::new("/opt/homebrew/opt/deepdeck/bin/ddeck")
        );
        assert_eq!(
            stable_path(Path::new(
                "/home/linuxbrew/.linuxbrew/Cellar/deepdeck/0.4.1/bin/ddeck"
            )),
            Path::new("/home/linuxbrew/.linuxbrew/opt/deepdeck/bin/ddeck")
        );
        let cargo = Path::new("/Users/dev/.cargo/bin/ddeck");
        assert_eq!(stable_path(cargo), cargo);
    }

    fn herdr() -> Value {
        json!({
            "theme": "dark",
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "herdr integration hook claude stop" }] }],
                "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "guard.sh" }] }]
            }
        })
    }

    #[test]
    fn recognises_our_command_only() {
        assert!(is_our_command("ddeck hook"));
        assert!(is_our_command("/Users/dev/.cargo/bin/ddeck hook --dump"));
        assert!(is_our_command("'/Users/my dev/bin/ddeck' hook"));
        assert!(!is_our_command("ddeck status"));
        assert!(!is_our_command("/opt/ddeck-old/herdr hook"));
        assert!(!is_our_command("echo ddeck hook"));
        assert!(!is_our_command("'/broken hook"));
        assert_eq!(command_for("/a b/ddeck", false), "'/a b/ddeck' hook");
        assert!(is_our_command(&command_for("/a b/ddeck", false)));
        assert!(is_our_command(&hook_command(Path::new("/a b/ddeck"))));
    }

    #[test]
    fn windows_paths_survive_git_bash() {
        let plain = r"C:\Users\dev\.cargo\bin\ddeck.exe";
        assert_eq!(
            command_for(plain, true),
            "C:/Users/dev/.cargo/bin/ddeck.exe hook"
        );
        let spaced = r"C:\Users\Dev User\.cargo\bin\ddeck.exe";
        assert_eq!(
            command_for(spaced, true),
            "\"C:/Users/Dev User/.cargo/bin/ddeck.exe\" hook"
        );
        for p in [plain, spaced] {
            assert!(is_our_command(&command_for(p, true)), "{p}");
        }
        assert!(is_our_command(r"C:\bin\ddeck.exe hook"));
        assert!(!is_our_command("C:/bin/ddeck.exe status"));
    }

    #[test]
    fn into_empty_settings() {
        let mut s = json!({});
        assert!(merge(&mut s, CMD).unwrap());
        assert_eq!(
            installed(&s)
                .iter()
                .filter(|(_, c)| c.as_deref() == Some(CMD))
                .count(),
            6
        );
        assert_eq!(
            s["hooks"]["PostToolUse"][0]["matcher"],
            "TaskCreate|TaskUpdate|TodoWrite"
        );
        assert!(s["hooks"]["Stop"][0].get("matcher").is_none());
    }

    #[test]
    fn keeps_foreign_hooks_and_is_idempotent() {
        let mut s = herdr();
        assert!(merge(&mut s, CMD).unwrap());
        let once = s.clone();
        assert!(!merge(&mut s, CMD).unwrap());
        assert_eq!(s, once);
        assert_eq!(s["theme"], "dark");
        assert_eq!(s["hooks"]["Stop"].as_array().unwrap().len(), 2);
        assert_eq!(
            s["hooks"]["Stop"][0]["hooks"][0]["command"],
            "herdr integration hook claude stop"
        );
        assert_eq!(s["hooks"]["PreToolUse"], herdr()["hooks"]["PreToolUse"]);
    }

    #[test]
    fn updates_binary_path_in_place() {
        let mut s = herdr();
        merge(&mut s, "/old/ddeck hook").unwrap();
        assert!(merge(&mut s, CMD).unwrap());
        assert_eq!(s["hooks"]["Stop"].as_array().unwrap().len(), 2);
        assert!(installed(&s).iter().all(|(_, c)| c.as_deref() == Some(CMD)));
    }

    #[test]
    fn uninstall_restores_the_original() {
        let mut s = herdr();
        merge(&mut s, CMD).unwrap();
        assert!(remove(&mut s).unwrap());
        assert_eq!(s, herdr());
        assert!(!remove(&mut s).unwrap());

        let mut empty = json!({ "theme": "x" });
        merge(&mut empty, CMD).unwrap();
        remove(&mut empty).unwrap();
        assert_eq!(empty, json!({ "theme": "x" }));
    }

    #[test]
    fn refuses_unexpected_shapes() {
        assert!(merge(&mut json!([]), CMD).is_err());
        assert!(merge(&mut json!({ "hooks": [] }), CMD).is_err());
        assert!(merge(&mut json!({ "hooks": { "Stop": {} } }), CMD).is_err());
    }

    #[test]
    fn broken_json_is_refused_without_writing() {
        let dir = std::env::temp_dir().join(format!("ddeck-install-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        std::fs::write(&path, "{ \"hooks\": ").unwrap();
        assert!(load_settings(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ \"hooks\": ");
        let (text, v) = load_settings(&dir.join("missing.json")).unwrap();
        assert!(text.is_empty() && v == json!({}));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_makes_a_backup() {
        let dir = std::env::temp_dir().join(format!("ddeck-backup-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        std::fs::write(&path, "{}").unwrap();
        let backup = write_with_backup(&path, "{\"a\":1}\n").unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(backup).unwrap(), "{}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"a\":1}\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
