//! The search screen (`/`): `ddeck find` inside the board. Type words, `↵` to
//! the results, then `↵` opens the project, `o` jumps to the agent, `y` copies
//! the command that resumes the session.

use deepdeck_core::search;
use deepdeck_core::{Event, NoteType, ProjectMeta};
use jiff::tz::TimeZone;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use super::input::{InputLine, InputOutcome};
use super::projects::gutter;
use super::strings::Strings;
use super::text::{fit, row};
use super::theme::Theme;
use crate::find::Group;

/// Matches shown under each session.
const HITS: usize = 3;
/// Characters typed before searching starts.
const MIN_CHARS: usize = 2;

pub struct Search {
    pub line: InputLine,
    /// Cursor in the query line; `false` — in the results.
    pub typing: bool,
    /// Every project's events, archive included, read when the screen opened.
    pub corpus: Option<Vec<(ProjectMeta, Vec<Event>)>>,
    pub groups: Vec<Group>,
    pub terms: Vec<String>,
    /// The query `groups` were found for.
    found_for: Option<String>,
    pub selected: usize,
    pub scroll: usize,
}

/// What a key asks the board to do.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Stay,
    Close,
    /// Close and select this project on the Projects screen.
    Open(usize),
    /// Switch to this group's agent.
    Focus(usize),
    /// Copy this group's resume command (or the text of a board note).
    Copy(usize),
}

impl Search {
    pub fn new() -> Self {
        Search {
            line: InputLine::new(NoteType::Idea, deepdeck_core::ProjectId(String::new())),
            typing: true,
            corpus: None,
            groups: Vec::new(),
            terms: Vec::new(),
            found_for: None,
            selected: 0,
            scroll: 0,
        }
    }

    /// Search again if the query changed since the last time (and the logs are read).
    pub fn refresh(&mut self) {
        let Some(corpus) = &self.corpus else {
            return;
        };
        if self.found_for.as_deref() == Some(self.line.text.as_str()) {
            return;
        }
        self.found_for = Some(self.line.text.clone());
        self.selected = 0;
        self.scroll = 0;
        let typed = self
            .line
            .text
            .chars()
            .filter(|c| !c.is_whitespace())
            .count();
        self.terms = if typed < MIN_CHARS {
            Vec::new()
        } else {
            self.line
                .text
                .split_whitespace()
                .map(search::stem)
                .filter(|t| !t.is_empty())
                .collect()
        };
        self.groups = if self.terms.is_empty() {
            Vec::new()
        } else {
            let mut g: Vec<Group> = corpus
                .iter()
                .flat_map(|(meta, events)| crate::find::group(meta, events, &self.terms))
                .collect();
            g.sort_by_key(|g| std::cmp::Reverse(g.hits[0].ts));
            g
        };
    }

    pub fn handle(&mut self, key: KeyEvent) -> Outcome {
        if self.typing {
            return match key.code {
                KeyCode::Esc => Outcome::Close,
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    self.refresh();
                    if !self.groups.is_empty() {
                        self.typing = false;
                    }
                    Outcome::Stay
                }
                _ => {
                    if let InputOutcome::Cancel = self.line.handle(key) {
                        return Outcome::Close;
                    }
                    Outcome::Stay
                }
            };
        }
        let last = self.groups.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Outcome::Close,
            KeyCode::Char('/') | KeyCode::Backspace => self.typing = true,
            KeyCode::Char('k') | KeyCode::Up if self.selected == 0 => self.typing = true,
            KeyCode::Char('j') | KeyCode::Down => self.selected = (self.selected + 1).min(last),
            KeyCode::Char('k') | KeyCode::Up => self.selected -= 1,
            KeyCode::Char('g') | KeyCode::Home => self.selected = 0,
            KeyCode::Char('G') | KeyCode::End => self.selected = last,
            KeyCode::Enter => return Outcome::Open(self.selected),
            KeyCode::Char('o') => return Outcome::Focus(self.selected),
            KeyCode::Char('y') => return Outcome::Copy(self.selected),
            _ => {}
        }
        Outcome::Stay
    }
}

/// What `y` copies: the command that reopens the session in its project,
/// or, for board notes, the newest matching text.
pub fn copy_text(g: &Group) -> String {
    match &g.session {
        Some(s) => {
            let root = g.meta.root.display().to_string();
            let root = if root.contains([' ', '\'', '"', '$', '&', '(', ')']) {
                format!("'{}'", root.replace('\'', r"'\''"))
            } else {
                root
            };
            format!("cd {root} && claude --resume {s}")
        }
        None => g.hits[0].text.clone(),
    }
}

fn mark(kind: search::HitKind, th: &Theme) -> (&'static str, ratatui::style::Style) {
    use search::HitKind as K;
    match kind {
        K::Prompt => ("›", th.name()),
        K::Reply => ("✓", th.done()),
        K::Summary => ("Σ", th.accent()),
        K::Note(NoteType::Question) => ("?", th.question()),
        K::Note(NoteType::Idea) => ("◆", th.idea()),
        K::Note(NoteType::Handoff) => ("→", th.waiting()),
        K::Note(NoteType::Task) => ("○", th.dim()),
        K::Plan => ("◐", th.working()),
        K::Roadmap => ("◇", th.accent()),
        K::Command => ("$", th.accent()),
    }
}

/// The screen: the query line, a count, then each session with its newest
/// matches. Returns the lines and the row each group starts at.
pub fn layout(
    s: &Search,
    width: u16,
    tz: &TimeZone,
    t: &Strings,
    th: &Theme,
) -> (Vec<Line<'static>>, Vec<usize>) {
    let width = usize::from(width);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" / ", th.accent()),
            Span::raw(fit(&s.line.text, width.saturating_sub(3))),
        ]),
        Line::default(),
    ];
    let status = if s.corpus.is_none() || s.terms.is_empty() {
        t.search_start.to_owned()
    } else if s.groups.is_empty() {
        t.search_none.to_owned()
    } else {
        let total = s.groups.iter().map(|g| g.hits.len()).sum();
        (t.search_found)(total, s.groups.len())
    };
    lines.push(Line::styled(
        fit(&format!(" {status}"), width),
        th.dim().add_modifier(Modifier::BOLD),
    ));
    let mut starts = Vec::new();
    for (i, g) in s.groups.iter().enumerate() {
        lines.push(Line::default());
        starts.push(lines.len());
        let sel = !s.typing && i == s.selected;
        let who = match &g.session {
            Some(id) => id.chars().take(8).collect(),
            None => t.search_board.to_owned(),
        };
        let when = g.hits[0]
            .ts
            .to_zoned(tz.clone())
            .strftime("%m-%d %H:%M")
            .to_string();
        let right = if g.ended {
            format!("{when} · {}", t.search_ended)
        } else {
            when
        };
        lines.push(row(
            vec![gutter(sel, th), Span::raw(" ")],
            &format!("{} · {who}", g.meta.name),
            th.name(),
            Some(Span::styled(right, th.dim())),
            width,
        ));
        for h in g.hits.iter().take(HITS) {
            let (m, style) = mark(h.kind, th);
            let room = width.saturating_sub(5);
            lines.push(row(
                vec![Span::raw("   "), Span::styled(format!("{m} "), style)],
                &search::snippet(&h.text, &s.terms, room),
                ratatui::style::Style::new(),
                None,
                width,
            ));
        }
        if g.hits.len() > HITS {
            lines.push(Line::styled(
                format!("     {}", (t.more)(g.hits.len() - HITS)),
                th.dim(),
            ));
        }
    }
    (lines, starts)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use deepdeck_core::{EventBody, Source};
    use jiff::Timestamp;
    use ratatui::crossterm::event::KeyModifiers;

    use super::super::strings::EN;
    use super::super::text::{plain, width};
    use super::*;

    fn at(min: i64) -> Timestamp {
        Timestamp::from_second(1_793_430_000 + min * 60).unwrap()
    }

    fn ev(min: i64, session: &str, body: EventBody) -> Event {
        Event::new(at(min), Source::Claude, Some(session.into()), body)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn corpus() -> Vec<(ProjectMeta, Vec<Event>)> {
        let prompt =
            |min, s: &str, text: &str| ev(min, s, EventBody::UserPrompt { text: text.into() });
        vec![
            (
                ProjectMeta::for_root(Path::new("/p/DeepShrink")),
                vec![
                    prompt(0, "76baef01-aaaa", "which keywords did users find us by?"),
                    ev(
                        1,
                        "76baef01-aaaa",
                        EventBody::AgentStopped {
                            recap: Some("Replaced the en-US keywords".into()),
                        },
                    ),
                    ev(2, "76baef01-aaaa", EventBody::AgentEnded { reason: None }),
                ],
            ),
            (
                ProjectMeta::for_root(Path::new("/p/DeepVault")),
                vec![prompt(5, "c0ffee00-bbbb", "keywords for the vault listing")],
            ),
        ]
    }

    #[test]
    fn type_then_pick_a_session() {
        let mut s = Search::new();
        s.corpus = Some(corpus());
        for c in "k".chars() {
            assert_eq!(s.handle(key(KeyCode::Char(c))), Outcome::Stay);
        }
        s.refresh();
        assert!(s.groups.is_empty(), "one letter is not a search yet");
        for c in "eywords".chars() {
            s.handle(key(KeyCode::Char(c)));
        }
        // Letters go to the query while typing, even o and y.
        assert!(s.typing);
        s.refresh();
        assert_eq!(s.groups.len(), 2);

        let (lines, starts) = layout(&s, 50, &TimeZone::UTC, &EN, &Theme { color: false });
        let text: Vec<String> = lines.iter().map(plain).collect();
        assert_eq!(
            text,
            [
                " / keywords",
                "",
                " 3 matches in 2 places",
                "",
                "  DeepVault · c0ffee00                 10-31 07:05",
                "   › keywords for the vault listing",
                "",
                "  DeepShrink · 76baef01        10-31 07:01 · ended",
                "   ✓ Replaced the en-US keywords",
                "   › which keywords did users find us by?",
            ]
        );
        assert_eq!(starts, [4, 7]);
        assert!(text.iter().all(|l| width(l) <= 50));

        assert_eq!(s.handle(key(KeyCode::Enter)), Outcome::Stay);
        assert!(!s.typing);
        assert_eq!(s.handle(key(KeyCode::Char('j'))), Outcome::Stay);
        assert_eq!(s.handle(key(KeyCode::Char('y'))), Outcome::Copy(1));
        assert_eq!(
            copy_text(&s.groups[1]),
            "cd /p/DeepShrink && claude --resume 76baef01-aaaa"
        );
        assert_eq!(s.handle(key(KeyCode::Char('o'))), Outcome::Focus(1));
        assert_eq!(s.handle(key(KeyCode::Enter)), Outcome::Open(1));
        // Up from the first result goes back to the query.
        s.handle(key(KeyCode::Char('k')));
        s.handle(key(KeyCode::Char('k')));
        assert!(s.typing);
        assert_eq!(s.handle(key(KeyCode::Esc)), Outcome::Close);
    }

    #[test]
    fn paths_with_spaces_are_quoted() {
        let mut g = crate::find::group(
            &ProjectMeta::for_root(Path::new("/Users/me/My Projects/App")),
            &[ev(
                0,
                "s1",
                EventBody::UserPrompt {
                    text: "keywords".into(),
                },
            )],
            &[search::stem("keywords")],
        );
        assert_eq!(
            copy_text(&g.remove(0)),
            "cd '/Users/me/My Projects/App' && claude --resume s1"
        );
    }
}
