//! `ddeck hook` — called by Claude Code on every hook event.
//!
//! Contract: fast, exit code 0 no matter what, nothing on stdout (Claude Code may
//! feed hook stdout into the agent's context). Errors go to `~/.deepdeck/logs/hook.log`.

use std::io::{Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

use anyhow::{Context, Result};
use deepdeck_core::EventBody;
use deepdeck_core::hooks::{hook_name, parse_hook};
use deepdeck_core::project::{ProjectMeta, find_root, has_vcs_marker};
use deepdeck_core::store::{self, Paths};
use jiff::Timestamp;

/// Payloads bigger than this are cut (a `Stop` with a huge last message).
const MAX_PAYLOAD: u64 = 8 * 1024 * 1024;
/// `hook.log` is rotated to `hook.log.1` past this size.
const MAX_LOG: u64 = 1024 * 1024;

pub fn run(dump: bool) -> ! {
    // A panic must not print a backtrace into the agent's terminal either.
    std::panic::set_hook(Box::new(|_| {}));
    let paths = Paths::from_env();
    let res = catch_unwind(AssertUnwindSafe(|| inner(paths.as_ref(), dump)));
    let err = match res {
        Ok(Ok(())) => None,
        Ok(Err(e)) => Some(format!("{e:#}")),
        Err(p) => Some(format!(
            "panic: {}",
            p.downcast_ref::<&str>()
                .copied()
                .or_else(|| p.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("?")
        )),
    };
    if let (Some(err), Some(paths)) = (err, paths.as_ref()) {
        let _ = log_error(paths, &err);
    }
    std::process::exit(0)
}

fn inner(paths: Option<&Paths>, dump: bool) -> Result<()> {
    let paths = paths.context("neither DEEPDECK_HOME nor HOME is set")?;
    let mut raw = Vec::new();
    std::io::stdin()
        .lock()
        .take(MAX_PAYLOAD)
        .read_to_end(&mut raw)
        .context("reading stdin")?;
    let now = Timestamp::now();

    if dump {
        dump_payload(paths, &raw, now).context("writing dump")?;
    }

    let Some(mut hook) = parse_hook(&raw, now).context("parsing payload")? else {
        return Ok(());
    };
    // The hook inherits the agent's environment: remember its herdr pane.
    if let EventBody::AgentStarted { herdr_pane, .. } = &mut hook.event.body {
        *herdr_pane = std::env::var("HERDR_PANE_ID")
            .ok()
            .filter(|v| !v.is_empty());
    }
    let root = find_root(&hook.cwd, has_vcs_marker);
    let meta = ProjectMeta::for_root(&root);
    store::append(&paths.events(&meta.id), &hook.event)
        .with_context(|| format!("appending {} for {}", hook.event.kind(), root.display()))?;
    store::ensure_registered(paths, &meta).context("updating projects.json")?;
    Ok(())
}

fn dump_payload(paths: &Paths, raw: &[u8], now: Timestamp) -> Result<()> {
    let dir = paths.dumps_dir();
    store::ensure_private_dir(&dir)?;
    let name = hook_name(raw).unwrap_or_else(|| "unknown".into());
    let name: String = name.chars().filter(char::is_ascii_alphanumeric).collect();
    let file = dir.join(format!(
        "{name}-{}-{}.json",
        now.as_nanosecond(),
        std::process::id()
    ));
    store::open_append(&file)?.write_all(raw)?;
    Ok(())
}

fn log_error(paths: &Paths, msg: &str) -> std::io::Result<()> {
    let log = paths.hook_log();
    if std::fs::metadata(&log).is_ok_and(|m| m.len() > MAX_LOG) {
        let _ = std::fs::rename(&log, log.with_extension("log.1"));
    }
    let mut f = store::open_append(&log)?;
    writeln!(f, "{} {}", Timestamp::now(), msg.replace('\n', " "))
}
