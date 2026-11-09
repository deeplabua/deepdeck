# DeepDeck

> Remembers what you talked about with each agent.

[![CI](https://github.com/deeplabua/deepdeck/actions/workflows/ci.yml/badge.svg)](https://github.com/deeplabua/deepdeck/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/deepdeck.svg)](https://crates.io/crates/deepdeck)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

`ddeck` is a task board for the terminal, made for people who run several Claude Code agents at
once — in herdr panes, tmux windows or a row of tabs. It lives
in a narrow side panel and shows, across all your projects:

- **who is waiting for you** — an agent stopped on a permission prompt or a question;
- **each agent's plan** and how far along it is;
- **the project roadmap** — what comes after the current request;
- **open questions and ideas** — the ones you postponed, and the ones the agents raised;
- **a timeline** — what you asked and what each agent answered, an hour or a day ago;
- **what the last session left** — a short summary the next session reads first.

Everything stays on your machine. No network, no telemetry, one static binary.

> **Status:** early (v0.2). Works with Claude Code on macOS and Linux; Windows builds are provided
> but not yet tested with Claude Code.

```
 Projects  Timeline  Inbox 2                        2 waiting
● api-server         waiting for you · 3m
  Claude needs your permission to use Bash
  plan 2/4 → Migrate the session table
  you: add a created_at index and backfill it
● web-app            waiting for you · 12m
  Claude is waiting for your input
◐ cli-tool           working · 1m
  plan 3/5 → Wire the new flag into the parser
    ✓ Read the current parser
    ✓ Add --since to the args struct
    ◐ Wire the new flag into the parser
    ○ Tests for date parsing
    ○ Update the README
  roadmap 1/3
  ideas                                            0/2
    ◆ Shell completions for zsh and fish
    ◆ Cache the remote index between runs
  last session: Added --since; left: tests and docs.
  docs-site          idle 2h · 1 question
```

## Install

Homebrew (macOS / Linux):

```sh
brew install deeplabua/tap/deepdeck
```

From crates.io:

```sh
cargo install deepdeck
```

Or download a prebuilt binary from the [releases page](https://github.com/deeplabua/deepdeck/releases).

### Updating

Update the way you installed: `brew upgrade deepdeck`, `cargo install deepdeck --locked`, or run the installer
again. Then run `ddeck setup` if the board asks for it — a new version may bring new agent instructions.

The board tells you when a newer DeepDeck is out and shows the command for your install. It learns that from
files brew and cargo already keep (the tap's formula, cargo's index cache) — DeepDeck itself never goes online,
so the hint appears once brew or cargo has refreshed (any `brew update`, `install` or `upgrade` does it).
`ddeck doctor` shows the same, and so do `ddeck list`, `ddeck status`, `ddeck cmd`, `ddeck -V` and `ddeck -h` when you run them in a terminal
(agents, which read them through a pipe, don't see it).

## Set up

```sh
ddeck setup --dry-run   # shows exactly what will change
ddeck setup
```

`setup` does two things, each shown as a diff first, each with a backup of the file it edits:

1. **Adds hooks to `~/.claude/settings.json`** — `SessionStart`, `UserPromptSubmit`, `Notification`,
   `Stop`, `SessionEnd` (and `PostToolUse` for task tools). Each one runs `ddeck hook`, which appends one
   line to a log and exits: a few milliseconds, always exit code 0, nothing on stdout. Hooks from other
   tools are left alone, and running `setup` again adds nothing twice.
2. **Adds a short block to `~/.claude/CLAUDE.md`** (between `<!-- ddeck:begin -->` and
   `<!-- ddeck:end -->`) that teaches agents to post their plan, questions and ideas with the commands
   below. `ddeck snippet` prints it. Use `ddeck setup --project` to put it in one repository's
   `CLAUDE.md` instead.

Restart running agents so they read the new instructions. `ddeck doctor` checks that everything is in
place. Hooks point to the binary's full path: after upgrading, run `ddeck setup` again (`doctor` tells
you when it is needed).

To undo: `ddeck uninstall` removes our hooks and the block, and nothing else.

## The board

Run `ddeck` in a side panel (60 columns is plenty). Three screens:

- **Projects** — every active project: agents and their state, the plan, the roadmap, questions, ideas
  and tasks. Idle projects with nothing open share one compact line; after 12 quiet hours a project
  drops off unless it still has open items.
- **Timeline** — your prompts and the agents' short answers, newest first, across projects or just one.
- **Inbox** — what needs you: waiting agents and open questions.

| Key | Action |
| --- | --- |
| `Tab` / `←` `→` | Switch screens |
| `j` `k` / `↑` `↓` | Move |
| `↵` | Expand or collapse (a plan, a long prompt, a project's done items) |
| `␣` | Mark done / reopen |
| `o` | Jump to the agent's pane in herdr (the waiting one, or the plan step's own) |
| `y` | Copy the selected line in full (a command, a note, a prompt, a step) |
| `s` | Projects by name (default, they stay put) / by attention (waiting first); remembered |
| `a` `?` `i` | Add a task, question or idea |
| `e` / `d` | Edit / delete |
| `f` | One project / all — on every screen (the selected project) |
| `q` | Quit |

The mouse works too: click to select, click again to expand, scroll the wheel, click a tab. Drag to select
text — it is copied when you let go (the board holds the mouse, so the terminal's own selection is off);
`y` copies the selected line in full, including what is cut off on screen.

The board reloads itself when the binary changes, so an upgrade does not need a restart.

For a status bar (tmux, herdr), `ddeck status --short` prints one number: agents waiting for you plus
open questions. `ddeck status` prints the whole board as text.

## Languages

The board and the screens you read (`status`, `doctor`, `setup`) speak English, Deutsch, Español,
Français, Italiano, 日本語, 한국어, Português (Brasil), Русский, Українська and 简体中文. The language follows
your system (`LC_ALL` / `LC_MESSAGES` / `LANG`, on macOS the first preferred language); change it with:

```sh
ddeck lang        # the current one and the choices
ddeck lang ru     # set one
ddeck lang auto   # follow the system again
```

`DEEPDECK_LANG=en ddeck doctor` gives English for a bug report. What agents read (`ddeck list`, the answers
of agent commands, the instructions block) and `--help` stay in English.

## Commands for agents

The `CLAUDE.md` block teaches agents these; you can use them too. Each prints one line.

```sh
ddeck plan "Read the parser" "Add the flag" "Tests"   # post the plan (replaces the previous one)
ddeck step 2 doing                                     # progress: doing | done
ddeck roadmap set "Release 0.3" "Shell completions"    # what comes after this request
ddeck roadmap step 1 done
ddeck ask "Keep the old flag as an alias?"             # a question only you can answer
ddeck idea "Cache the index between runs"              # for later
ddeck add "Update the README"                          # a task
ddeck done <id>
ddeck summary "Added --since; decided to keep -s; left: tests."
ddeck list                                             # the project's plan and open items (--json)
ddeck retract "a few words"                            # a message sent to the wrong session
ddeck cmd add "release" 'scripts/release.sh 0.3.0'     # a project command worth keeping
```

### Saved commands

Each project keeps a short runbook — the commands you or an agent will need again: a release, a deploy,
setting CI secrets. They show on the board under `commands` and in `ddeck list`.

```sh
ddeck cmd add "ci secrets" 'dv run --item "Release" --sh '\''printf %s "$TOKEN" | gh secret set TOKEN'\'''
ddeck cmd                 # list
ddeck cmd run release     # run in the project root (a unique prefix is enough)
ddeck cmd show release    # print it
ddeck cmd rm release
```

The log is plain text, so a command must refer to secrets — a vault (`dv run --item …`) or an environment
variable — never contain them. `cmd add` refuses anything that looks like a token or a password
(`--force` if it is a false alarm).

Commands find the project from the current directory (the nearest `.git` or `.jj` root); `--project
<path>` points elsewhere.

`ddeck forget <project>` removes a project from the board (its log is kept). `ddeck archive` moves old
sessions and closed notes out of the live log; the board does this by itself on start, after 14 days.

## Privacy

All data lives in `~/.deepdeck/` (owner-only permissions; `DEEPDECK_HOME` moves it). Nothing leaves your
machine — DeepDeck has no network code at all.

What is kept: your prompts cut to 200 characters, a one- or two-sentence recap of each agent answer
(up to 240 characters), plans, questions, ideas and summaries, with times and session ids. Transcripts are never
read or copied. The log is plain JSON Lines — `~/.deepdeck/projects/<id>/events.jsonl`; delete it any
time.

## Building

```sh
cargo build --release
cargo test
```

Rust 1.89 or newer; `rust-toolchain.toml` pins the version CI uses.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
