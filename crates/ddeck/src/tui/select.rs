//! Mouse selection inside the board. The TUI captures the mouse (clicks,
//! wheel), so the terminal cannot select text itself: dragging selects here,
//! the selection is shown reversed and copied on release.
//!
//! Like a terminal: from where the drag began to where it is now, in reading
//! order — whole rows in between.

use ratatui::buffer::Buffer;
use ratatui::style::Modifier;
use unicode_width::UnicodeWidthStr;

/// Screen cell: (column, row).
pub type Pos = (u16, u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Where the button went down.
    pub anchor: Pos,
    /// Where the pointer is now.
    pub head: Pos,
}

impl Selection {
    pub fn at(pos: Pos) -> Self {
        Selection {
            anchor: pos,
            head: pos,
        }
    }

    /// A press without a drag is a click, not a selection.
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// (first, last) in reading order, both inclusive.
    fn ordered(&self) -> (Pos, Pos) {
        let key = |p: Pos| (p.1, p.0);
        if key(self.anchor) <= key(self.head) {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }

    /// Columns selected on `row` within `width`, as `start..end`.
    fn columns(&self, row: u16, width: u16) -> Option<std::ops::Range<u16>> {
        let (first, last) = self.ordered();
        if row < first.1 || row > last.1 {
            return None;
        }
        let start = if row == first.1 { first.0 } else { 0 };
        let end = if row == last.1 {
            last.0.saturating_add(1)
        } else {
            width
        };
        Some(start.min(width)..end.min(width))
    }

    #[cfg(test)]
    pub fn contains(&self, col: u16, row: u16, width: u16) -> bool {
        self.columns(row, width).is_some_and(|r| r.contains(&col))
    }

    /// The selected text: trailing spaces trimmed per row, rows joined by `\n`.
    pub fn text(&self, buf: &Buffer) -> String {
        let area = buf.area;
        let (first, last) = self.ordered();
        let mut lines = Vec::new();
        for row in first.1..=last.1.min(area.bottom().saturating_sub(1)) {
            let Some(cols) = self.columns(row, area.width) else {
                continue;
            };
            let mut line = String::new();
            let mut skip = 0;
            for col in 0..cols.end {
                let symbol = buf[(area.x + col, row)].symbol();
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                // A wide character covers the next cell(s) too.
                skip = symbol.width().saturating_sub(1);
                if col >= cols.start {
                    line.push_str(symbol);
                }
            }
            lines.push(line.trim_end().to_owned());
        }
        lines.join("\n")
    }

    /// Show the selection reversed.
    pub fn highlight(&self, buf: &mut Buffer) {
        let area = buf.area;
        for row in area.top()..area.bottom() {
            if let Some(cols) = self.columns(row, area.width) {
                for col in cols {
                    let cell = &mut buf[(area.x + col, row)];
                    cell.modifier.insert(Modifier::REVERSED);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::*;

    fn screen(rows: &[&str]) -> Buffer {
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, rows.len() as u16));
        for (y, r) in rows.iter().enumerate() {
            buf.set_string(0, y as u16, r, ratatui::style::Style::new());
        }
        buf
    }

    #[test]
    fn selects_like_a_terminal() {
        let buf = screen(&["$ check  ok", "cargo test", "третья"]);
        // Within one row, either direction.
        let sel = Selection {
            anchor: (2, 0),
            head: (6, 0),
        };
        assert_eq!(sel.text(&buf), "check");
        let back = Selection {
            anchor: (6, 0),
            head: (2, 0),
        };
        assert_eq!(back.text(&buf), "check");
        // Across rows: rest of the first, whole middle, start of the last.
        let multi = Selection {
            anchor: (9, 0),
            head: (3, 2),
        };
        assert_eq!(multi.text(&buf), "ok\ncargo test\nтрет");
        assert!(multi.contains(0, 1, 12));
        assert!(!multi.contains(8, 0, 12));
        assert!(!multi.contains(4, 2, 12));
        assert!(Selection::at((1, 1)).is_empty());
    }

    #[test]
    fn wide_characters_are_whole() {
        let buf = screen(&["日本語 ok"]);
        let sel = Selection {
            anchor: (0, 0),
            head: (5, 0),
        };
        assert_eq!(sel.text(&buf), "日本語");
    }

    #[test]
    fn highlight_reverses_only_the_selection() {
        let mut buf = screen(&["abcdef", "ghijkl"]);
        Selection {
            anchor: (4, 0),
            head: (1, 1),
        }
        .highlight(&mut buf);
        let rev = |x, y| buf[(x, y)].modifier.contains(Modifier::REVERSED);
        assert!(!rev(3, 0) && rev(4, 0) && rev(11, 0) && rev(0, 1) && rev(1, 1) && !rev(2, 1));
    }
}
