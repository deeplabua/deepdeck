//! The one-line editor at the bottom (mock-up C): add or edit a note.

use deepdeck_core::{NoteType, ProjectId};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub struct InputLine {
    pub kind: NoteType,
    pub project: ProjectId,
    /// `Some(note id)` when editing an existing note.
    pub editing: Option<String>,
    pub text: String,
    /// Cursor position in chars.
    pub cursor: usize,
}

pub enum InputOutcome {
    Continue,
    Cancel,
    Submit,
    NextProject,
}

impl InputLine {
    pub fn new(kind: NoteType, project: ProjectId) -> Self {
        InputLine {
            kind,
            project,
            editing: None,
            text: String::new(),
            cursor: 0,
        }
    }

    pub fn edit(kind: NoteType, project: ProjectId, id: String, text: &str) -> Self {
        InputLine {
            kind,
            project,
            editing: Some(id),
            text: text.to_owned(),
            cursor: text.chars().count(),
        }
    }

    fn byte_at(&self, char_idx: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_idx)
            .map_or(self.text.len(), |(i, _)| i)
    }

    pub fn handle(&mut self, key: KeyEvent) -> InputOutcome {
        let len = self.text.chars().count();
        match key.code {
            KeyCode::Esc => return InputOutcome::Cancel,
            KeyCode::Enter => return InputOutcome::Submit,
            KeyCode::Tab if self.editing.is_none() => return InputOutcome::NextProject,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return InputOutcome::Cancel;
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.text.clear();
                self.cursor = 0;
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => self.cursor = 0,
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cursor = len
            }
            KeyCode::Char(c) => {
                let at = self.byte_at(self.cursor);
                self.text.insert(at, c);
                self.cursor += 1;
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                let at = self.byte_at(self.cursor);
                self.text.remove(at);
            }
            KeyCode::Delete if self.cursor < len => {
                let at = self.byte_at(self.cursor);
                self.text.remove(at);
            }
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(len),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = len,
            _ => {}
        }
        InputOutcome::Continue
    }

    /// Text before the cursor (to place the terminal cursor by display width).
    pub fn before_cursor(&self) -> &str {
        &self.text[..self.byte_at(self.cursor)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn edits_cyrillic_by_chars() {
        let mut l = InputLine::new(NoteType::Idea, ProjectId("p".into()));
        for c in "идея".chars() {
            l.handle(key(KeyCode::Char(c)));
        }
        l.handle(key(KeyCode::Left));
        l.handle(key(KeyCode::Backspace));
        assert_eq!(l.text, "идя");
        assert_eq!(l.before_cursor(), "ид");
        l.handle(key(KeyCode::Home));
        l.handle(key(KeyCode::Delete));
        assert_eq!(l.text, "дя");
        assert!(matches!(
            l.handle(key(KeyCode::Enter)),
            InputOutcome::Submit
        ));
    }
}
