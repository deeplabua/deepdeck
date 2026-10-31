//! `~/.deepdeck/` on disk: per-project append-only logs and the project registry.
//!
//! Many processes write concurrently (every hook call is its own process), so a
//! log is only ever appended to: one `write` of one line under an exclusive lock.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::event::{Event, parse_line};
use crate::project::{ProjectId, ProjectMeta};

/// Layout of the data directory.
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    /// `$DEEPDECK_HOME`, else `~/.deepdeck` (`%USERPROFILE%` on Windows).
    pub fn from_env() -> Option<Self> {
        if let Some(dir) = std::env::var_os("DEEPDECK_HOME").filter(|v| !v.is_empty()) {
            return Some(Paths { root: dir.into() });
        }
        let home = std::env::home_dir().filter(|h| !h.as_os_str().is_empty())?;
        Some(Paths {
            root: home.join(".deepdeck"),
        })
    }

    pub fn new(root: impl Into<PathBuf>) -> Self {
        Paths { root: root.into() }
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    pub fn events(&self, id: &ProjectId) -> PathBuf {
        self.projects_dir().join(id.as_str()).join("events.jsonl")
    }

    pub fn registry(&self) -> PathBuf {
        self.root.join("projects.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn hook_log(&self) -> PathBuf {
        self.logs_dir().join("hook.log")
    }

    pub fn dumps_dir(&self) -> PathBuf {
        self.root.join("dumps")
    }
}

/// Create a directory (and parents) readable only by the owner.
pub fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    let mut b = fs::DirBuilder::new();
    b.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        b.mode(0o700);
    }
    b.create(dir)
}

/// Open for appending, creating the file with owner-only permissions.
pub fn open_append(path: &Path) -> io::Result<File> {
    let mut o = OpenOptions::new();
    o.create(true).append(true);
    open_private(path, o)
}

/// Open (creating with owner-only permissions) with the given options.
fn open_private(path: &Path, o: OpenOptions) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    #[cfg(unix)]
    let o = {
        use std::os::unix::fs::OpenOptionsExt;
        let mut o = o;
        o.mode(0o600);
        o
    };
    o.open(path)
}

/// Append one event as one line, under an exclusive lock.
pub fn append(path: &Path, event: &Event) -> io::Result<()> {
    let mut line = serde_json::to_vec(event).map_err(io::Error::other)?;
    line.push(b'\n');
    let mut f = open_append(path)?;
    f.lock()?;
    let res = f.write_all(&line);
    let _ = f.unlock();
    res
}

/// Result of reading a log from an offset.
#[derive(Debug, Default)]
pub struct ReadChunk {
    pub events: Vec<Event>,
    /// Offset just past the last complete line; pass it to the next call.
    pub next_offset: u64,
    /// Lines that could not be parsed (unknown kind, garbage).
    pub skipped: usize,
}

/// Read complete lines starting at `offset`. A trailing partial line (a writer
/// in the middle of `write`) is left for the next call. A missing file is empty.
pub fn read_from(path: &Path, offset: u64) -> io::Result<ReadChunk> {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(ReadChunk {
                next_offset: offset,
                ..Default::default()
            });
        }
        Err(e) => return Err(e),
    };
    let len = f.metadata()?.len();
    // The log shrank (rotated or replaced): start over.
    let offset = if offset > len { 0 } else { offset };
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let complete = buf.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    let mut chunk = ReadChunk {
        next_offset: offset + complete as u64,
        ..Default::default()
    };
    for line in buf[..complete].split(|&b| b == b'\n') {
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        match std::str::from_utf8(line).ok().and_then(parse_line) {
            Some(e) => chunk.events.push(e),
            None => chunk.skipped += 1,
        }
    }
    Ok(chunk)
}

/// `projects.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub projects: Vec<ProjectMeta>,
}

impl Registry {
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Registry::default()),
            Err(e) => Err(e),
        }
    }

    pub fn get(&self, id: &ProjectId) -> Option<&ProjectMeta> {
        self.projects.iter().find(|p| &p.id == id)
    }
}

/// Make sure `meta` is in the registry. Cheap when it already is (one read, no
/// lock, no write) — the common case for a hook call.
pub fn ensure_registered(paths: &Paths, meta: &ProjectMeta) -> io::Result<()> {
    let path = paths.registry();
    if Registry::load(&path).is_ok_and(|r| r.get(&meta.id) == Some(meta)) {
        return Ok(());
    }
    update_registry(paths, |reg| {
        match reg.projects.iter_mut().find(|p| p.id == meta.id) {
            // Another hook may have just added it.
            Some(p) if p == meta => false,
            Some(p) => {
                *p = meta.clone();
                true
            }
            None => {
                reg.projects.push(meta.clone());
                true
            }
        }
    })
}

/// Read-modify-write `projects.json` under `projects.lock`; `change` returns
/// whether it changed anything (nothing is written otherwise).
fn update_registry(paths: &Paths, change: impl FnOnce(&mut Registry) -> bool) -> io::Result<()> {
    let path = paths.registry();
    ensure_private_dir(&paths.root)?;
    let lock = open_append(&paths.root.join("projects.lock"))?;
    lock.lock()?;
    let res = (|| {
        let mut reg = Registry::load(&path).unwrap_or_default();
        if !change(&mut reg) {
            return Ok(());
        }
        let tmp = path.with_extension("json.tmp");
        // Not append mode: Windows cannot truncate a file opened for appending.
        let mut o = OpenOptions::new();
        o.create(true).write(true).truncate(true);
        let mut f = open_private(&tmp, o)?;
        f.write_all(&serde_json::to_vec_pretty(&reg).map_err(io::Error::other)?)?;
        f.sync_all()?;
        // Windows cannot rename a file that is still open.
        drop(f);
        fs::rename(&tmp, &path)
    })();
    let _ = lock.unlock();
    res
}

/// Drop a project from the board: remove it from the registry and move its
/// directory to `forgotten/<id>-<unix seconds>` (nothing is deleted). Returns
/// where the log went. If its agent becomes active again, the hook re-registers
/// it with a fresh log.
pub fn forget(paths: &Paths, id: &ProjectId) -> io::Result<Option<PathBuf>> {
    update_registry(paths, |reg| {
        let before = reg.projects.len();
        reg.projects.retain(|p| &p.id != id);
        reg.projects.len() != before
    })?;
    let dir = paths.projects_dir().join(id.as_str());
    if !dir.exists() {
        return Ok(None);
    }
    let dest_dir = paths.root.join("forgotten");
    ensure_private_dir(&dest_dir)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let dest = dest_dir.join(format!("{id}-{stamp}"));
    fs::rename(&dir, &dest)?;
    Ok(Some(dest))
}

/// Result of [`rotate`] for one log.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Rotated {
    pub archived: usize,
    pub kept: usize,
}

/// Move events the board no longer needs (see [`crate::archive::keep_mask`])
/// from `events.jsonl` to `archive/<YYYY-MM>.jsonl` next to it.
///
/// Done in place under the same lock as [`append`]: the archive is written and
/// synced first, then the log is truncated and the kept lines written back. No
/// tmp+rename — a hook that already opened the log and waits for the lock
/// would append to a deleted file. A crash between the two steps leaves
/// duplicates in the archive, never a lost event. Lines that do not parse stay.
pub fn rotate(log: &Path, cutoff: jiff::Timestamp) -> io::Result<Rotated> {
    // Read+write, not append: Windows cannot truncate a file opened for
    // appending. Appenders use their own append handles, so their lines still
    // land at the end.
    let mut f = match OpenOptions::new().read(true).write(true).open(log) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Rotated::default()),
        Err(e) => return Err(e),
    };
    f.lock()?;
    let res = rotate_locked(&mut f, log, cutoff);
    let _ = f.unlock();
    res
}

fn rotate_locked(f: &mut File, log: &Path, cutoff: jiff::Timestamp) -> io::Result<Rotated> {
    let mut buf = Vec::new();
    f.seek(SeekFrom::Start(0))?;
    f.read_to_end(&mut buf)?;
    // A partial last line cannot exist under the lock, but keep it if it does.
    let lines: Vec<&[u8]> = buf
        .split_inclusive(|&b| b == b'\n')
        .filter(|l| !l.iter().all(u8::is_ascii_whitespace))
        .collect();
    let parsed: Vec<Option<Event>> = lines
        .iter()
        .map(|l| std::str::from_utf8(l).ok().and_then(parse_line))
        .collect();
    let events: Vec<Event> = parsed.iter().flatten().cloned().collect();
    let mut mask = crate::archive::keep_mask(&events, cutoff).into_iter();
    let keep: Vec<bool> = parsed
        .iter()
        .map(|p| p.is_none() || mask.next().unwrap_or(true))
        .collect();
    let archived = keep.iter().filter(|k| !**k).count();
    if archived == 0 {
        return Ok(Rotated {
            archived: 0,
            kept: lines.len(),
        });
    }

    // Archive first, grouped by month, each file synced.
    let dir = log.with_file_name("archive");
    let mut by_month: Vec<(String, Vec<u8>)> = Vec::new();
    for ((line, ev), k) in lines.iter().zip(&parsed).zip(&keep) {
        let (false, Some(ev)) = (*k, ev) else {
            continue;
        };
        let month = ev.ts.strftime("%Y-%m").to_string();
        let i = match by_month.iter().position(|(m, _)| *m == month) {
            Some(i) => i,
            None => {
                by_month.push((month, Vec::new()));
                by_month.len() - 1
            }
        };
        by_month[i].1.extend_from_slice(line);
        if !line.ends_with(b"\n") {
            by_month[i].1.push(b'\n');
        }
    }
    for (month, data) in &by_month {
        let mut a = open_append(&dir.join(format!("{month}.jsonl")))?;
        a.write_all(data)?;
        a.sync_all()?;
    }

    // Then the live log: truncate and write back what stays.
    let mut rest = Vec::new();
    for (line, k) in lines.iter().zip(&keep) {
        if *k {
            rest.extend_from_slice(line);
        }
    }
    f.set_len(0)?;
    f.seek(SeekFrom::Start(0))?;
    f.write_all(&rest)?;
    f.sync_all()?;
    Ok(Rotated {
        archived,
        kept: lines.len() - archived,
    })
}

/// [`rotate`] every registered project's log. Errors per project are skipped:
/// archiving is housekeeping and must not stop the board from opening.
pub fn rotate_all(paths: &Paths, cutoff: jiff::Timestamp) -> Rotated {
    let reg = Registry::load(&paths.registry()).unwrap_or_default();
    let mut total = Rotated::default();
    for meta in reg.projects {
        if let Ok(r) = rotate(&paths.events(&meta.id), cutoff) {
            total.archived += r.archived;
            total.kept += r.kept;
        }
    }
    total
}

/// Every registered project with all of its events, in log order.
pub fn load_all(paths: &Paths) -> io::Result<Vec<(ProjectMeta, Vec<Event>)>> {
    let reg = Registry::load(&paths.registry())?;
    reg.projects
        .into_iter()
        .map(|meta| {
            let events = read_from(&paths.events(&meta.id), 0)?.events;
            Ok((meta, events))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventBody, Source};

    fn tmp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("deepdeck-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn ev(text: &str) -> Event {
        Event::new(
            "2026-10-31T06:00:00Z".parse().unwrap(),
            Source::Claude,
            Some("s".into()),
            EventBody::UserPrompt { text: text.into() },
        )
    }

    #[test]
    fn append_then_read_incrementally() {
        let dir = tmp_dir("append");
        let log = dir.join("p/events.jsonl");
        append(&log, &ev("one")).unwrap();
        let first = read_from(&log, 0).unwrap();
        assert_eq!(first.events.len(), 1);

        append(&log, &ev("two")).unwrap();
        // Simulate a writer caught mid-line plus a line from a newer version.
        let mut f = open_append(&log).unwrap();
        f.write_all(b"{\"v\":9,\"kind\":\"future\"}\n{\"v\":1,\"ts\":")
            .unwrap();

        let second = read_from(&log, first.next_offset).unwrap();
        assert_eq!(second.events, vec![ev("two")]);
        assert_eq!(second.skipped, 1);
        let rest = read_from(&log, second.next_offset).unwrap();
        assert!(rest.events.is_empty());
        assert_eq!(rest.next_offset, second.next_offset);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn files_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tmp_dir("perm");
        let log = dir.join("p/events.jsonl");
        append(&log, &ev("x")).unwrap();
        assert_eq!(
            fs::metadata(&log).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(log.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn concurrent_appends_do_not_interleave() {
        let dir = tmp_dir("concurrent");
        let log = dir.join("p/events.jsonl");
        let text = "ж".repeat(150);
        std::thread::scope(|s| {
            for _ in 0..8 {
                s.spawn(|| {
                    for _ in 0..50 {
                        append(&log, &ev(&text)).unwrap();
                    }
                });
            }
        });
        let chunk = read_from(&log, 0).unwrap();
        assert_eq!(chunk.events.len(), 400);
        assert_eq!(chunk.skipped, 0);
        fs::remove_dir_all(&dir).unwrap();
    }

    fn ev_at(ts: &str, session: &str, text: &str) -> Event {
        Event::new(
            ts.parse().unwrap(),
            Source::Claude,
            Some(session.into()),
            EventBody::UserPrompt { text: text.into() },
        )
    }

    #[test]
    fn rotate_moves_old_sessions_to_monthly_archives() {
        let dir = tmp_dir("rotate");
        let log = dir.join("p/events.jsonl");
        append(&log, &ev_at("2026-08-30T10:00:00Z", "aug", "a")).unwrap();
        append(&log, &ev_at("2026-09-02T10:00:00Z", "sep", "b")).unwrap();
        let mut f = open_append(&log).unwrap();
        f.write_all(b"{\"v\":9,\"kind\":\"future\"}\n").unwrap();
        append(&log, &ev_at("2026-10-30T10:00:00Z", "oct", "c")).unwrap();

        let cutoff = "2026-10-17T00:00:00Z".parse().unwrap();
        let r = rotate(&log, cutoff).unwrap();
        assert_eq!(
            r,
            Rotated {
                archived: 2,
                kept: 2
            }
        );
        let live = fs::read_to_string(&log).unwrap();
        assert!(
            live.starts_with("{\"v\":9,\"kind\":\"future\"}\n"),
            "{live}"
        );
        assert_eq!(read_from(&log, 0).unwrap().events.len(), 1);
        let aug = read_from(&dir.join("p/archive/2026-08.jsonl"), 0).unwrap();
        let sep = read_from(&dir.join("p/archive/2026-09.jsonl"), 0).unwrap();
        assert_eq!(aug.events, vec![ev_at("2026-08-30T10:00:00Z", "aug", "a")]);
        assert_eq!(sep.events.len(), 1);
        // Idempotent; a missing log is fine.
        assert_eq!(rotate(&log, cutoff).unwrap().archived, 0);
        assert_eq!(
            rotate(&dir.join("none.jsonl"), cutoff).unwrap(),
            Rotated::default()
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn appends_during_rotation_are_not_lost() {
        let dir = tmp_dir("rotate-concurrent");
        let log = dir.join("p/events.jsonl");
        for i in 0..200 {
            append(
                &log,
                &ev_at("2026-08-01T10:00:00Z", &format!("old{i}"), "x"),
            )
            .unwrap();
        }
        let cutoff: jiff::Timestamp = "2026-10-17T00:00:00Z".parse().unwrap();
        let mut archived = 0;
        std::thread::scope(|s| {
            for t in 0..4 {
                let log = &log;
                s.spawn(move || {
                    for i in 0..50 {
                        append(
                            log,
                            &ev_at("2026-10-30T10:00:00Z", &format!("n{t}-{i}"), "y"),
                        )
                        .unwrap();
                    }
                });
            }
            for _ in 0..10 {
                archived += rotate(&log, cutoff).unwrap().archived;
            }
        });
        let live = read_from(&log, 0).unwrap();
        assert_eq!(live.events.len(), 200);
        assert_eq!(live.skipped, 0);
        assert_eq!(archived, 200);
        let arch = read_from(&dir.join("p/archive/2026-08.jsonl"), 0).unwrap();
        assert_eq!(arch.events.len(), 200);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn forget_moves_the_log_and_unregisters() {
        let dir = tmp_dir("forget");
        let paths = Paths::new(&dir);
        let keep = ProjectMeta::for_root(Path::new("/home/dev/DeepVault"));
        let gone = ProjectMeta::for_root(Path::new("/tmp/alpha"));
        for m in [&keep, &gone] {
            ensure_registered(&paths, m).unwrap();
            append(&paths.events(&m.id), &ev("x")).unwrap();
        }
        let moved = forget(&paths, &gone.id).unwrap().unwrap();
        assert!(moved.join("events.jsonl").exists());
        assert!(!paths.events(&gone.id).exists());
        let reg = Registry::load(&paths.registry()).unwrap();
        assert_eq!(reg.projects, vec![keep.clone()]);
        assert_eq!(load_all(&paths).unwrap().len(), 1);
        // Unknown or already forgotten: nothing to do.
        assert_eq!(forget(&paths, &gone.id).unwrap(), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn registry_is_idempotent() {
        let dir = tmp_dir("registry");
        let paths = Paths::new(&dir);
        let meta = ProjectMeta::for_root(Path::new("/home/dev/DeepVault"));
        ensure_registered(&paths, &meta).unwrap();
        ensure_registered(&paths, &meta).unwrap();
        ensure_registered(
            &paths,
            &ProjectMeta::for_root(Path::new("/home/dev/DeepSnap")),
        )
        .unwrap();
        let reg = Registry::load(&paths.registry()).unwrap();
        assert_eq!(reg.projects.len(), 2);
        fs::remove_dir_all(&dir).unwrap();
    }
}
