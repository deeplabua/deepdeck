//! The Inbox screen (mock-up C): everything that needs your decision, across
//! projects — agents waiting for you, your part (handoffs), open questions,
//! ideas for later.

use deepdeck_core::{AgentState, Board, NoteType};
use jiff::Timestamp;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use ratatui::style::Style;

use super::projects::{Item, Rendered, View, gutter, item_lines, run_line};
use super::strings::Strings;
use super::text::row;
use super::theme::Theme;
use super::timeline::project_style;

/// Items in the inbox: waiting agents + handoffs + open questions (ideas are not "waiting").
pub fn inbox_count(board: &Board) -> usize {
    let questions = board
        .projects
        .iter()
        .flat_map(|p| &p.notes)
        .filter(|n| matches!(n.note_type, NoteType::Question | NoteType::Handoff))
        .count();
    board.waiting_count() + questions
}

pub fn layout_inbox(
    board: &Board,
    now: Timestamp,
    width: u16,
    view: &View<'_>,
    t: &Strings,
    th: &Theme,
) -> Rendered {
    let width = usize::from(width);
    let mut r = Rendered::default();
    let selected = view.selected;
    let header = |r: &mut Rendered, title: &str, n: usize| {
        if !r.lines.is_empty() {
            r.lines.push(Line::default());
        }
        r.lines.push(Line::styled(
            format!(" {title} · {n}"),
            th.dim().add_modifier(Modifier::BOLD),
        ));
    };

    // ── waiting agents ──
    let waiting: Vec<_> = board
        .projects
        .iter()
        .filter_map(|p| match &p.headline()?.state {
            AgentState::Waiting { since, text } => Some((p, *since, text)),
            _ => None,
        })
        .collect();
    if !waiting.is_empty() {
        header(&mut r, t.ib_waiting, waiting.len());
        for (p, since, text) in waiting {
            let item = Item::Project(p.meta.id.clone());
            let sel = selected == Some(&item);
            r.items.push((item, r.lines.len()));
            let ago = (t.ago)(now.duration_since(since).as_secs());
            r.lines.push(row(
                vec![gutter(sel, th), Span::styled("● ", th.waiting())],
                &p.meta.name,
                th.name(),
                Some(Span::styled(ago, th.waiting())),
                width,
            ));
            item_lines(
                &mut r,
                Item::Waiting(p.meta.id.clone()),
                view,
                th,
                vec![Span::raw("  ")],
                text,
                th.waiting(),
                None,
                width,
            );
        }
    }

    // ── notes by type ──
    for (note_type, title, mark, style) in [
        (NoteType::Handoff, t.ib_handoffs, "→ ", th.waiting()),
        (NoteType::Question, t.ib_questions, "? ", th.question()),
        (NoteType::Idea, t.ib_ideas, "◆ ", th.idea()),
    ] {
        let notes: Vec<_> = board
            .projects
            .iter()
            .flat_map(|p| {
                p.notes
                    .iter()
                    .filter(move |n| n.note_type == note_type)
                    .map(move |n| (p, n))
            })
            .collect();
        if notes.is_empty() {
            continue;
        }
        header(&mut r, title, notes.len());
        for (p, n) in notes {
            item_lines(
                &mut r,
                Item::Note(p.meta.id.clone(), n.id.clone()),
                view,
                th,
                vec![Span::raw(" "), Span::styled(mark, style)],
                &n.text,
                Style::new(),
                Some(Span::styled(
                    p.meta.name.clone(),
                    project_style(&p.meta.id, th),
                )),
                width,
            );
            if let Some(run) = &n.run {
                run_line(&mut r, 4, run, th, width);
            }
        }
    }

    if r.lines.is_empty() {
        r.lines.push(Line::default());
        r.lines
            .push(Line::styled(format!(" {}", t.ib_empty), th.dim()));
    }
    r
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use deepdeck_core::{Event, EventBody, ProjectMeta, Source, fold};

    use super::super::strings::EN;
    use super::super::text::{plain, width};
    use super::*;

    fn at(min: i64) -> Timestamp {
        Timestamp::from_second(1_793_430_000 + min * 60).unwrap()
    }

    #[test]
    fn sections_across_projects() {
        let note = |id: &str, ty, text: &str| {
            Event::new(
                at(0),
                Source::User,
                None,
                EventBody::NoteAdd {
                    id: id.into(),
                    note_type: ty,
                    text: text.into(),
                    run: None,
                },
            )
        };
        let vault = vec![
            Event::new(
                at(1),
                Source::Claude,
                Some("s".into()),
                EventBody::AgentWaiting {
                    text: "Claude needs your permission".into(),
                    reason: None,
                },
            ),
            note("q1", NoteType::Question, "Bitwarden JSON too?"),
            note("i1", NoteType::Idea, "CSV export"),
        ];
        let snap = vec![
            note("q2", NoteType::Question, "Ship tray fix in 0.4?"),
            note("t1", NoteType::Task, "not in inbox"),
        ];
        let board = fold([
            (ProjectMeta::for_root(Path::new("/p/DeepVault")), vault),
            (ProjectMeta::for_root(Path::new("/p/DeepSnap")), snap),
        ]);
        let r = layout_inbox(
            &board,
            at(4),
            50,
            &View {
                selected: None,
                open: &Default::default(),
            },
            &EN,
            &Theme { color: false },
        );
        let lines: Vec<String> = r.lines.iter().map(plain).collect();
        assert_eq!(
            lines,
            [
                " WAITING FOR YOU · 1",
                " ● DeepVault                                    3m",
                "   Claude needs your permission",
                "",
                " OPEN QUESTIONS · 2",
                "  ? Bitwarden JSON too?                  DeepVault",
                "  ? Ship tray fix in 0.4?                 DeepSnap",
                "",
                " IDEAS FOR LATER · 1",
                "  ◆ CSV export                           DeepVault",
            ]
        );
        assert!(lines.iter().all(|l| width(l) <= 50));
        assert_eq!(r.items.len(), 5);
        assert_eq!(inbox_count(&board), 3);
    }

    #[test]
    fn handoffs_come_first_with_their_command() {
        let add = |id: &str, ty, text: &str, run: Option<&str>| {
            Event::new(
                at(0),
                Source::Agent,
                None,
                EventBody::NoteAdd {
                    id: id.into(),
                    note_type: ty,
                    text: text.into(),
                    run: run.map(Into::into),
                },
            )
        };
        let board = fold([(
            ProjectMeta::for_root(Path::new("/p/DeepDeck")),
            vec![
                add("q1", NoteType::Question, "Ship 0.6?", None),
                add(
                    "h1",
                    NoteType::Handoff,
                    "Allow board commands",
                    Some("sh allow.sh"),
                ),
                add("h2", NoteType::Handoff, "Check on the phone", None),
            ],
        )]);
        let r = layout_inbox(
            &board,
            at(1),
            50,
            &View {
                selected: None,
                open: &Default::default(),
            },
            &EN,
            &Theme { color: false },
        );
        let lines: Vec<String> = r.lines.iter().map(plain).collect();
        assert_eq!(
            lines,
            [
                " YOUR PART · 2",
                "  → Allow board commands                  DeepDeck",
                "    $ sh allow.sh",
                "  → Check on the phone                    DeepDeck",
                "",
                " OPEN QUESTIONS · 1",
                "  ? Ship 0.6?                             DeepDeck",
            ]
        );
        assert_eq!(r.copy_rows, vec![(2, "sh allow.sh".to_owned())]);
        assert_eq!(inbox_count(&board), 3);
    }

    #[test]
    fn empty_inbox() {
        let r = layout_inbox(
            &Board::default(),
            at(0),
            40,
            &View {
                selected: None,
                open: &Default::default(),
            },
            &EN,
            &Theme { color: false },
        );
        assert_eq!(plain(&r.lines[1]), format!(" {}", EN.ib_empty));
        assert!(r.items.is_empty());
    }
}
