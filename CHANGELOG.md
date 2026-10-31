# Changelog

All notable changes to DeepDeck are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0 — unreleased

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
