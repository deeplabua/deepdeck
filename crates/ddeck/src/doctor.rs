//! `ddeck doctor` — are the hooks in place, is the log being written.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use deepdeck_core::store::{self, Paths};
use jiff::Timestamp;

use crate::install;

/// A label padded to `width` terminal cells (CJK labels are two cells a character).
fn label(text: &str, width: usize) -> String {
    let w = unicode_width::UnicodeWidthStr::width(text);
    format!("{text}{}", " ".repeat(width.saturating_sub(w) + 1))
}

/// Width of the left column: the widest label of this language.
fn column(t: &crate::i18n::Strings) -> usize {
    [
        t.dr_binary,
        t.dr_data,
        t.dr_claude,
        t.dr_instructions,
        t.dr_version,
    ]
    .iter()
    .map(|l| unicode_width::UnicodeWidthStr::width(*l))
    .max()
    .unwrap_or(0)
}

pub fn run(paths: &Paths, settings_path: &Path) -> Result<bool> {
    let mut ok = true;
    let now = Timestamp::now();
    let t = crate::i18n::t();
    let col = column(t);
    let ago = |ts: Timestamp| (t.ago)(now.duration_since(ts).as_secs());
    let exe = std::env::current_exe().ok();
    println!(
        "{}{}",
        label(t.dr_binary, col),
        exe.as_deref()
            .map_or("?".into(), |p| p.display().to_string())
    );
    println!("{}{}", label(t.dr_data, col), paths.root.display());

    // Claude Code version (local process, no network).
    let claude = std::process::Command::new("claude")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    println!(
        "{}{}",
        label(t.dr_claude, col),
        claude.as_deref().unwrap_or(t.dr_claude_missing)
    );

    // Hooks.
    println!(
        "\n{}",
        (t.dr_hooks_in)(&settings_path.display().to_string())
    );
    match install::load_settings(settings_path) {
        Err(e) => {
            ok = false;
            println!("  ✗ {e:#}");
        }
        Ok((_, settings)) => {
            let want = install::hook_exe().ok().map(|e| install::hook_command(&e));
            for (event, cmd) in install::installed(&settings) {
                match cmd {
                    None => {
                        ok = false;
                        println!("  ✗ {event:<17} {}", t.dr_hook_missing);
                    }
                    Some(c)
                        if want
                            .as_deref()
                            .is_some_and(|w| c != w && !c.starts_with(&format!("{w} "))) =>
                    {
                        println!("  ! {event:<17} {c}  {}", t.dr_not_this_binary);
                    }
                    Some(c) => println!("  ✓ {event:<17} {c}"),
                }
            }
        }
    }

    // Agent instructions (global CLAUDE.md).
    match crate::setup::global_claude_md() {
        Ok(path) => {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            match crate::setup::installed_version(&text) {
                Some(v) if v == crate::setup::BLOCK_VERSION => {
                    println!(
                        "\n{}✓ {} (v{v})",
                        label(t.dr_instructions, col),
                        path.display()
                    );
                }
                Some(v) => {
                    ok = false;
                    println!(
                        "\n{}! {}",
                        label(t.dr_instructions, col),
                        (t.dr_instr_old)(
                            &path.display().to_string(),
                            v,
                            crate::setup::BLOCK_VERSION
                        )
                    );
                }
                None => {
                    ok = false;
                    println!(
                        "\n{}✗ {}",
                        label(t.dr_instructions, col),
                        (t.dr_instr_none)(&path.display().to_string())
                    );
                }
            }
        }
        Err(e) => println!("\n{}? {e:#}", label(t.dr_instructions, col)),
    }

    // A newer release brew or cargo already knows about (no network).
    match crate::update::notices(false)
        .into_iter()
        .find_map(|n| match n {
            crate::update::Notice::NewVersion(v, cmd) => Some((v, cmd)),
            crate::update::Notice::Setup { .. } => None,
        }) {
        Some((v, cmd)) => println!(
            "\n{}! {}",
            label(t.dr_version, col),
            (t.dr_version_new)(env!("CARGO_PKG_VERSION"), &v, &cmd)
        ),
        None => println!(
            "\n{}✓ {}",
            label(t.dr_version, col),
            (t.dr_version_ok)(env!("CARGO_PKG_VERSION"))
        ),
    }

    // Log.
    println!("\n{}", t.dr_events);
    let projects = store::load_all(paths)?;
    let mut last: BTreeMap<&'static str, Timestamp> = BTreeMap::new();
    let mut total = 0;
    for (_, events) in &projects {
        total += events.len();
        for e in events {
            let ts = last.entry(e.kind()).or_insert(e.ts);
            *ts = (*ts).max(e.ts);
        }
    }
    println!("  {}", (t.dr_events_total)(projects.len(), total));
    for (kind, ts) in &last {
        println!("  {kind:<15} {}", (t.dr_last)(&ago(*ts)));
    }

    // Hook errors.
    let log = paths.hook_log();
    match std::fs::read_to_string(&log) {
        Ok(text) if !text.trim().is_empty() => {
            let lines: Vec<&str> = text.lines().collect();
            println!(
                "\n{}",
                (t.dr_hook_errors)(lines.len(), &log.display().to_string())
            );
            for l in lines.iter().rev().take(3).rev() {
                println!("  {l}");
            }
        }
        _ => println!("\n{}", t.dr_hook_errors_none),
    }
    Ok(ok)
}
