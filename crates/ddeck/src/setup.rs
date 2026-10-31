//! `ddeck setup`: teach agents to keep the board current. Claude Code 2.1.311+
//! has no task-list tools, so the agent posts its plan through `ddeck plan` /
//! `ddeck step`. The instructions live in a marked block of a CLAUDE.md —
//! the global one by default (hooks are global too), or a repository's.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Bump when the block text changes, so `setup` and `doctor` see stale copies.
pub const BLOCK_VERSION: u32 = 4;
const BEGIN_PREFIX: &str = "<!-- ddeck:begin";
const END: &str = "<!-- ddeck:end -->";

/// The block as written into CLAUDE.md. English: the agent reads it.
pub fn block() -> String {
    format!(
        "{BEGIN_PREFIX} v{BLOCK_VERSION} -->
## DeepDeck task board

The user follows your work on a task board. Keep it current:

- When you start multi-step work, post the plan: `ddeck plan \"step 1\" \"step 2\" …` (replaces the previous plan).
- Mark progress as you go: `ddeck step <n> doing`, then `ddeck step <n> done`.
- When you lay out what comes after this request (next sessions, follow-ups, steps for the user), post it as the project roadmap: `ddeck roadmap set \"…\" \"…\"` (replaces it; `ddeck roadmap` shows the current one). Mark progress with `ddeck roadmap step <n> done`.
- A question only the user can answer: `ddeck ask \"…\"`, then continue with a safe default if you can.
- An idea or fix deferred for later: `ddeck idea \"…\"`.
- If the user confirms their message was meant for another session, run `ddeck retract \"<a few words from it>\"` (when they name where it belonged, add `--to <that project>`).
- When you finish a piece of work (a commit, the end of a task), save a 2–4 sentence summary — what was done, what was decided, what is left: `ddeck summary \"…\"`.
- Before starting work in a project, read `ddeck list`: open items, the roadmap and what the last session left.

Each command prints one line; do not repeat it in your reply.
{END}"
    )
}

/// Where the block is, as line-start byte offsets `[begin, end_of_end_line)`.
fn find_block(text: &str) -> Result<Option<(usize, usize)>> {
    let begin = text.find(BEGIN_PREFIX);
    let end = text.find(END);
    match (begin, end) {
        (None, None) => Ok(None),
        (Some(b), Some(e)) if b < e => {
            let start = text[..b].rfind('\n').map_or(0, |i| i + 1);
            let stop = text[e..].find('\n').map_or(text.len(), |i| e + i + 1);
            Ok(Some((start, stop)))
        }
        _ => bail!(
            "the DeepDeck block markers are damaged (begin/end missing or out of order) — fix them by hand"
        ),
    }
}

/// Version written in an existing block, if any.
pub fn installed_version(text: &str) -> Option<u32> {
    let at = text.find(BEGIN_PREFIX)? + BEGIN_PREFIX.len();
    let rest = text[at..].trim_start().strip_prefix('v')?;
    rest.split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

/// Insert or replace the block. Other content is left byte-for-byte as is.
pub fn upsert_block(text: &str, block: &str) -> Result<String> {
    let mut block = block.trim_end().to_owned();
    block.push('\n');
    Ok(match find_block(text)? {
        Some((start, stop)) => format!("{}{block}{}", &text[..start], &text[stop..]),
        None if text.trim().is_empty() => block,
        None => format!("{}\n\n{block}", text.trim_end()),
    })
}

/// Remove the block and the blank line we added before it.
pub fn remove_block(text: &str) -> Result<String> {
    let Some((start, stop)) = find_block(text)? else {
        return Ok(text.to_owned());
    };
    let head = text[..start].trim_end_matches('\n');
    let tail = &text[stop..];
    Ok(match (head.is_empty(), tail.trim().is_empty()) {
        (true, _) => tail.trim_start_matches('\n').to_owned(),
        (false, true) => format!("{head}\n"),
        (false, false) => format!("{head}\n\n{}", tail.trim_start_matches('\n')),
    })
}

// ── file edge ───────────────────────────────────────────────────────────────

/// `$CLAUDE_CONFIG_DIR/CLAUDE.md`, else `~/.claude/CLAUDE.md`.
pub fn global_claude_md() -> Result<PathBuf> {
    Ok(crate::install::default_settings_path()?.with_file_name("CLAUDE.md"))
}

/// `CLAUDE.md` at the repository root containing `dir`.
pub fn project_claude_md(dir: &Path) -> Result<PathBuf> {
    let dir = dir
        .canonicalize()
        .with_context(|| format!("no such directory: {}", dir.display()))?;
    let root = deepdeck_core::project::find_root(&dir, deepdeck_core::project::has_vcs_marker);
    Ok(root.join("CLAUDE.md"))
}

fn read_or_empty(path: &Path) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(t) => Ok(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

/// Write the block into `path`. Returns whether anything changed.
/// `backup_dir`: `None` — next to the file; `Some` — elsewhere (repositories).
pub fn apply_block(path: &Path, dry_run: bool, backup_dir: Option<&Path>) -> Result<bool> {
    let old = read_or_empty(path)?;
    let new = upsert_block(&old, &block()).with_context(|| path.display().to_string())?;
    write_if_changed(path, &old, &new, dry_run, backup_dir)
}

/// Remove the block from `path` (deleting a file that only held the block).
pub fn unapply_block(path: &Path, dry_run: bool, backup_dir: Option<&Path>) -> Result<bool> {
    let old = read_or_empty(path)?;
    let new = remove_block(&old).with_context(|| path.display().to_string())?;
    if new == old {
        return Ok(false);
    }
    if new.trim().is_empty() {
        // Nothing but our block: no backup needed, `setup` recreates it.
        print!("{}", crate::install::diff(path, &old, ""));
        if dry_run {
            println!("\n(dry run — {} would be removed)", path.display());
        } else {
            std::fs::remove_file(path).with_context(|| format!("removing {}", path.display()))?;
            println!("\nremoved {}", path.display());
        }
        return Ok(true);
    }
    write_if_changed(path, &old, &new, dry_run, backup_dir)
}

fn write_if_changed(
    path: &Path,
    old: &str,
    new: &str,
    dry_run: bool,
    backup_dir: Option<&Path>,
) -> Result<bool> {
    if old == new {
        return Ok(false);
    }
    print!("{}", crate::install::diff(path, old, new));
    if dry_run {
        println!("\n(dry run — nothing written)");
    } else {
        match crate::install::write_with_backup_in(path, new, backup_dir)? {
            Some(b) => println!("\nwrote {} (backup: {})", path.display(), b.display()),
            None => println!("\nwrote {}", path.display()),
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_empty_and_existing_files() {
        let b = block();
        assert_eq!(upsert_block("", &b).unwrap(), format!("{b}\n"));
        assert_eq!(
            upsert_block("# Me\n\nUse jj.\n", &b).unwrap(),
            format!("# Me\n\nUse jj.\n\n{b}\n")
        );
    }

    #[test]
    fn replaces_in_place_and_is_idempotent() {
        let old =
            "# Me\n\n<!-- ddeck:begin v0 -->\nold text\n<!-- ddeck:end -->\n\n## Tail\nkeep\n";
        let new = upsert_block(old, &block()).unwrap();
        assert!(new.starts_with(&format!("# Me\n\n<!-- ddeck:begin v{BLOCK_VERSION} -->")));
        assert!(new.ends_with("<!-- ddeck:end -->\n\n## Tail\nkeep\n"));
        assert!(!new.contains("old text"));
        assert_eq!(upsert_block(&new, &block()).unwrap(), new);
        assert_eq!(installed_version(old), Some(0));
        assert_eq!(installed_version(&new), Some(BLOCK_VERSION));
        assert_eq!(installed_version("# nothing"), None);
    }

    #[test]
    fn remove_restores_the_original() {
        for original in ["", "# Me\n\nUse jj.\n"] {
            let with = upsert_block(original, &block()).unwrap();
            assert_eq!(remove_block(&with).unwrap(), original);
        }
        let middle = "# A\n\n<!-- ddeck:begin v1 -->\nx\n<!-- ddeck:end -->\n\n# B\n";
        assert_eq!(remove_block(middle).unwrap(), "# A\n\n# B\n");
        assert_eq!(remove_block("# none\n").unwrap(), "# none\n");
    }

    #[test]
    fn damaged_markers_are_refused() {
        assert!(upsert_block("<!-- ddeck:begin v1 -->\nno end\n", &block()).is_err());
        assert!(upsert_block("<!-- ddeck:end -->\n<!-- ddeck:begin v1 -->\n", &block()).is_err());
        assert!(remove_block("<!-- ddeck:end -->\n").is_err());
    }

    #[test]
    fn block_mentions_every_command() {
        let b = block();
        for cmd in [
            "ddeck plan",
            "ddeck step",
            "ddeck roadmap set",
            "ddeck roadmap step",
            "ddeck ask",
            "ddeck idea",
            "ddeck retract",
            "ddeck summary",
            "ddeck list",
        ] {
            assert!(b.contains(cmd), "{cmd}");
        }
    }
}
