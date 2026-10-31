//! Commands an agent (or the user) runs from a shell: `plan`, `step`, `add`,
//! `ask`, `idea`, `done`, `list`, `snippet`. They write the same events as the
//! hooks and the TUI. Output is one short line — the agent reads stdout.

use std::path::Path;

use anyhow::{Context, Result, bail};
use deepdeck_core::event::{CLI_SESSION, PlanItem, note_id};
use deepdeck_core::project::{find_root, has_vcs_marker};
use deepdeck_core::store::{self, Paths};
use deepdeck_core::{
    AgentView, Event, EventBody, NoteType, ProjectMeta, ProjectView, Source, TaskStatus,
    fold_project,
};
use jiff::Timestamp;
use serde_json::json;

/// `ddeck step <n> <state>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum StepState {
    Todo,
    Doing,
    Done,
}

impl StepState {
    fn word(self) -> &'static str {
        match self {
            StepState::Todo => "todo",
            StepState::Doing => "doing",
            StepState::Done => "done",
        }
    }

    fn status(self) -> TaskStatus {
        match self {
            StepState::Todo => TaskStatus::Pending,
            StepState::Doing => TaskStatus::InProgress,
            StepState::Done => TaskStatus::Completed,
        }
    }
}

/// Where and as whom a command writes.
pub struct Ctx {
    paths: Paths,
    meta: ProjectMeta,
    session: String,
    src: Source,
}

impl Ctx {
    /// Project from `--project` or the current directory (repository root, as in
    /// the hook); session from `CLAUDE_CODE_SESSION_ID`, so the plan lands on the
    /// agent that posted it.
    pub fn new(paths: Paths, project: Option<&Path>) -> Result<Self> {
        let dir = match project {
            Some(p) => p.to_path_buf(),
            None => std::env::current_dir().context("no current directory")?,
        };
        let dir = dir
            .canonicalize()
            .with_context(|| format!("no such directory: {}", dir.display()))?;
        let session = std::env::var("CLAUDE_CODE_SESSION_ID")
            .ok()
            .filter(|s| !s.is_empty());
        let src = if session.is_some() || std::env::var_os("CLAUDECODE").is_some() {
            Source::Agent
        } else {
            Source::User
        };
        Ok(Ctx {
            paths,
            meta: ProjectMeta::for_root(&find_root(&dir, has_vcs_marker)),
            session: session.unwrap_or_else(|| CLI_SESSION.to_owned()),
            src,
        })
    }

    fn write(&self, body: EventBody) -> Result<()> {
        let event = Event::new(Timestamp::now(), self.src, Some(self.session.clone()), body);
        store::append(&self.paths.events(&self.meta.id), &event)?;
        store::ensure_registered(&self.paths, &self.meta)?;
        Ok(())
    }

    fn view(&self) -> Result<ProjectView> {
        let events = store::read_from(&self.paths.events(&self.meta.id), 0)?.events;
        Ok(fold_project(self.meta.clone(), &events))
    }
}

/// The agent whose plan a command acts on: this session's, else the most
/// recently active one that has a plan.
fn plan_owner<'a>(view: &'a ProjectView, session: &str) -> Option<&'a AgentView> {
    view.agents
        .iter()
        .find(|a| a.session == session && !a.tasks.is_empty())
        .or_else(|| view.agents.iter().find(|a| !a.tasks.is_empty()))
}

pub fn plan(ctx: &Ctx, steps: &[String]) -> Result<String> {
    let tasks: Vec<PlanItem> = steps
        .iter()
        .map(|s| deepdeck_core::hooks::collapse_ws(s))
        .filter(|s| !s.is_empty())
        .map(|subject| PlanItem {
            subject: deepdeck_core::hooks::truncate_chars(
                &subject,
                deepdeck_core::hooks::RECAP_MAX_CHARS,
            ),
            status: TaskStatus::Pending,
        })
        .collect();
    if tasks.is_empty() {
        bail!("a plan needs at least one step: ddeck plan \"step 1\" \"step 2\"");
    }
    let n = tasks.len();
    ctx.write(EventBody::PlanSnapshot { tasks })?;
    Ok(format!(
        "plan: {n} step{} · {}",
        if n == 1 { "" } else { "s" },
        ctx.meta.name
    ))
}

pub fn step(ctx: &Ctx, n: usize, state: StepState) -> Result<String> {
    let view = ctx.view()?;
    let Some(agent) = plan_owner(&view, &ctx.session) else {
        bail!("no plan yet — post one first: ddeck plan \"step 1\" \"step 2\"");
    };
    let total = agent.tasks.len();
    let Some(task) = n.checked_sub(1).and_then(|i| agent.tasks.get(i)) else {
        bail!(
            "no step {n}: the plan has {total} step{}",
            if total == 1 { "" } else { "s" }
        );
    };
    let done_before = agent.progress().0;
    let was_done = task.status == TaskStatus::Completed;
    let event = Event::new(
        Timestamp::now(),
        ctx.src,
        Some(agent.session.clone()),
        EventBody::TaskUpsert {
            task_id: task.id.clone(),
            subject: None,
            status: Some(state.status()),
        },
    );
    store::append(&ctx.paths.events(&ctx.meta.id), &event)?;
    let done = match (was_done, state) {
        (false, StepState::Done) => done_before + 1,
        (true, StepState::Todo | StepState::Doing) => done_before - 1,
        _ => done_before,
    };
    Ok(format!("step {n}: {} ({done}/{total})", state.word()))
}

pub fn roadmap_set(ctx: &Ctx, steps: &[String]) -> Result<String> {
    let steps = clean_steps(steps);
    if steps.is_empty() {
        bail!("a roadmap needs at least one step: ddeck roadmap set \"step 1\" \"step 2\"");
    }
    let n = steps.len();
    ctx.write(EventBody::RoadmapSet { steps })?;
    Ok(format!(
        "roadmap: {n} step{} · {}",
        if n == 1 { "" } else { "s" },
        ctx.meta.name
    ))
}

pub fn roadmap_step(ctx: &Ctx, n: usize, state: StepState) -> Result<String> {
    let view = ctx.view()?;
    let total = view.roadmap.len();
    if total == 0 {
        bail!("no roadmap yet — post one first: ddeck roadmap set \"step 1\" \"step 2\"");
    }
    if n == 0 || n > total {
        bail!(
            "no roadmap step {n}: the roadmap has {total} step{}",
            if total == 1 { "" } else { "s" }
        );
    }
    ctx.write(EventBody::RoadmapStep {
        n,
        status: state.status(),
    })?;
    let done = ctx
        .view()?
        .roadmap
        .iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .count();
    Ok(format!(
        "roadmap step {n}: {} ({done}/{total})",
        state.word()
    ))
}

fn clean_steps(steps: &[String]) -> Vec<PlanItem> {
    steps
        .iter()
        .map(|s| deepdeck_core::hooks::collapse_ws(s))
        .filter(|s| !s.is_empty())
        .map(|subject| PlanItem {
            subject: deepdeck_core::hooks::truncate_chars(
                &subject,
                deepdeck_core::hooks::RECAP_MAX_CHARS,
            ),
            status: TaskStatus::Pending,
        })
        .collect()
}

/// Hide a prompt the user confirmed was meant for another session. With a
/// fragment: the latest prompt of this session containing it (case-insensitive);
/// without: the one before the latest — the latest is the user's "wrong
/// session" reply that made the agent run this.
pub fn retract(ctx: &Ctx, fragment: Option<&str>, to: Option<&str>) -> Result<String> {
    let view = ctx.view()?;
    let prompts = view
        .agents
        .iter()
        .find(|a| a.session == ctx.session)
        .map(|a| a.recent_prompts.as_slice())
        .unwrap_or_default();
    let target = match fragment.map(str::trim).filter(|f| !f.is_empty()) {
        Some(f) => {
            let f = f.to_lowercase();
            prompts
                .iter()
                .rev()
                .find(|(_, p)| p.to_lowercase().contains(&f))
        }
        None => prompts.iter().rev().nth(1).or(prompts.last()),
    };
    let Some((at, text)) = target else {
        match fragment {
            Some(f) => bail!("no recent prompt in this session contains \"{f}\""),
            None => bail!("no prompts in this session to retract"),
        }
    };
    ctx.write(EventBody::PromptRetract {
        at: *at,
        to: to.map(str::to_owned),
    })?;
    let excerpt = deepdeck_core::hooks::truncate_chars(text, 48);
    Ok(match to {
        Some(p) => format!("retracted \"{excerpt}\" (meant for {p})"),
        None => format!("retracted \"{excerpt}\""),
    })
}

pub fn summary_write(ctx: &Ctx, text: &str) -> Result<String> {
    let text = deepdeck_core::hooks::collapse_ws(text);
    if text.is_empty() {
        bail!("empty summary");
    }
    let text = deepdeck_core::hooks::truncate_chars(&text, SUMMARY_MAX_CHARS);
    ctx.write(EventBody::SessionSummary { text })?;
    Ok(format!("summary saved · {}", ctx.meta.name))
}

/// Latest summaries of the project's sessions, newest first; a session without
/// one gets a digest of its events instead.
pub fn summary_show(ctx: &Ctx, limit: usize) -> Result<String> {
    let view = ctx.view()?;
    let mut out = format!("{}\n", view.meta.name);
    let mut shown = 0;
    for a in view.agents.iter().take(limit) {
        let text = match &a.summary {
            Some(s) => s.clone(),
            None => {
                let d = a.digest();
                if d.is_empty() {
                    continue;
                }
                format!("(no summary) {d}")
            }
        };
        let when = a
            .last_activity
            .to_zoned(jiff::tz::TimeZone::system())
            .strftime("%m-%d %H:%M");
        let me = if a.session == ctx.session {
            " (this session)"
        } else {
            ""
        };
        out.push_str(&format!("{when}{me}  {text}\n"));
        shown += 1;
    }
    if shown == 0 {
        out.push_str("no sessions yet\n");
    }
    Ok(out)
}

/// Max characters of a session summary.
const SUMMARY_MAX_CHARS: usize = 600;

pub fn add_note(ctx: &Ctx, note_type: NoteType, text: &str) -> Result<String> {
    let text = deepdeck_core::hooks::collapse_ws(text);
    if text.is_empty() {
        bail!("empty text");
    }
    let id = note_id(Timestamp::now());
    ctx.write(EventBody::NoteAdd {
        id: id.clone(),
        note_type,
        text,
    })?;
    let kind = match note_type {
        NoteType::Task => "task",
        NoteType::Question => "question",
        NoteType::Idea => "idea",
    };
    Ok(format!("{kind} {id} · {}", ctx.meta.name))
}

pub fn done(ctx: &Ctx, id: &str) -> Result<String> {
    let view = ctx.view()?;
    if !view.notes.iter().any(|n| n.id == id) {
        bail!("no open item {id} in {} (see: ddeck list)", ctx.meta.name);
    }
    ctx.write(EventBody::NoteDone { id: id.to_owned() })?;
    Ok(format!("done {id}"))
}

pub fn list(ctx: &Ctx, as_json: bool) -> Result<String> {
    Ok(render_list(&ctx.view()?, &ctx.session, as_json))
}

pub fn render_list(view: &ProjectView, session: &str, as_json: bool) -> String {
    let plan = plan_owner(view, session)
        .map(|a| a.tasks.as_slice())
        .unwrap_or_default();
    if as_json {
        let status = |s: TaskStatus| match s {
            TaskStatus::Pending => "todo",
            TaskStatus::InProgress => "doing",
            TaskStatus::Completed => "done",
            TaskStatus::Deleted | TaskStatus::Unknown => "unknown",
        };
        let v = json!({
            "project": view.meta.name,
            "root": view.meta.root,
            "last_summary": view.last_summary.as_ref().map(|s| json!({
                "session": s.session, "at": s.at.to_string(), "text": s.text
            })),
            "roadmap": view.roadmap.iter().enumerate().map(|(i, t)| json!({
                "step": i + 1, "subject": t.subject, "status": status(t.status)
            })).collect::<Vec<_>>(),
            "plan": plan.iter().enumerate().map(|(i, t)| json!({
                "step": i + 1, "subject": t.subject, "status": status(t.status)
            })).collect::<Vec<_>>(),
            "notes": view.notes.iter().map(|n| json!({
                "id": n.id,
                "type": match n.note_type { NoteType::Task => "task", NoteType::Question => "question", NoteType::Idea => "idea" },
                "text": n.text
            })).collect::<Vec<_>>(),
        });
        return format!(
            "{}\n",
            serde_json::to_string_pretty(&v).expect("a Value always serializes")
        );
    }
    let mut out = format!("{}\n", view.meta.name);
    if let Some(s) = view.last_summary.as_ref().filter(|s| s.session != session) {
        out.push_str(&format!("last session: {}\n", s.text));
    }
    if !view.roadmap.is_empty() {
        let done = view
            .roadmap
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        out.push_str(&format!("roadmap {done}/{}\n", view.roadmap.len()));
        for (i, t) in view.roadmap.iter().enumerate() {
            let m = match t.status {
                TaskStatus::Completed => "✓",
                TaskStatus::InProgress => "◐",
                _ => "○",
            };
            out.push_str(&format!("  {} {m} {}\n", i + 1, t.subject));
        }
    }
    if !plan.is_empty() {
        let done = plan
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        out.push_str(&format!("plan {done}/{}\n", plan.len()));
        for (i, t) in plan.iter().enumerate() {
            let m = match t.status {
                TaskStatus::Completed => "✓",
                TaskStatus::InProgress => "◐",
                _ => "○",
            };
            out.push_str(&format!("  {} {m} {}\n", i + 1, t.subject));
        }
    }
    for n in &view.notes {
        let m = match n.note_type {
            NoteType::Question => "?",
            NoteType::Idea => "◆",
            NoteType::Task => "○",
        };
        out.push_str(&format!("{m} {}  {}\n", n.id, n.text));
    }
    if plan.is_empty()
        && view.notes.is_empty()
        && view.roadmap.is_empty()
        && view.last_summary.is_none()
    {
        out.push_str("nothing open\n");
    }
    out
}

/// Find a registered project by id, name (case-insensitive) or path.
/// An ambiguous name is an error, never a guess.
pub fn resolve<'a>(reg: &'a store::Registry, query: &str) -> Result<&'a ProjectMeta> {
    if let Some(p) = reg.projects.iter().find(|p| p.id.as_str() == query) {
        return Ok(p);
    }
    let by_name: Vec<_> = reg
        .projects
        .iter()
        .filter(|p| p.name.eq_ignore_ascii_case(query))
        .collect();
    match by_name.as_slice() {
        [one] => return Ok(one),
        [] => {}
        many => bail!(
            "{query} matches {} projects: {} — use the id",
            many.len(),
            many.iter()
                .map(|p| format!("{} ({})", p.id, p.root.display()))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
    if let Ok(dir) = Path::new(query).canonicalize() {
        let id = ProjectMeta::for_root(&find_root(&dir, has_vcs_marker)).id;
        if let Some(p) = reg.projects.iter().find(|p| p.id == id) {
            return Ok(p);
        }
    }
    bail!("no project {query} on the board (see: ddeck status --all)")
}

pub fn forget(paths: &Paths, queries: &[String], dry_run: bool) -> Result<String> {
    let reg = store::Registry::load(&paths.registry())?;
    // Resolve everything first: a typo must not leave a half-done job.
    let metas = queries
        .iter()
        .map(|q| resolve(&reg, q).cloned())
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::new();
    for meta in metas {
        if dry_run {
            out.push(format!(
                "would forget {} ({})",
                meta.name,
                meta.root.display()
            ));
            continue;
        }
        match store::forget(paths, &meta.id)? {
            Some(dest) => out.push(format!(
                "forgot {} (log moved to {})",
                meta.name,
                dest.display()
            )),
            None => out.push(format!("forgot {}", meta.name)),
        }
    }
    Ok(out.join("\n"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ddeck-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("repo/.git")).unwrap();
        d
    }

    fn ctx(dir: &Path, session: &str) -> Ctx {
        Ctx {
            paths: Paths::new(dir.join("home")),
            meta: ProjectMeta::for_root(&dir.join("repo")),
            session: session.into(),
            src: Source::Agent,
        }
    }

    #[test]
    fn plan_step_list_round_trip() {
        let d = tmp("plan");
        let c = ctx(&d, "s1");
        assert_eq!(
            plan(
                &c,
                &["Parser".into(), "  Import  files ".into(), "UI".into()]
            )
            .unwrap(),
            "plan: 3 steps · repo"
        );
        assert_eq!(step(&c, 1, StepState::Done).unwrap(), "step 1: done (1/3)");
        assert_eq!(
            step(&c, 2, StepState::Doing).unwrap(),
            "step 2: doing (1/3)"
        );
        assert_eq!(step(&c, 1, StepState::Done).unwrap(), "step 1: done (1/3)");
        assert!(
            step(&c, 4, StepState::Done)
                .unwrap_err()
                .to_string()
                .contains("3 steps")
        );
        assert!(plan(&c, &["  ".into()]).is_err());

        let id = add_note(&c, NoteType::Question, "CSV  too?").unwrap();
        let id = id.split_whitespace().nth(1).unwrap().to_owned();
        let text = list(&c, false).unwrap();
        assert_eq!(
            text,
            format!(
                "repo\nplan 1/3\n  1 ✓ Parser\n  2 ◐ Import files\n  3 ○ UI\n? {id}  CSV too?\n"
            )
        );
        let v: serde_json::Value = serde_json::from_str(&list(&c, true).unwrap()).unwrap();
        assert_eq!(v["plan"][1]["status"], "doing");
        assert_eq!(v["notes"][0]["type"], "question");

        assert_eq!(done(&c, &id).unwrap(), format!("done {id}"));
        assert!(done(&c, &id).is_err());

        // Another session without a plan still sees (and moves) the existing one.
        let other = ctx(&d, "s2");
        assert_eq!(
            step(&other, 3, StepState::Done).unwrap(),
            "step 3: done (2/3)"
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn resolve_by_id_name_or_path() {
        let d = tmp("resolve");
        let paths = Paths::new(d.join("home"));
        let repo = ProjectMeta::for_root(&d.join("repo").canonicalize().unwrap());
        let twin_a = ProjectMeta::for_root(Path::new("/a/twin"));
        let twin_b = ProjectMeta::for_root(Path::new("/b/twin"));
        for m in [&repo, &twin_a, &twin_b] {
            store::ensure_registered(&paths, m).unwrap();
        }
        let reg = store::Registry::load(&paths.registry()).unwrap();
        assert_eq!(resolve(&reg, "REPO").unwrap().id, repo.id);
        assert_eq!(resolve(&reg, twin_a.id.as_str()).unwrap().id, twin_a.id);
        assert_eq!(
            resolve(&reg, d.join("repo").to_str().unwrap()).unwrap().id,
            repo.id
        );
        assert!(
            resolve(&reg, "twin")
                .unwrap_err()
                .to_string()
                .contains("2 projects")
        );
        assert!(resolve(&reg, "nope").is_err());

        // One bad name: nothing is forgotten.
        assert!(forget(&paths, &["repo".into(), "nope".into()], false).is_err());
        assert_eq!(
            store::Registry::load(&paths.registry())
                .unwrap()
                .projects
                .len(),
            3
        );
        assert_eq!(
            forget(&paths, &["repo".into()], false).unwrap(),
            "forgot repo"
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn retract_picks_the_misdirected_prompt() {
        let d = tmp("retract");
        let c = ctx(&d, "s1");
        let prompt = |text: &str| {
            let e = Event::new(
                Timestamp::now(),
                Source::Claude,
                Some("s1".into()),
                EventBody::UserPrompt { text: text.into() },
            );
            store::append(&c.paths.events(&c.meta.id), &e).unwrap();
        };
        assert!(retract(&c, None, None).is_err());
        prompt("what next?");
        prompt("chips No l1O0 — tooltip?");
        prompt("oops, that was for another session");
        // No fragment: the one before the "oops".
        assert_eq!(
            retract(&c, None, None).unwrap(),
            "retracted \"chips No l1O0 — tooltip?\""
        );
        assert_eq!(
            c.view().unwrap().agents[0].last_prompt.as_deref(),
            Some("oops, that was for another session")
        );
        assert!(
            retract(&c, Some("bitwarden"), None)
                .unwrap_err()
                .to_string()
                .contains("bitwarden")
        );
        assert_eq!(
            retract(&c, Some("WHAT next"), Some("DeepVault")).unwrap(),
            "retracted \"what next?\" (meant for DeepVault)"
        );
        // Another session's prompts are out of reach.
        assert!(retract(&ctx(&d, "s2"), None, None).is_err());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn summary_reaches_the_next_session() {
        let d = tmp("summary");
        let first = ctx(&d, "s1");
        plan(&first, &["Parser".into(), "UI".into()]).unwrap();
        assert_eq!(
            summary_write(&first, "Parser done; UI next, keep the old import path.").unwrap(),
            "summary saved · repo"
        );
        // The session that wrote it does not see it as "last session".
        assert!(!list(&first, false).unwrap().contains("last session"));
        let next = ctx(&d, "s2");
        let text = list(&next, false).unwrap();
        assert!(
            text.starts_with(
                "repo\nlast session: Parser done; UI next, keep the old import path.\n"
            ),
            "{text}"
        );
        let v: serde_json::Value = serde_json::from_str(&list(&next, true).unwrap()).unwrap();
        assert_eq!(v["last_summary"]["session"], "s1");
        let shown = summary_show(&next, 5).unwrap();
        assert!(shown.contains("Parser done"), "{shown}");
        assert!(summary_write(&next, "   ").is_err());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn roadmap_set_step_list() {
        let d = tmp("roadmap");
        let c = ctx(&d, "s1");
        assert!(
            roadmap_step(&c, 1, StepState::Done)
                .unwrap_err()
                .to_string()
                .contains("roadmap set")
        );
        assert_eq!(
            roadmap_set(
                &c,
                &["Restart agents".into(), "Session 004: release".into()]
            )
            .unwrap(),
            "roadmap: 2 steps · repo"
        );
        // Another session (after a restart) moves the same roadmap.
        let later = ctx(&d, "s2");
        assert_eq!(
            roadmap_step(&later, 1, StepState::Done).unwrap(),
            "roadmap step 1: done (1/2)"
        );
        assert!(roadmap_step(&later, 3, StepState::Done).is_err());
        assert_eq!(
            list(&later, false).unwrap(),
            "repo\nroadmap 1/2\n  1 ✓ Restart agents\n  2 ○ Session 004: release\n"
        );
        let v: serde_json::Value = serde_json::from_str(&list(&later, true).unwrap()).unwrap();
        assert_eq!(v["roadmap"][0]["status"], "done");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn step_without_plan_explains() {
        let d = tmp("noplan");
        let c = ctx(&d, "s1");
        assert!(
            step(&c, 1, StepState::Done)
                .unwrap_err()
                .to_string()
                .contains("ddeck plan")
        );
        assert_eq!(list(&c, false).unwrap(), "repo\nnothing open\n");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
