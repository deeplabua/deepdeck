//! `ddeck` without arguments: the board in a narrow terminal panel.
//!
//! Synchronous loop: wait for a key up to one tick, poll the logs, redraw.
//! Never imported from the `hook` path.

mod app;
mod inbox;
mod input;
mod projects;
mod select;
mod source;
pub(crate) mod strings;
mod text;
mod theme;
mod timeline;

use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use deepdeck_core::store::{self, Paths};
use deepdeck_core::{Event, NoteType, Source as Src};
use jiff::Timestamp;
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event as TermEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::install;
use app::{App, Effect, Screen};
use projects::{View, layout_projects};
use source::Source;
#[cfg(test)]
use strings::EN;
use strings::Strings;
use theme::Theme;

/// How long a key wait lasts before the logs are polled again.
const TICK: Duration = Duration::from_millis(250);
/// Projects quiet for longer than this are hidden (unless they have notes).
const ACTIVE_HOURS: i64 = 12;
/// Notices are re-read every this many ticks (~1 min).
const NOTICE_EVERY: u32 = 240;
/// A status message (copied, sorted, an error) is cleared after this long.
const MESSAGE_FOR: Duration = Duration::from_secs(4);

pub fn run(paths: Paths, settings_path: &Path) -> Result<()> {
    let hooks_installed = install::load_settings(settings_path)
        .map(|(_, v)| install::installed(&v).iter().all(|(_, c)| c.is_some()))
        .unwrap_or(false);
    // Housekeeping before the first read: old sessions and closed notes move
    // to `archive/`, so the logs the board folds stay small.
    let cutoff = Timestamp::now()
        - jiff::SignedDuration::from_hours(deepdeck_core::archive::ARCHIVE_AFTER_DAYS * 24);
    store::rotate_all(&paths, cutoff);
    let mut source = Source::new(paths);
    source.poll()?;
    let mut app = App::new(
        source.board(Timestamp::now(), ACTIVE_HOURS),
        hooks_installed,
    );
    app.set_order(load_order(&source.paths().tui_prefs()));
    app.notices = crate::update::notices(hooks_installed);
    let mut ticks: u32 = 0;

    let mut watch = BinaryWatch::new();
    let mut terminal = ratatui::init();
    // Mouse capture must be released on every exit path, panics included.
    let _ = ratatui::crossterm::execute!(
        std::io::stdout(),
        ratatui::crossterm::event::EnableMouseCapture
    );
    let restore_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = ratatui::crossterm::execute!(
            std::io::stdout(),
            ratatui::crossterm::event::DisableMouseCapture
        );
        restore_hook(info);
    }));
    let res = (|| -> Result<Exit> {
        let mut message: Option<String> = None;
        let mut message_until = std::time::Instant::now();
        loop {
            let now = Timestamp::now();
            let frame = terminal.draw(|f| {
                draw(
                    f,
                    &mut app,
                    &source,
                    now,
                    message.as_deref(),
                    crate::i18n::t(),
                    &Theme::from_env(),
                );
                if let Some(sel) = app.drag.filter(|s| !s.is_empty()) {
                    sel.highlight(f.buffer_mut());
                }
            })?;
            // What is on screen, for copying a mouse selection.
            let screen = frame.buffer.clone();
            let effect = if event::poll(TICK)? {
                match event::read()? {
                    TermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                        message = None;
                        app.handle_key(key, now)
                    }
                    TermEvent::Mouse(m) => app.handle_mouse(m),
                    _ => None,
                }
            } else {
                None
            };
            let had_effect = effect.is_some();
            {
                match effect {
                    Some(Effect::Quit) => return Ok(Exit::Quit),
                    Some(Effect::Write(project, bodies)) => {
                        for body in bodies {
                            let event = Event::new(Timestamp::now(), Src::User, None, body);
                            if let Err(e) = store::append(&source.paths().events(&project), &event)
                            {
                                message = Some(format!("{e}"));
                                break;
                            }
                        }
                    }
                    Some(Effect::SaveOrder(order)) => {
                        let saved = save_order(&source.paths().tui_prefs(), order);
                        message = Some(match saved {
                            Ok(()) => (crate::i18n::t().sorted)(order),
                            Err(e) => format!("{e}"),
                        });
                    }
                    Some(Effect::CopySelection(sel)) => {
                        let text = sel.text(&screen);
                        message = Some(match crate::clipboard::copy(&text) {
                            Ok(how) => (crate::i18n::t().copied)(how),
                            Err(e) => e,
                        });
                    }
                    Some(Effect::Copy(text)) => {
                        message = Some(match crate::clipboard::copy(&text) {
                            Ok(how) => (crate::i18n::t().copied)(how),
                            Err(e) => e,
                        });
                    }
                    Some(Effect::Focus { pane, root }) => {
                        message = Some(if !crate::herdr::available() {
                            crate::i18n::t().herdr_outside.to_owned()
                        } else {
                            match crate::herdr::focus(pane.as_deref(), &root) {
                                Ok(()) => String::new(),
                                Err(e) => e,
                            }
                        })
                        .filter(|m| !m.is_empty());
                    }
                    None => {}
                }
            }
            // A message lives a few seconds, not until the next key press.
            if had_effect && message.is_some() {
                message_until = std::time::Instant::now() + MESSAGE_FOR;
            } else if message.is_some() && std::time::Instant::now() >= message_until {
                message = None;
            }
            if source.poll()? {
                app.set_board(source.board(Timestamp::now(), ACTIVE_HOURS));
            }
            // brew may have learned of a release, `ddeck setup` may have run: about once a minute.
            ticks = ticks.wrapping_add(1);
            if ticks.is_multiple_of(NOTICE_EVERY) {
                app.notices = crate::update::notices(app.hooks_installed);
            }
            // A new ddeck was installed: restart into it, unless the user is typing.
            if app.input.is_none() && watch.updated() {
                return Ok(Exit::Reload);
            }
        }
    })();
    let _ = ratatui::crossterm::execute!(
        std::io::stdout(),
        ratatui::crossterm::event::DisableMouseCapture
    );
    ratatui::restore();
    match res? {
        Exit::Quit => Ok(()),
        Exit::Reload => reload(),
    }
}

enum Exit {
    Quit,
    Reload,
}

/// Notices when the `ddeck` binary on disk is replaced (`cargo install`,
/// `brew upgrade`), so a TUI left open in a side panel does not keep showing an
/// old build that skips newer event kinds.
struct BinaryWatch {
    path: Option<std::path::PathBuf>,
    seen: Option<(u64, std::time::SystemTime)>,
    /// A changed stamp must be seen twice in a row: the file may still be written.
    pending: Option<(u64, std::time::SystemTime)>,
    ticks: u32,
}

impl BinaryWatch {
    /// Check every this many loop ticks (~2 s).
    const EVERY: u32 = 8;

    fn new() -> Self {
        let path = std::env::current_exe().ok();
        let seen = path.as_deref().and_then(Self::stamp);
        BinaryWatch {
            path,
            seen,
            pending: None,
            ticks: 0,
        }
    }

    fn stamp(path: &Path) -> Option<(u64, std::time::SystemTime)> {
        let m = std::fs::metadata(path).ok()?;
        Some((m.len(), m.modified().ok()?))
    }

    fn updated(&mut self) -> bool {
        self.ticks = self.ticks.wrapping_add(1);
        if !self.ticks.is_multiple_of(Self::EVERY) {
            return false;
        }
        let Some(now) = self.path.as_deref().and_then(Self::stamp) else {
            return false;
        };
        if Some(now) == self.seen {
            self.pending = None;
            return false;
        }
        if self.pending == Some(now) {
            return true;
        }
        self.pending = Some(now);
        false
    }
}

/// Board preferences (`tui.json`): missing or unreadable means the defaults.
fn load_order(path: &Path) -> deepdeck_core::Order {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| serde_json::from_value(v["order"].clone()).ok())
        .unwrap_or_default()
}

/// Keeps the other preferences (the language) in the same file.
fn save_order(path: &Path, order: deepdeck_core::Order) -> std::io::Result<()> {
    let value = serde_json::to_value(order).map_err(std::io::Error::other)?;
    crate::i18n::set_pref(path, "order", Some(value))
}

/// Replace this process with the freshly installed binary, same arguments.
fn reload() -> Result<()> {
    let exe = std::env::current_exe()?;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&exe).args(&args).exec();
        Err(anyhow::anyhow!("restarting {}: {err}", exe.display()))
    }
    #[cfg(not(unix))]
    {
        let status = std::process::Command::new(&exe).args(&args).status()?;
        std::process::exit(status.code().unwrap_or(0));
    }
}

fn draw(
    f: &mut Frame<'_>,
    app: &mut App,
    source: &Source,
    now: Timestamp,
    message: Option<&str>,
    t: &Strings,
    th: &Theme,
) {
    let notice_rows = u16::try_from(app.notices.len()).unwrap_or(0);
    let [header, body, notices, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(notice_rows),
        Constraint::Length(1),
    ])
    .areas(f.area());
    let lines: Vec<Line<'_>> = app
        .notices
        .iter()
        .map(|n| {
            Line::styled(
                text::fit(
                    &format!(" {}{}", (t.notice)(n), t.notice_copy),
                    usize::from(notices.width),
                ),
                th.waiting(),
            )
        })
        .collect();
    f.render_widget(Paragraph::new(lines), notices);
    app.notice_top = notices.y;
    draw_header(f, header, app, t, th);
    match app.screen {
        Screen::Projects => draw_projects(f, body, app, now, t, th),
        Screen::Timeline => {
            let logs = source.logs();
            let lines = timeline::layout_timeline(
                &logs,
                now,
                &jiff::tz::TimeZone::system(),
                body.width,
                app.only.as_ref(),
                t,
                th,
            );
            let max = lines.len().saturating_sub(usize::from(body.height));
            app.timeline_scroll = app.timeline_scroll.min(max);
            let scroll = u16::try_from(app.timeline_scroll).unwrap_or(u16::MAX);
            f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), body);
        }
        Screen::Inbox => {
            let r = {
                let view = View {
                    selected: app.selected.as_ref(),
                    open: &app.open,
                };
                inbox::layout_inbox(&app.shown(), now, body.width, &view, t, th)
            };
            show_selectable(f, body, app, r);
        }
    }
    draw_footer(f, footer, app, message, t, th);
}

fn draw_header(f: &mut Frame<'_>, area: Rect, app: &mut App, t: &Strings, th: &Theme) {
    let line = header_line(
        app.screen,
        inbox::inbox_count(&app.board),
        app.board.waiting_count(),
        usize::from(area.width),
        t,
        th,
    );
    app.tabs = tab_areas(&line, area.x);
    f.render_widget(Paragraph::new(line), area);
}

/// Click areas of the tabs: spans that start with " N " (the inbox count span
/// has no leading space, so it is not mistaken for a tab).
fn tab_areas(line: &Line<'_>, x0: u16) -> Vec<(Screen, std::ops::Range<u16>)> {
    let mut x = x0;
    let mut out = Vec::new();
    for span in &line.spans {
        let w = u16::try_from(text::width(&span.content)).unwrap_or(u16::MAX);
        let screen = match span
            .content
            .strip_prefix(' ')
            .and_then(|s| s.chars().next())
        {
            Some('1') => Some(Screen::Projects),
            Some('2') => Some(Screen::Timeline),
            Some('3') => Some(Screen::Inbox),
            _ => None,
        };
        if let Some(s) = screen {
            out.push((s, x..x.saturating_add(w)));
        }
        x = x.saturating_add(w);
    }
    out
}

/// Title, tabs, inbox count and the "waiting" badge — shortened step by step
/// until it fits: inactive tabs become bare numbers, then the title goes.
fn header_line(
    screen: Screen,
    inbox: usize,
    waiting: usize,
    width: usize,
    t: &Strings,
    th: &Theme,
) -> Line<'static> {
    let badge = (waiting > 0).then(|| format!("● {waiting} "));
    let badge_w = badge.as_deref().map_or(0, text::width);
    for (title, labels) in [(true, true), (true, false), (false, false)] {
        let mut spans = Vec::new();
        if title {
            spans.push(Span::styled(format!(" {} ", t.app), th.accent()));
        }
        for (n, label, s) in [
            ("1", t.tab_projects, Screen::Projects),
            ("2", t.tab_timeline, Screen::Timeline),
            ("3", t.tab_inbox, Screen::Inbox),
        ] {
            let (text, style) = if s == screen {
                (format!(" {n} {label} "), th.tab_active())
            } else if labels {
                (format!(" {n} {label} "), th.dim())
            } else {
                (format!(" {n} "), th.dim())
            };
            spans.push(Span::styled(text, style));
            if s == Screen::Inbox && inbox > 0 {
                spans.push(Span::styled(format!("{inbox} "), th.waiting()));
            }
        }
        let used: usize = spans.iter().map(|s| text::width(&s.content)).sum();
        if used + badge_w <= width || !title && !labels {
            if let Some(b) = &badge {
                spans.push(Span::raw(" ".repeat(width.saturating_sub(used + badge_w))));
                spans.push(Span::styled(b.clone(), th.waiting()));
            }
            return Line::from(spans);
        }
    }
    unreachable!("the last variant always returns")
}

fn draw_projects(
    f: &mut Frame<'_>,
    area: Rect,
    app: &mut App,
    now: Timestamp,
    t: &Strings,
    th: &Theme,
) {
    if app.board.projects.is_empty() {
        let lines = if app.hooks_installed {
            t.empty_no_events
        } else {
            t.empty_no_hooks
        };
        let lines: Vec<Line<'_>> = std::iter::once(Line::default())
            .chain(lines.iter().map(|l| Line::raw(format!(" {l}"))))
            .collect();
        f.render_widget(Paragraph::new(lines), area);
        return;
    }
    let r = {
        let view = View {
            selected: app.selected.as_ref(),
            open: &app.open,
        };
        layout_projects(&app.shown(), now, area.width, &view, t, th)
    };
    show_selectable(f, area, app, r);
}

/// Render a screen with a cursor (Projects, Inbox), scrolled so that the
/// selected row and the line after it are visible.
fn show_selectable(f: &mut Frame<'_>, area: Rect, app: &mut App, r: projects::Rendered) {
    let rows = r.items;
    app.rows = rows.clone();
    app.copy_rows = r.copy_rows;
    app.body_top = area.y;
    app.set_items(rows.iter().map(|(i, _)| i.clone()).collect());

    // Keep the selected item on screen: all its lines (an opened one wraps)
    // and the line after it, as far as the height allows; its first line always.
    let height = usize::from(area.height);
    if let Some(pos) = app
        .selected
        .as_ref()
        .and_then(|s| rows.iter().position(|(i, _)| i == s))
    {
        let row = rows[pos].1;
        let next = rows.get(pos + 1).map_or(r.lines.len(), |(_, start)| *start);
        let end = (next + 1).max(row + 2).min(row + height);
        if row < app.scroll {
            app.scroll = row.saturating_sub(1);
        } else if end > app.scroll + height {
            app.scroll = end.saturating_sub(height);
        }
    }
    app.scroll = app.scroll.min(r.lines.len().saturating_sub(height));
    let scroll = u16::try_from(app.scroll).unwrap_or(u16::MAX);
    f.render_widget(Paragraph::new(r.lines).scroll((scroll, 0)), area);
}

fn draw_footer(
    f: &mut Frame<'_>,
    area: Rect,
    app: &App,
    message: Option<&str>,
    t: &Strings,
    th: &Theme,
) {
    let width = usize::from(area.width);
    if let Some(input) = &app.input {
        let kind = match input.kind {
            NoteType::Task => t.kind_task,
            NoteType::Question => t.kind_question,
            NoteType::Idea => t.kind_idea,
            NoteType::Handoff => t.kind_handoff,
        };
        let project = app
            .board
            .projects
            .iter()
            .find(|p| p.meta.id == input.project)
            .map_or("?", |p| p.meta.name.as_str());
        let label = if input.editing.is_some() {
            format!(" {} {kind} · {project}: ", t.edit_label)
        } else {
            format!(" {kind} → {project}: ")
        };
        let label_w = text::width(&label);
        // Scroll the text so the cursor stays visible.
        let room = width.saturating_sub(label_w + 1).max(1);
        let before = input.before_cursor();
        let mut start = 0;
        while text::width(&before[start..]) > room {
            start += before[start..].chars().next().map_or(1, char::len_utf8);
        }
        let shown = text::fit(&input.text[start..], room);
        let cursor_x = label_w + text::width(&before[start..]);
        let mut spans = vec![Span::styled(label, th.accent()), Span::raw(shown.clone())];
        let hint_room = width.saturating_sub(label_w + text::width(&shown) + 2);
        if text::width(t.input_hint) <= hint_room {
            spans.push(Span::raw(
                " ".repeat(hint_room - text::width(t.input_hint) + 1),
            ));
            spans.push(Span::styled(t.input_hint, th.dim()));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
        let x = area
            .x
            .saturating_add(u16::try_from(cursor_x).unwrap_or(u16::MAX));
        f.set_cursor_position(Position::new(x.min(area.right().saturating_sub(1)), area.y));
        return;
    }
    if let Some(msg) = message {
        f.render_widget(
            Paragraph::new(Line::styled(
                text::fit(&format!(" {msg}"), width),
                th.waiting(),
            )),
            area,
        );
        return;
    }
    let mut spans = vec![Span::raw(" ")];
    let mut used = 1;
    if let Some(name) = app
        .only
        .as_ref()
        .and_then(|id| app.board.projects.iter().find(|p| &p.meta.id == id))
        .map(|p| p.meta.name.clone())
    {
        let label = format!("{}  ", (t.only)(&name));
        used += text::width(&label);
        spans.push(Span::styled(label, th.accent()));
    }
    let keys = match app.screen {
        Screen::Projects => t.keys,
        Screen::Timeline => t.keys_timeline,
        Screen::Inbox => t.keys_inbox,
    };
    // `o` jumps to an agent's herdr pane: only worth a hint inside herdr.
    let herdr = crate::herdr::available();
    for (k, label) in keys.iter().filter(|(k, _)| herdr || *k != "o") {
        let w = text::width(k) + 1 + text::width(label) + 2;
        if used + w > width {
            break;
        }
        used += w;
        spans.push(Span::styled(*k, th.name()));
        spans.push(Span::styled(format!(" {label}  "), th.dim()));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_preference_round_trips() {
        let dir = std::env::temp_dir().join(format!("ddeck-prefs-{}", std::process::id()));
        let path = dir.join("tui.json");
        assert_eq!(load_order(&path), deepdeck_core::Order::Name);
        save_order(&path, deepdeck_core::Order::Attention).unwrap();
        assert_eq!(load_order(&path), deepdeck_core::Order::Attention);
        std::fs::write(&path, "garbage").unwrap();
        assert_eq!(load_order(&path), deepdeck_core::Order::Name);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn binary_watch_needs_a_stable_new_stamp() {
        let path = std::env::temp_dir().join(format!("ddeck-watch-{}", std::process::id()));
        std::fs::write(&path, "v1").unwrap();
        let mut w = BinaryWatch {
            seen: BinaryWatch::stamp(&path),
            path: Some(path.clone()),
            pending: None,
            ticks: 0,
        };
        let check = |w: &mut BinaryWatch| (0..BinaryWatch::EVERY).any(|_| w.updated());
        assert!(!check(&mut w));
        std::fs::write(&path, "v2 longer").unwrap();
        assert!(!check(&mut w), "first sighting only arms the watch");
        assert!(check(&mut w), "same new stamp twice: reload");
        std::fs::remove_file(&path).unwrap();
        assert!(!check(&mut w), "binary gone (mid-replace): keep running");
    }

    #[test]
    fn tabs_are_clickable() {
        let th = Theme { color: false };
        let tabs = tab_areas(&header_line(Screen::Projects, 2, 0, 60, &EN, &th), 0);
        let screens: Vec<_> = tabs.iter().map(|(s, _)| *s).collect();
        assert_eq!(screens, [Screen::Projects, Screen::Timeline, Screen::Inbox]);
        assert_eq!(tabs[0].1, 10..22, "' 1 Projects ' after ' DeepDeck '");
    }

    #[test]
    fn header_shrinks_to_fit() {
        let th = Theme { color: false };
        let plain = |l: &Line<'_>| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        };
        let full = plain(&header_line(Screen::Projects, 2, 1, 60, &EN, &th));
        assert!(
            full.starts_with(" DeepDeck  1 Projects  2 Timeline  3 Inbox 2 "),
            "{full}"
        );
        assert!(
            full.ends_with(" ● 1 ") && text::width(&full) == 60,
            "{full}"
        );
        let mid = plain(&header_line(Screen::Timeline, 2, 1, 40, &EN, &th));
        assert!(
            mid.starts_with(" DeepDeck  1  2 Timeline  3 2 ") && text::width(&mid) == 40,
            "{mid}"
        );
        assert_eq!(
            plain(&header_line(Screen::Projects, 0, 0, 22, &EN, &th)),
            " 1 Projects  2  3 "
        );
    }
}
