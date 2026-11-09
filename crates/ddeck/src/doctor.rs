//! `ddeck doctor` — are the hooks in place, is the log being written.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use deepdeck_core::store::{self, Paths};
use jiff::Timestamp;

use crate::install;
use crate::status::ago;

pub fn run(paths: &Paths, settings_path: &Path) -> Result<bool> {
    let mut ok = true;
    let now = Timestamp::now();
    let exe = std::env::current_exe().ok();
    println!(
        "binary     {}",
        exe.as_deref()
            .map_or("?".into(), |p| p.display().to_string())
    );
    println!("data       {}", paths.root.display());

    // Claude Code version (local process, no network).
    let claude = std::process::Command::new("claude")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    println!(
        "claude     {}",
        claude.as_deref().unwrap_or("not found in PATH")
    );

    // Hooks.
    println!("\nhooks in {}", settings_path.display());
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
                        println!("  ✗ {event:<17} missing — run `ddeck setup`");
                    }
                    Some(c)
                        if want
                            .as_deref()
                            .is_some_and(|w| c != w && !c.starts_with(&format!("{w} "))) =>
                    {
                        println!("  ! {event:<17} {c}  (not this binary)");
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
                        "
instructions  ✓ {} (v{v})",
                        path.display()
                    );
                }
                Some(v) => {
                    ok = false;
                    println!(
                        "
instructions  ! {} has v{v}, current is v{} — run `ddeck setup`",
                        path.display(),
                        crate::setup::BLOCK_VERSION
                    );
                }
                None => {
                    ok = false;
                    println!(
                        "
instructions  ✗ not in {} — run `ddeck setup` (agents won't post plans)",
                        path.display()
                    );
                }
            }
        }
        Err(e) => println!(
            "
instructions  ? {e:#}"
        ),
    }

    // A newer release brew or cargo already knows about (no network).
    match crate::update::notices(false)
        .into_iter()
        .find_map(|n| match n {
            crate::update::Notice::NewVersion(v, cmd) => Some((v, cmd)),
            crate::update::Notice::Setup { .. } => None,
        }) {
        Some((v, cmd)) => println!(
            "\nversion       ! {} installed, {v} is out — {cmd}",
            env!("CARGO_PKG_VERSION")
        ),
        None => println!(
            "\nversion       ✓ {} (newest that brew and cargo have seen)",
            env!("CARGO_PKG_VERSION")
        ),
    }

    // Log.
    println!("\nevents");
    let projects = store::load_all(paths)?;
    let mut last: BTreeMap<&'static str, Timestamp> = BTreeMap::new();
    let mut total = 0;
    for (_, events) in &projects {
        total += events.len();
        for e in events {
            let t = last.entry(e.kind()).or_insert(e.ts);
            *t = (*t).max(e.ts);
        }
    }
    println!("  {} projects, {total} events", projects.len());
    for (kind, ts) in &last {
        println!("  {kind:<15} last {} ago", ago(now, *ts));
    }

    // Hook errors.
    let log = paths.hook_log();
    match std::fs::read_to_string(&log) {
        Ok(text) if !text.trim().is_empty() => {
            let lines: Vec<&str> = text.lines().collect();
            println!(
                "\nhook errors ({} in {}), last:",
                lines.len(),
                log.display()
            );
            for l in lines.iter().rev().take(3).rev() {
                println!("  {l}");
            }
        }
        _ => println!("\nhook errors: none"),
    }
    Ok(ok)
}
