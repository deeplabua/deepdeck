//! `ddeck` — DeepDeck command line.

mod cli;
mod clipboard;
mod doctor;
mod herdr;
mod hook;
mod install;
mod setup;
mod status;
mod tui;
mod update;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use deepdeck_core::store::{self, Paths};
use deepdeck_core::{NoteType, fold};
use jiff::Timestamp;

#[derive(Parser)]
#[command(
    name = "ddeck",
    version,
    about = "A terminal task board for people running several AI agents at once",
    after_help = "Without a command, opens the board (TUI).\nGet started: ddeck setup --dry-run, then ddeck setup."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Install hooks and teach agents to keep the board current (global CLAUDE.md).
    Setup {
        /// Show the changes, write nothing.
        #[arg(long)]
        dry_run: bool,
        /// Only add the instructions to this repository's CLAUDE.md (hooks untouched).
        #[arg(long, value_name = "PATH", num_args = 0..=1, default_missing_value = ".")]
        project: Option<PathBuf>,
    },
    /// Remove DeepDeck hooks and the global instructions; nothing else.
    Uninstall {
        #[arg(long)]
        dry_run: bool,
        /// Only remove the instructions from this repository's CLAUDE.md.
        #[arg(long, value_name = "PATH", num_args = 0..=1, default_missing_value = ".")]
        project: Option<PathBuf>,
        #[arg(long, hide = true)]
        settings: Option<PathBuf>,
    },
    /// Post the agent's plan, replacing the previous one.
    Plan {
        /// Steps in order.
        #[arg(required = true)]
        steps: Vec<String>,
        #[arg(long, value_name = "PATH")]
        project: Option<PathBuf>,
    },
    /// The project roadmap: what comes after this request. Without a subcommand, prints it.
    Roadmap {
        #[command(subcommand)]
        action: Option<RoadmapAction>,
        #[arg(long, value_name = "PATH", global = true)]
        project: Option<PathBuf>,
    },
    /// Saved project commands (runbook). Without a subcommand, lists them.
    Cmd {
        #[command(subcommand)]
        action: Option<CmdAction>,
        #[arg(long, value_name = "PATH", global = true)]
        project: Option<PathBuf>,
    },
    /// Mark progress on a plan step (numbered from 1).
    Step {
        n: usize,
        #[arg(value_enum)]
        state: cli::StepState,
        #[arg(long, value_name = "PATH")]
        project: Option<PathBuf>,
    },
    /// Hide a message the user confirmed was meant for another session.
    Retract {
        /// A few words from that message (default: the one before the latest).
        fragment: Option<String>,
        /// The project it was meant for.
        #[arg(long, value_name = "PROJECT")]
        to: Option<String>,
        #[arg(long, value_name = "PATH")]
        project: Option<PathBuf>,
    },
    /// Save this session's summary (what was done, decided, left open); without text, show recent ones.
    Summary {
        text: Option<String>,
        #[arg(long, value_name = "PATH")]
        project: Option<PathBuf>,
    },
    /// Add a task to the project's board.
    Add {
        text: String,
        #[arg(long, value_name = "PATH")]
        project: Option<PathBuf>,
    },
    /// Record an open question for the user.
    Ask {
        text: String,
        #[arg(long, value_name = "PATH")]
        project: Option<PathBuf>,
    },
    /// Record an idea for later.
    Idea {
        text: String,
        #[arg(long, value_name = "PATH")]
        project: Option<PathBuf>,
    },
    /// Close a task, question or idea by id.
    Done {
        id: String,
        #[arg(long, value_name = "PATH")]
        project: Option<PathBuf>,
    },
    /// The project's plan and open items.
    List {
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        project: Option<PathBuf>,
    },
    /// Print the CLAUDE.md block that `setup` installs.
    Snippet,
    /// Remove projects from the board (by name, id or path); their logs are kept in ~/.deepdeck/forgotten/.
    Forget {
        #[arg(required = true)]
        projects: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Move old sessions and closed notes to ~/.deepdeck/projects/<id>/archive/ (the board does this on start).
    Archive {
        /// Events older than this many days.
        #[arg(long, default_value_t = deepdeck_core::archive::ARCHIVE_AFTER_DAYS)]
        days: i64,
    },
    /// Plain-text board.
    Status {
        /// Only projects active within this many hours.
        #[arg(long, default_value_t = 12)]
        hours: i64,
        /// All projects, however old.
        #[arg(long, conflicts_with = "hours")]
        all: bool,
        /// One number for a status bar: agents waiting for you + open questions.
        #[arg(long)]
        short: bool,
    },
    /// Only add DeepDeck hooks to Claude Code settings (`setup` does this and more).
    Install {
        #[arg(long)]
        dry_run: bool,
        /// Settings file (default: ~/.claude/settings.json).
        #[arg(long)]
        settings: Option<PathBuf>,
    },
    /// Check hooks, instructions, data directory and recent events.
    Doctor {
        #[arg(long)]
        settings: Option<PathBuf>,
    },
    /// Called by Claude Code hooks: reads the payload on stdin. Always exits 0, prints nothing.
    #[command(hide = true)]
    Hook {
        #[arg(long)]
        dump: bool,
    },
}

#[derive(Subcommand)]
enum CmdAction {
    /// Save a command under a name (replaces one with the same name). Refer to secrets, never paste them.
    Add {
        name: String,
        command: String,
        /// Save even if it looks like it contains a secret value.
        #[arg(long)]
        force: bool,
    },
    /// Run a saved command in the project root (name or unique prefix).
    Run { name: String },
    /// Print a saved command, e.g. to copy it.
    Show { name: String },
    /// Remove a saved command.
    Rm { name: String },
}

#[derive(Subcommand)]
enum RoadmapAction {
    /// Replace the roadmap with these steps.
    Set {
        #[arg(required = true)]
        steps: Vec<String>,
    },
    /// Mark progress on a roadmap step (numbered from 1).
    Step {
        n: usize,
        #[arg(value_enum)]
        state: cli::StepState,
    },
}

fn main() -> ExitCode {
    // The hook path bypasses clap: even a bad argument must not print or fail.
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("hook") {
        hook::run(args.any(|a| a == "--dump"));
    }

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        // `-V` and `-h`: a person asking — a newer version is worth a line after.
        Err(e)
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayVersion | clap::error::ErrorKind::DisplayHelp
            ) =>
        {
            let _ = e.print();
            print!("{}", update::for_terminal());
            return ExitCode::SUCCESS;
        }
        Err(e) => e.exit(),
    };
    let res = match cli.command {
        Some(command) => run(command),
        None => paths()
            .and_then(|p| Ok((p, install::default_settings_path()?)))
            .and_then(|(p, s)| tui::run(p, &s))
            .map(|()| ExitCode::SUCCESS),
    };
    match res {
        Ok(code) => code,
        Err(e) => {
            eprintln!("ddeck: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn paths() -> Result<Paths> {
    Paths::from_env().context("neither DEEPDECK_HOME nor HOME is set")
}

fn settings_path(arg: Option<PathBuf>) -> Result<PathBuf> {
    arg.map_or_else(install::default_settings_path, Ok)
}

fn say(line: String) -> Result<ExitCode> {
    println!("{line}");
    Ok(ExitCode::SUCCESS)
}

fn run(command: Command) -> Result<ExitCode> {
    match command {
        Command::Hook { dump } => hook::run(dump),
        Command::Setup { dry_run, project } => {
            if let Some(dir) = project {
                let path = setup::project_claude_md(&dir)?;
                let backups = paths()?.root.join("backups");
                if !setup::apply_block(&path, dry_run, Some(&backups))? {
                    println!("DeepDeck instructions are up to date in {}", path.display());
                }
                return Ok(ExitCode::SUCCESS);
            }
            install_hooks(&install::default_settings_path()?, dry_run)?;
            let path = setup::global_claude_md()?;
            if !setup::apply_block(&path, dry_run, None)? {
                println!("DeepDeck instructions are up to date in {}", path.display());
            }
            if !dry_run {
                println!(
                    "\nDone. New Claude Code sessions will post plans to the board; open it with `ddeck`."
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Uninstall {
            dry_run,
            project,
            settings,
        } => {
            if let Some(dir) = project {
                let path = setup::project_claude_md(&dir)?;
                let backups = paths()?.root.join("backups");
                if !setup::unapply_block(&path, dry_run, Some(&backups))? {
                    println!("No DeepDeck instructions in {}", path.display());
                }
                return Ok(ExitCode::SUCCESS);
            }
            uninstall_hooks(&settings_path(settings)?, dry_run)?;
            let path = setup::global_claude_md()?;
            if !setup::unapply_block(&path, dry_run, None)? {
                println!("No DeepDeck instructions in {}", path.display());
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Plan { steps, project } => say(cli::plan(&ctx(project)?, &steps)?),
        Command::Roadmap { action, project } => {
            let ctx = ctx(project)?;
            match action {
                Some(RoadmapAction::Set { steps }) => say(cli::roadmap_set(&ctx, &steps)?),
                Some(RoadmapAction::Step { n, state }) => say(cli::roadmap_step(&ctx, n, state)?),
                None => {
                    print!("{}", cli::list(&ctx, false)?);
                    Ok(ExitCode::SUCCESS)
                }
            }
        }
        Command::Cmd { action, project } => {
            let ctx = ctx(project)?;
            match action {
                Some(CmdAction::Add {
                    name,
                    command,
                    force,
                }) => say(cli::cmd_add(&ctx, &name, &command, force)?),
                Some(CmdAction::Rm { name }) => say(cli::cmd_rm(&ctx, &name)?),
                Some(CmdAction::Show { name }) => say(cli::cmd_get(&ctx, &name)?.command),
                Some(CmdAction::Run { name }) => {
                    let code = cli::cmd_run(&ctx, &name)?;
                    Ok(ExitCode::from(u8::try_from(code).unwrap_or(1)))
                }
                None => {
                    print!("{}", cli::cmd_list(&ctx, &status::Style::for_stdout())?);
                    print!("{}", update::for_terminal());
                    Ok(ExitCode::SUCCESS)
                }
            }
        }
        Command::Step { n, state, project } => say(cli::step(&ctx(project)?, n, state)?),
        Command::Retract {
            fragment,
            to,
            project,
        } => say(cli::retract(
            &ctx(project)?,
            fragment.as_deref(),
            to.as_deref(),
        )?),
        Command::Summary { text, project } => {
            let ctx = ctx(project)?;
            match text {
                Some(t) => say(cli::summary_write(&ctx, &t)?),
                None => {
                    print!("{}", cli::summary_show(&ctx, 5)?);
                    Ok(ExitCode::SUCCESS)
                }
            }
        }
        Command::Add { text, project } => {
            say(cli::add_note(&ctx(project)?, NoteType::Task, &text)?)
        }
        Command::Ask { text, project } => {
            say(cli::add_note(&ctx(project)?, NoteType::Question, &text)?)
        }
        Command::Idea { text, project } => {
            say(cli::add_note(&ctx(project)?, NoteType::Idea, &text)?)
        }
        Command::Done { id, project } => say(cli::done(&ctx(project)?, &id)?),
        Command::List { json, project } => {
            print!("{}", cli::list(&ctx(project)?, json)?);
            if !json {
                print!("{}", update::for_terminal());
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Snippet => say(setup::block()),
        Command::Forget { projects, dry_run } => say(cli::forget(&paths()?, &projects, dry_run)?),
        Command::Archive { days } => {
            let cutoff = Timestamp::now() - jiff::SignedDuration::from_hours(days.max(0) * 24);
            let r = store::rotate_all(&paths()?, cutoff);
            say(format!("archived {} events, {} kept", r.archived, r.kept))
        }
        Command::Status { hours, all, short } => {
            let now = Timestamp::now();
            let board = fold(store::load_all(&paths()?)?);
            let board = status::recent(board, now, (!all).then_some(hours));
            if short {
                println!("{}", status::render_short(&board));
            } else {
                print!(
                    "{}",
                    status::render(&board, now, &status::Style::for_stdout())
                );
                print!("{}", update::for_terminal());
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Install { dry_run, settings } => {
            install_hooks(&settings_path(settings)?, dry_run)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Doctor { settings } => {
            let ok = doctor::run(&paths()?, &settings_path(settings)?)?;
            Ok(if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
    }
}

fn ctx(project: Option<PathBuf>) -> Result<cli::Ctx> {
    cli::Ctx::new(paths()?, project.as_deref())
}

fn install_hooks(path: &Path, dry_run: bool) -> Result<()> {
    let exe = install::hook_exe()?;
    let (old, mut value) = install::load_settings(path)?;
    if !install::merge(&mut value, &install::hook_command(&exe))? {
        println!("DeepDeck hooks are already installed in {}", path.display());
        return Ok(());
    }
    apply(path, &old, &value, dry_run)?;
    if exe.components().any(|c| c.as_os_str() == "target") {
        eprintln!(
            "warning: hooks point to a build in target/ ({}); `cargo install --path crates/ddeck` and re-run for a stable path",
            exe.display()
        );
    }
    Ok(())
}

fn uninstall_hooks(path: &Path, dry_run: bool) -> Result<()> {
    let (old, mut value) = install::load_settings(path)?;
    if !install::remove(&mut value)? {
        println!("No DeepDeck hooks in {}", path.display());
        return Ok(());
    }
    apply(path, &old, &value, dry_run)
}

fn apply(path: &Path, old: &str, value: &serde_json::Value, dry_run: bool) -> Result<()> {
    let new = install::render(value);
    print!("{}", install::diff(path, old, &new));
    if dry_run {
        println!("\n(dry run — nothing written)");
        return Ok(());
    }
    match install::write_with_backup(path, &new)? {
        Some(backup) => println!("\nwrote {} (backup: {})", path.display(), backup.display()),
        None => println!("\nwrote {}", path.display()),
    }
    println!("Restart running Claude Code sessions (or open /hooks) to pick the change up.");
    Ok(())
}
