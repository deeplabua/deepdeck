# Changelog

All notable changes to DeepDeck are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.1 — 2026-10-31

The 0.3.0 tag built no binaries (a masked release note emptied the build plan); 0.3.1 is the same
release, published.

### Added

- **`o` jumps to the agent in herdr.** On Projects and in the Inbox, `o` switches herdr to the pane of the
  agent behind the selected line — the one waiting for you, or the session a plan step belongs to. The pane
  comes from the hook (`HERDR_PANE_ID`); for agents started before the hooks, the agent working in the
  project directory is found with `herdr agent list`. Shown only when `ddeck` itself runs inside herdr.
- **Saved commands (runbook).** `ddeck cmd add "<name>" '<command>'` keeps a project command worth repeating —
  a release, a deploy, setting CI secrets; `ddeck cmd` lists them, `cmd run` runs one in the project root,
  `cmd show` prints it, `cmd rm` removes it. They appear under `commands` on the Projects screen, in the
  Timeline and in `ddeck list` (also `--json`), and survive log archiving. Anything that looks like a secret
  value (a token, a key, a literal password) is refused: refer to a vault or an environment variable instead.
- **`y` copies the selected line** in full — a saved command, a note, a prompt, a plan step — with `pbcopy`,
  `wl-copy`, `xclip`, `xsel` or `clip`, else through the terminal (OSC 52). Mouse capture had made the
  terminal's own selection impossible.
- **Drag to select text** on any screen: the selection is highlighted and copied when the button is released;
  a click without a drag still selects and opens lines.
- **Projects stay in place:** listed by name by default; `s` switches to the attention order (waiting →
  working → idle) and back, remembered in `~/.deepdeck/tui.json`.
- The `CLAUDE.md` block (v5) teaches agents to save such commands and to check `ddeck cmd` before asking.

## 0.2.0 — 2026-10-31

The first public release.

### Added

- **Hook capture.** `ddeck hook` records Claude Code events — session start and end, your prompts (cut to
  200 characters), permission and input waits, a short recap of each answer — as one JSON line each in
  `~/.deepdeck/projects/<id>/events.jsonl`. It takes a few milliseconds, always exits 0, prints nothing,
  and never reads transcripts. System messages and pasted-content wrappers are filtered out.
- **`ddeck setup`** adds the hooks to `~/.claude/settings.json` and a short instruction block to
  `~/.claude/CLAUDE.md`, showing a diff and keeping a backup; `--dry-run` writes nothing, `--project`
  puts the block in one repository instead. `ddeck uninstall` removes only what `setup` added.
  `ddeck doctor` checks hooks, the block version, the data directory and recent events.
- **Commands for agents:** `plan` / `step` for the plan of the current request, `roadmap` for what comes
  after it, `ask`, `idea`, `add`, `done`, `list` (also `--json`), `summary` for what a session did and left,
  and `retract` for a message sent to the wrong session.
- **The board (`ddeck`)**, a TUI for a narrow side panel with three screens:
  - *Projects* — agents and their state, plans, the roadmap, questions, ideas and tasks grouped under
    headers, done items kept in grey, the last session's summary; idle projects share one line.
  - *Timeline* — your prompts and the agents' recaps across projects, with retracted messages hidden.
  - *Inbox* — waiting agents and open questions.
  - Cursor over every line, `↵` to expand, `␣` to mark done or reopen, notes added and edited in place,
    `Tab` / `←` `→` between screens, mouse support. Live updates; reloads itself when the binary changes.
- **`ddeck status`** — the board as text; `--short` prints one number for a status bar.
- **`ddeck forget`** removes a project from the board, keeping its log.
- **Install** with Homebrew, `cargo install deepdeck`, or prebuilt binaries for macOS (arm64, x86_64), Linux
  (x86_64) and Windows (x86_64; not yet tested with Claude Code on Windows).
- **Log archive.** On start the board moves sessions older than 14 days, deleted and long-closed notes and
  superseded roadmaps to `archive/<YYYY-MM>.jsonl`; open items stay. `ddeck archive` runs it by hand.
