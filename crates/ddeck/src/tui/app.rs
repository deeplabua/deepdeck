//! TUI state and key handling. No terminal and no files: keys produce
//! [`Effect`]s that the loop in `mod.rs` carries out.

use std::collections::HashSet;

use deepdeck_core::event::note_id;
use deepdeck_core::{Board, EventBody, NoteType, Order, ProjectId};
use jiff::Timestamp;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use super::input::{InputLine, InputOutcome};
use super::projects::Item;
use super::select::Selection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Projects,
    Timeline,
    Inbox,
}

/// Something the loop has to do outside the state.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Quit,
    /// Append user events to this project's log, in order.
    Write(ProjectId, Vec<EventBody>),
    /// Copy this text to the clipboard.
    Copy(String),
    /// The project order changed: remember it.
    SaveOrder(Order),
    /// Copy what this mouse selection covers on screen.
    CopySelection(super::select::Selection),
    /// Switch to the agent's herdr pane (recorded pane, else found by `root`).
    Focus {
        pane: Option<String>,
        root: std::path::PathBuf,
    },
}

pub struct App {
    pub board: Board,
    pub screen: Screen,
    pub selected: Option<Item>,
    /// Selectable items in screen order, refreshed on every layout.
    pub items: Vec<Item>,
    /// Where each item starts in the scrolled content, from the last layout (mouse).
    pub rows: Vec<(Item, usize)>,
    /// Screen row where the body starts, and tab hit areas `(screen, x0..x1)`.
    pub body_top: u16,
    pub tabs: Vec<(Screen, std::ops::Range<u16>)>,
    /// Items opened with `↵` (full text; a project — all completed tasks).
    pub open: HashSet<Item>,
    pub input: Option<InputLine>,
    pub scroll: usize,
    pub hooks_installed: bool,
    /// Timeline: lines scrolled from the top.
    pub timeline_scroll: usize,
    /// One project instead of all, on every screen (`f`).
    pub only: Option<ProjectId>,
    /// Mouse selection in progress or just copied (shown reversed).
    pub drag: Option<Selection>,
    /// How projects are listed; `s` toggles.
    pub order: Order,
    /// A newer release, old agent instructions: shown above the footer.
    pub notices: Vec<crate::update::Notice>,
    /// Screen row of the first notice, from the last draw (clicks copy its command).
    pub notice_top: u16,
}

impl App {
    pub fn new(mut board: Board, hooks_installed: bool) -> Self {
        board.sort(Order::default());
        App {
            board,
            screen: Screen::Projects,
            selected: None,
            items: Vec::new(),
            rows: Vec::new(),
            body_top: 1,
            tabs: Vec::new(),
            open: HashSet::new(),
            input: None,
            scroll: 0,
            hooks_installed,
            timeline_scroll: 0,
            only: None,
            drag: None,
            order: Order::default(),
            notices: Vec::new(),
            notice_top: u16::MAX,
        }
    }

    /// A fresh board from the logs, in the chosen order.
    pub fn set_board(&mut self, mut board: Board) {
        board.sort(self.order);
        self.board = board;
    }

    pub fn set_order(&mut self, order: Order) {
        self.order = order;
        self.board.sort(order);
    }

    /// After a layout: remember item order and keep the cursor on something real.
    pub fn set_items(&mut self, items: Vec<Item>) {
        if let Some(sel) = &self.selected
            && !items.contains(sel)
        {
            // A note was completed or deleted: fall back to its project.
            let project = Item::Project(sel.project().clone());
            self.selected = items.contains(&project).then_some(project);
        }
        if self.selected.is_none() {
            self.selected = items.first().cloned();
        }
        self.items = items;
    }

    /// The board as shown: one project when `f` narrowed it and that project
    /// is still on the board, else all of them.
    pub fn shown(&self) -> std::borrow::Cow<'_, Board> {
        match self
            .only
            .as_ref()
            .and_then(|id| self.board.projects.iter().find(|p| &p.meta.id == id))
        {
            Some(p) => std::borrow::Cow::Owned(Board {
                projects: vec![p.clone()],
            }),
            None => std::borrow::Cow::Borrowed(&self.board),
        }
    }

    /// `f`: only the selected project, or all again.
    fn toggle_only(&mut self) {
        self.only = match self.only {
            Some(_) => None,
            None => self.selected_project(),
        };
        self.timeline_scroll = 0;
        self.scroll = 0;
    }

    fn selected_project(&self) -> Option<ProjectId> {
        self.selected
            .as_ref()
            .map(|i| i.project().clone())
            .or_else(|| self.board.projects.first().map(|p| p.meta.id.clone()))
    }

    fn move_cursor(&mut self, delta: isize) {
        if self.items.is_empty() {
            return;
        }
        let pos = self
            .selected
            .as_ref()
            .and_then(|s| self.items.iter().position(|i| i == s))
            .unwrap_or(0);
        let next = pos.saturating_add_signed(delta).min(self.items.len() - 1);
        self.selected = Some(self.items[next].clone());
    }

    /// A note by id, open or done; `true` when it is done.
    fn note(&self, project: &ProjectId, id: &str) -> Option<(&deepdeck_core::Note, bool)> {
        let p = self.board.projects.iter().find(|p| &p.meta.id == project)?;
        p.notes
            .iter()
            .find(|n| n.id == id)
            .map(|n| (n, false))
            .or_else(|| p.done_notes.iter().find(|n| n.id == id).map(|n| (n, true)))
    }

    pub fn handle_key(&mut self, key: KeyEvent, now: Timestamp) -> Option<Effect> {
        self.drag = None;
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('c')
            && self.input.is_none()
        {
            return Some(Effect::Quit);
        }
        if let Some(input) = &mut self.input {
            return match input.handle(key) {
                InputOutcome::Continue => None,
                InputOutcome::Cancel => {
                    self.input = None;
                    None
                }
                InputOutcome::NextProject => {
                    let ids: Vec<_> = self.board.projects.iter().map(|p| &p.meta.id).collect();
                    if let Some(i) = ids.iter().position(|id| **id == input.project) {
                        input.project = ids[(i + 1) % ids.len()].clone();
                    }
                    None
                }
                InputOutcome::Submit => {
                    let line = self.input.take()?;
                    let text = line.text.trim().to_owned();
                    if text.is_empty() {
                        return None;
                    }
                    let body = match line.editing {
                        Some(id) => EventBody::NoteUpdate { id, text },
                        None => EventBody::NoteAdd {
                            id: note_id(now),
                            note_type: line.kind,
                            text,
                        },
                    };
                    Some(Effect::Write(line.project, vec![body]))
                }
            };
        }

        match key.code {
            KeyCode::Char('q') => return Some(Effect::Quit),
            KeyCode::Char('1') => self.screen = Screen::Projects,
            KeyCode::Char('2') => self.screen = Screen::Timeline,
            KeyCode::Char('3') => self.screen = Screen::Inbox,
            KeyCode::Tab | KeyCode::Right => {
                self.screen = match self.screen {
                    Screen::Projects => Screen::Timeline,
                    Screen::Timeline => Screen::Inbox,
                    Screen::Inbox => Screen::Projects,
                }
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.screen = match self.screen {
                    Screen::Projects => Screen::Inbox,
                    Screen::Timeline => Screen::Projects,
                    Screen::Inbox => Screen::Timeline,
                }
            }
            KeyCode::Char('f') => self.toggle_only(),
            KeyCode::Char('u') if !self.notices.is_empty() => {
                return Some(Effect::Copy(crate::update::command(&self.notices[0])));
            }
            _ if self.screen == Screen::Timeline => self.timeline_key(key),
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor(-1),
            KeyCode::Char('g') | KeyCode::Home => self.move_cursor(isize::MIN),
            KeyCode::Char('G') | KeyCode::End => self.move_cursor(isize::MAX),
            KeyCode::Char('o') => return self.focus_selected(),
            KeyCode::Char('s') => {
                self.set_order(match self.order {
                    Order::Name => Order::Attention,
                    Order::Attention => Order::Name,
                });
                return Some(Effect::SaveOrder(self.order));
            }
            KeyCode::Char('y') => {
                let item = self.selected.as_ref()?;
                let p = self
                    .board
                    .projects
                    .iter()
                    .find(|p| &p.meta.id == item.project())?;
                return super::projects::item_text(p, item, crate::i18n::t()).map(Effect::Copy);
            }
            KeyCode::Char(c @ ('a' | '?' | 'i')) => {
                let kind = match c {
                    'a' => NoteType::Task,
                    '?' => NoteType::Question,
                    _ => NoteType::Idea,
                };
                if let Some(project) = self.selected_project() {
                    self.input = Some(InputLine::new(kind, project));
                }
            }
            code => {
                let item = self.selected.clone()?;
                let toggle = code == KeyCode::Enter
                    || (code == KeyCode::Char(' ') && matches!(item, Item::Project(_)));
                if toggle {
                    if !self.open.remove(&item) {
                        self.open.insert(item);
                    }
                } else if let Item::Note(project, id) = item {
                    return self.note_key(code, project, id);
                }
            }
        }
        None
    }

    /// Click selects a line, a click on the selected line opens or closes it,
    /// the wheel moves the cursor (scrolls the Timeline), a click on a tab switches.
    pub fn handle_mouse(&mut self, ev: MouseEvent) -> Option<Effect> {
        if self.input.is_some() {
            return None;
        }
        match ev.kind {
            MouseEventKind::ScrollDown if self.screen == Screen::Timeline => {
                self.timeline_scroll += 3
            }
            MouseEventKind::ScrollUp if self.screen == Screen::Timeline => {
                self.timeline_scroll = self.timeline_scroll.saturating_sub(3)
            }
            MouseEventKind::ScrollDown => self.move_cursor(1),
            MouseEventKind::ScrollUp => self.move_cursor(-1),
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some((screen, _)) = self
                    .tabs
                    .iter()
                    .find(|(_, x)| ev.row == 0 && x.contains(&ev.column))
                {
                    self.screen = *screen;
                    self.drag = None;
                    return None;
                }
                // Click or the start of a selection: decided on release.
                self.drag = Some(Selection::at((ev.column, ev.row)));
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(sel) = &mut self.drag {
                    sel.head = (ev.column, ev.row);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                let sel = self.drag?;
                if !sel.is_empty() {
                    // Stays highlighted until the next click or key.
                    return Some(Effect::CopySelection(sel));
                }
                self.drag = None;
                if let Some(n) = ev
                    .row
                    .checked_sub(self.notice_top)
                    .and_then(|i| self.notices.get(usize::from(i)))
                {
                    return Some(Effect::Copy(crate::update::command(n)));
                }
                return self.click(ev.row);
            }
            _ => {}
        }
        None
    }

    /// A click on a line selects it; on the selected line, opens or closes it.
    fn click(&mut self, row: u16) -> Option<Effect> {
        if self.screen == Screen::Timeline || row < self.body_top {
            return None;
        }
        let line = usize::from(row - self.body_top) + self.scroll;
        // The item whose lines contain the click: the last one starting at or above it.
        let (item, _) = self
            .rows
            .iter()
            .rev()
            .find(|(_, start)| *start <= line)?
            .clone();
        if self.selected.as_ref() == Some(&item) {
            if !self.open.remove(&item) {
                self.open.insert(item);
            }
        } else {
            self.selected = Some(item);
        }
        None
    }

    /// The agent behind the selected line: a plan step's own session, otherwise
    /// the project's headline agent (the one waiting, if any).
    fn focus_selected(&self) -> Option<Effect> {
        let item = self.selected.as_ref()?;
        let p = self
            .board
            .projects
            .iter()
            .find(|p| &p.meta.id == item.project())?;
        let agent = match item {
            Item::Task(_, session, _) => p.agents.iter().find(|a| &a.session == session),
            _ => None,
        }
        .or_else(|| p.headline());
        Some(Effect::Focus {
            pane: agent.and_then(|a| a.herdr_pane.clone()),
            root: p.meta.root.clone(),
        })
    }

    fn note_key(&mut self, code: KeyCode, project: ProjectId, id: String) -> Option<Effect> {
        let inbox = self.screen == Screen::Inbox;
        match code {
            // In the inbox `d` means "resolved", as in mock-up C; on Projects it deletes.
            // Space toggles: an accidental "done" is undone by the same key.
            KeyCode::Char(' ') => {
                let (_, done) = self.note(&project, &id)?;
                let body = if done {
                    EventBody::NoteReopen { id }
                } else {
                    EventBody::NoteDone { id }
                };
                Some(Effect::Write(project, vec![body]))
            }
            KeyCode::Char('d') if inbox => {
                Some(Effect::Write(project, vec![EventBody::NoteDone { id }]))
            }
            KeyCode::Char('d') => Some(Effect::Write(project, vec![EventBody::NoteDelete { id }])),
            KeyCode::Char('e') => {
                let (note, _) = self.note(&project, &id)?;
                self.input = Some(InputLine::edit(
                    note.note_type,
                    project,
                    id,
                    &note.text.clone(),
                ));
                None
            }
            // Question or idea → a task with the same text.
            KeyCode::Char('m') => {
                let (note, _) = self.note(&project, &id)?;
                if note.note_type == NoteType::Task {
                    return None;
                }
                let text = note.text.clone();
                Some(Effect::Write(
                    project,
                    vec![
                        EventBody::NoteDelete { id: id.clone() },
                        EventBody::NoteAdd {
                            id: format!("{id}_t"),
                            note_type: NoteType::Task,
                            text,
                        },
                    ],
                ))
            }
            _ => None,
        }
    }

    fn timeline_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.timeline_scroll += 1,
            KeyCode::Char('k') | KeyCode::Up => {
                self.timeline_scroll = self.timeline_scroll.saturating_sub(1)
            }
            KeyCode::Char('g') | KeyCode::Home => self.timeline_scroll = 0,
            KeyCode::Char('G') | KeyCode::End => self.timeline_scroll = usize::MAX,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use deepdeck_core::{Event, ProjectMeta, Source, fold};

    use super::*;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn now() -> Timestamp {
        Timestamp::from_second(1_793_430_000).unwrap()
    }

    fn app() -> (App, ProjectId) {
        let meta = ProjectMeta::for_root(Path::new("/p/DeepVault"));
        let id = meta.id.clone();
        let note = Event::new(
            now(),
            Source::User,
            None,
            EventBody::NoteAdd {
                id: "n1".into(),
                note_type: NoteType::Idea,
                text: "csv".into(),
            },
        );
        let mut app = App::new(fold([(meta, vec![note])]), true);
        app.set_items(vec![
            Item::Project(id.clone()),
            Item::Note(id.clone(), "n1".into()),
        ]);
        (app, id)
    }

    #[test]
    fn add_idea_writes_note_add() {
        let (mut app, id) = app();
        assert_eq!(app.handle_key(key('i'), now()), None);
        for c in "экспорт".chars() {
            app.handle_key(key(c), now());
        }
        // 'q' inside the input is text, not quit.
        app.handle_key(key('q'), now());
        let e = app.handle_key(enter(), now());
        let add = EventBody::NoteAdd {
            id: note_id(now()),
            note_type: NoteType::Idea,
            text: "экспортq".into(),
        };
        assert_eq!(e, Some(Effect::Write(id, vec![add])));
        assert!(app.input.is_none());
    }

    fn enter() -> KeyEvent {
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
    }

    #[test]
    fn projects_stay_in_name_order_until_s() {
        let meta = |n: &str| ProjectMeta::for_root(Path::new(n));
        let waiting = Event::new(
            now(),
            Source::Claude,
            Some("s".into()),
            EventBody::AgentWaiting {
                text: "?".into(),
                reason: None,
            },
        );
        let board = fold([
            (meta("/p/alpha"), vec![]),
            (meta("/p/Beta"), vec![]),
            (meta("/p/zeta"), vec![waiting]),
        ]);
        let names = |app: &App| -> Vec<String> {
            app.board
                .projects
                .iter()
                .map(|p| p.meta.name.clone())
                .collect()
        };
        let mut app = App::new(board.clone(), true);
        assert_eq!(names(&app), ["alpha", "Beta", "zeta"]);
        // A new board from the logs keeps the order.
        app.set_board(board.clone());
        assert_eq!(names(&app), ["alpha", "Beta", "zeta"]);
        assert_eq!(
            app.handle_key(key('s'), now()),
            Some(Effect::SaveOrder(Order::Attention))
        );
        assert_eq!(names(&app)[0], "zeta");
        app.set_board(board);
        assert_eq!(names(&app)[0], "zeta");
        assert_eq!(
            app.handle_key(key('s'), now()),
            Some(Effect::SaveOrder(Order::Name))
        );
        assert_eq!(names(&app), ["alpha", "Beta", "zeta"]);
    }

    #[test]
    fn y_copies_the_full_text_of_the_line() {
        let (mut app, id) = app();
        app.selected = Some(Item::Note(id.clone(), "n1".into()));
        assert_eq!(
            app.handle_key(key('y'), now()),
            Some(Effect::Copy("csv".into()))
        );
        app.selected = Some(Item::Project(id));
        assert_eq!(
            app.handle_key(key('y'), now()),
            Some(Effect::Copy("/p/DeepVault".into()))
        );
        app.selected = None;
        assert_eq!(app.handle_key(key('y'), now()), None);
    }

    #[test]
    fn o_focuses_the_agent_behind_the_line() {
        let meta = ProjectMeta::for_root(Path::new("/p/DeepVault"));
        let id = meta.id.clone();
        let ev =
            |session: &str, body| Event::new(now(), Source::Claude, Some(session.into()), body);
        let started = |pane: Option<&str>| EventBody::AgentStarted {
            source: "startup".into(),
            herdr_pane: pane.map(Into::into),
        };
        let events = vec![
            ev("a", started(Some("wE:p1"))),
            ev("b", started(Some("wE:p2"))),
            ev(
                "b",
                EventBody::AgentWaiting {
                    text: "permission".into(),
                    reason: None,
                },
            ),
        ];
        let mut app = App::new(fold([(meta.clone(), events)]), true);
        let focus = |pane: &str| {
            Some(Effect::Focus {
                pane: Some(pane.into()),
                root: meta.root.clone(),
            })
        };
        // The project line goes to the agent waiting for you.
        app.set_items(vec![Item::Project(id.clone())]);
        assert_eq!(app.handle_key(key('o'), now()), focus("wE:p2"));
        // A plan step goes to its own session.
        app.selected = Some(Item::Task(id.clone(), "a".into(), "1".into()));
        assert_eq!(app.handle_key(key('o'), now()), focus("wE:p1"));
        // Nothing selected: nothing to do.
        app.selected = None;
        assert_eq!(app.handle_key(key('o'), now()), None);
    }

    #[test]
    fn empty_input_and_escape_write_nothing() {
        let (mut app, _) = app();
        app.handle_key(key('?'), now());
        assert_eq!(app.handle_key(enter(), now()), None);
        app.handle_key(key('a'), now());
        app.handle_key(key('x'), now());
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), now()),
            None
        );
        assert!(app.input.is_none());
    }

    #[test]
    fn note_actions_on_projects() {
        let (mut app, id) = app();
        app.handle_key(key('j'), now());
        let one = |b| Some(Effect::Write(id.clone(), vec![b]));
        assert_eq!(
            app.handle_key(key(' '), now()),
            one(EventBody::NoteDone { id: "n1".into() })
        );
        assert_eq!(
            app.handle_key(key('d'), now()),
            one(EventBody::NoteDelete { id: "n1".into() })
        );
        app.handle_key(key('e'), now());
        assert_eq!(app.input.as_ref().unwrap().text, "csv");
        app.handle_key(key('!'), now());
        assert_eq!(
            app.handle_key(enter(), now()),
            one(EventBody::NoteUpdate {
                id: "n1".into(),
                text: "csv!".into()
            })
        );
    }

    #[test]
    fn space_on_a_done_note_reopens_it() {
        let (mut app, id) = app();
        let note = Item::Note(id.clone(), "n1".into());
        app.selected = Some(note);
        let done = Event::new(
            now(),
            Source::User,
            None,
            EventBody::NoteDone { id: "n1".into() },
        );
        let add = Event::new(
            now(),
            Source::User,
            None,
            EventBody::NoteAdd {
                id: "n1".into(),
                note_type: NoteType::Idea,
                text: "csv".into(),
            },
        );
        app.board = fold([(
            ProjectMeta::for_root(Path::new("/p/DeepVault")),
            vec![add, done],
        )]);
        assert_eq!(
            app.handle_key(key(' '), now()),
            Some(Effect::Write(
                id,
                vec![EventBody::NoteReopen { id: "n1".into() }]
            ))
        );
    }

    #[test]
    fn enter_opens_and_closes_any_item() {
        let (mut app, id) = app();
        let task = Item::Task(id.clone(), "s".into(), "1".into());
        app.set_items(vec![
            Item::Project(id.clone()),
            task.clone(),
            Item::Note(id, "n1".into()),
        ]);
        app.handle_key(key('j'), now());
        app.handle_key(enter(), now());
        assert!(app.open.contains(&task));
        // Space and d on an agent's task do nothing: it is the agent's plan.
        assert_eq!(app.handle_key(key(' '), now()), None);
        assert_eq!(app.handle_key(key('d'), now()), None);
        app.handle_key(enter(), now());
        assert!(app.open.is_empty());
    }

    #[test]
    fn mouse_selects_opens_and_switches_tabs() {
        let (mut app, id) = app();
        let note = Item::Note(id.clone(), "n1".into());
        app.rows = vec![(Item::Project(id.clone()), 0), (note.clone(), 3)];
        app.tabs = vec![(Screen::Projects, 10..20), (Screen::Inbox, 30..38)];
        let ev = |kind, col, row| MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        };
        let down = |col, row| ev(MouseEventKind::Down(MouseButton::Left), col, row);
        let drag = |col, row| ev(MouseEventKind::Drag(MouseButton::Left), col, row);
        let up = |col, row| ev(MouseEventKind::Up(MouseButton::Left), col, row);
        let click = |app: &mut App, col, row| {
            app.handle_mouse(down(col, row));
            app.handle_mouse(up(col, row))
        };
        // Row 1 + 4 = content line 4: inside the note (starts at 3).
        click(&mut app, 5, 5);
        assert_eq!(app.selected, Some(note.clone()));
        click(&mut app, 5, 4);
        assert!(app.open.contains(&note));

        // A drag selects text instead: copied on release, nothing toggled.
        app.handle_mouse(down(2, 4));
        app.handle_mouse(drag(9, 5));
        let sel = Selection {
            anchor: (2, 4),
            head: (9, 5),
        };
        assert_eq!(app.handle_mouse(up(9, 5)), Some(Effect::CopySelection(sel)));
        assert!(app.open.contains(&note));
        assert_eq!(app.drag, Some(sel));
        // Any key clears the highlight.
        app.handle_key(key('j'), now());
        assert_eq!(app.drag, None);

        click(&mut app, 32, 0);
        assert_eq!(app.screen, Screen::Inbox);
        app.handle_mouse(ev(MouseEventKind::ScrollUp, 0, 3));
        assert_eq!(app.selected, Some(Item::Project(id)));
    }

    #[test]
    fn inbox_resolves_and_converts() {
        let (mut app, id) = app();
        app.handle_key(key('3'), now());
        app.selected = Some(Item::Note(id.clone(), "n1".into()));
        assert_eq!(
            app.handle_key(key('d'), now()),
            Some(Effect::Write(
                id.clone(),
                vec![EventBody::NoteDone { id: "n1".into() }]
            ))
        );
        assert_eq!(
            app.handle_key(key('m'), now()),
            Some(Effect::Write(
                id,
                vec![
                    EventBody::NoteDelete { id: "n1".into() },
                    EventBody::NoteAdd {
                        id: "n1_t".into(),
                        note_type: NoteType::Task,
                        text: "csv".into()
                    },
                ]
            ))
        );
    }

    #[test]
    fn arrows_switch_tabs_but_move_the_cursor_in_input() {
        let (mut app, _) = app();
        let arrow = |c| KeyEvent::new(c, KeyModifiers::NONE);
        app.handle_key(arrow(KeyCode::Right), now());
        assert_eq!(app.screen, Screen::Timeline);
        app.handle_key(arrow(KeyCode::Left), now());
        app.handle_key(arrow(KeyCode::Left), now());
        assert_eq!(app.screen, Screen::Inbox, "wraps around");
        app.handle_key(key('1'), now());
        app.handle_key(key('i'), now());
        app.handle_key(key('x'), now());
        app.handle_key(arrow(KeyCode::Left), now());
        assert_eq!(app.screen, Screen::Projects);
        assert_eq!(app.input.as_ref().unwrap().cursor, 0);
    }

    #[test]
    fn timeline_scrolls_and_filters() {
        let (mut app, id) = app();
        app.handle_key(key('2'), now());
        app.handle_key(key('j'), now());
        app.handle_key(key('j'), now());
        app.handle_key(key('k'), now());
        assert_eq!(app.timeline_scroll, 1);
        app.handle_key(key('f'), now());
        assert_eq!(app.only, Some(id));
        assert_eq!(app.timeline_scroll, 0);
        app.handle_key(key('f'), now());
        assert_eq!(app.only, None);
        // Notes keys do nothing on the timeline.
        assert_eq!(app.handle_key(key('d'), now()), None);
    }

    #[test]
    fn f_narrows_every_screen_to_one_project() {
        let a = ProjectMeta::for_root(Path::new("/p/DeepDeck"));
        let b = ProjectMeta::for_root(Path::new("/p/DeepVault"));
        let mut app = App::new(fold([(a.clone(), vec![]), (b.clone(), vec![])]), true);
        app.set_items(vec![
            Item::Project(a.id.clone()),
            Item::Project(b.id.clone()),
        ]);
        app.handle_key(key('j'), now());
        assert_eq!(app.shown().projects.len(), 2);
        app.handle_key(key('f'), now());
        assert_eq!(app.only, Some(b.id.clone()));
        let shown = app.shown();
        assert_eq!(shown.projects.len(), 1);
        assert_eq!(shown.projects[0].meta.id, b.id);
        drop(shown);
        // Same filter on the inbox; the project leaving the board shows all again.
        app.handle_key(key('3'), now());
        assert_eq!(app.shown().projects.len(), 1);
        app.set_board(fold([(a, vec![])]));
        assert_eq!(app.shown().projects.len(), 1);
        app.handle_key(key('f'), now());
        assert_eq!(app.only, None);
    }

    #[test]
    fn cursor_falls_back_to_project_when_note_disappears() {
        let (mut app, id) = app();
        app.handle_key(key('j'), now());
        app.set_items(vec![Item::Project(id.clone())]);
        assert_eq!(app.selected, Some(Item::Project(id)));
        assert_eq!(app.handle_key(key('q'), now()), Some(Effect::Quit));
    }

    #[test]
    fn note_ids_are_sortable() {
        let a = note_id(now());
        let b = note_id(now() + jiff::SignedDuration::from_nanos(1));
        assert!(a < b && a.starts_with("n_"));
    }
}
