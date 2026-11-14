# Changelog

All notable changes to DeepDeck are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.7.1 — 2026-11-14

### Fixed

- `f` is remembered per board: each board keeps its own project, keyed by the directory it runs in.
  In 0.7.0 one choice was shared, so boards open in several projects' tabs overwrote each other.
  Choose once more with `f` after upgrading.

## 0.7.0 — 2026-11-14

### Added

- **`ddeck find <words>`** — which session talked about something, across all projects: your messages, agent
  replies and summaries, notes, plans, roadmaps and saved commands, the archive included. Grouped by
  session, newest first, with the `claude --resume` command to get back to it. All words must match;
  endings are ignored. `--all`, `-p <project>`, `--json`.
- **Search on the board:** `/` opens the same search inside the TUI. `↵` on a result opens its project, `o`
  jumps to the agent in herdr, `y` copies `claude --resume` for the session.

### Fixed

- `f` (one project only) is remembered: the board no longer shows every project again after it
  restarts, for example into a newly installed version.

## 0.6.2 — 2026-11-12

The 0.6.1 build never ran: a GitHub Actions outage left it queued. 0.6.2 ships the same changes.

## 0.6.1 — 2026-11-11 (not published)

### Changed

- **Agent instructions (block v8):** the roadmap holds the project's own steps. What the user has to check,
  try or decide goes to «your part» (`ddeck handoff`), and steps of that kind already on a roadmap move there.

- A long command under «your part» wraps onto the next rows, spaces kept as typed, instead of being cut with
  `…`; a click on any of its rows copies it.

### Fixed

- A status line such as «copied · pbcopy» clears itself after a few seconds instead of staying until the
  next key press.

## 0.6.0 — 2026-11-11

### Added

- **Your part.** `ddeck handoff "…" [--run '<command>']` hands the user a step only they can take — a check
  on a device, a login, a decision, a command the agent may not run. It shows in the Inbox right after
  waiting agents, and in the project, with its command as `$ …`: `y` or a click copies it, DeepDeck never
  runs it. `ddeck list` shows them to agents; `ddeck done <id>` closes one.

### Changed

- **Agent instructions (block v7):** run each `ddeck` command as its own shell call. Chained with a push or a
  deploy, a board update was lost whenever the permission check refused the whole line.
- **Agent instructions (block v7):** the reply that ends a piece of work names what is left — not verified, put off,
  or for the user to do or check; the user's part goes to `ddeck handoff`, later work to ideas — not to the plan,
  which the next plan replaces. The roadmap is for the project's own next steps.

## 0.5.0 — 2026-11-09

### Added

- **Eleven languages.** The board, `ddeck status`, `doctor`, `setup` and the update hints in English, German,
  Spanish, French, Italian, Japanese, Korean, Brazilian Portuguese, Russian, Ukrainian and Simplified Chinese.
  The language follows the system; `ddeck lang <code>` sets one, `ddeck lang auto` follows the system again,
  `DEEPDECK_LANG` overrides both. What agents read stays in English.

## 0.4.3 — 2026-11-09

### Added

- **`ddeck list`, `ddeck status`, `ddeck cmd`, `-V` and `-h` show the same hints** (a newer version, `ddeck setup`) at the end — only in a
  terminal. Agents read these commands through a pipe, so they never see the hint and never act on it.

## 0.4.2 — 2026-11-09

### Added

- **`u` copies the update command** shown above the footer (a click on the line does the same).
- A build from a source checkout also gets the hint, with `cargo install --path <checkout> --locked`.

### Fixed

- **Hooks of a Homebrew install survive `brew upgrade`.** `ddeck setup` wrote the versioned Cellar path,
  which an upgrade removes; it now writes `<prefix>/opt/deepdeck/bin/ddeck`. Run `ddeck setup` once after
  upgrading to fix existing hooks.

## 0.4.1 — 2026-11-09

### Added

- **The board says when to update.** A line above the footer shows a newer DeepDeck that brew or cargo has
  already seen, with the upgrade command for this install (brew, cargo or the installer), and agent
  instructions older than this binary (`ddeck setup`). Read from files brew and cargo keep — no network.
  `ddeck doctor` shows the same.

## 0.4.0 — 2026-11-09

### Added

- **`f` shows one project on every screen.** Projects and Inbox narrow to the selected project the same way
  the Timeline already did; `f` again shows all. The header counts stay global, so a waiting agent elsewhere
  is still visible.

### Changed

- **Saved commands read like a script.** Each one takes two lines on the board — `# name`, then
  `$ command` with the full width — and after three the rest fold into `+ N more` (`↵` on the project shows
  all). An opened item now scrolls into view whole, not just its first line.
- **`ddeck cmd` lists commands the same way** — `# name`, then `$ command` — colored in a terminal
  (`NO_COLOR` turns it off).
- **Agent instructions (block v6):** save a command once it has worked, even one the agent ran itself;
  update the roadmap in the same turn as a "what's next" reply, keeping the steps still open; check both when
  saving a summary. `ddeck setup` updates the block.

## 0.3.2 — 2026-10-31

The first published release of the 0.3 line. The 0.3.0 tag built no binaries (a masked release note
emptied the build plan) and 0.3.1 stopped on a retired macOS runner; macOS builds now use macOS 15.

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
