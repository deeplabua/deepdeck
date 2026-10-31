//! Display-width helpers: the panel is narrow and Cyrillic/symbols must not
//! overflow. Long text is cut with `…`, never wrapped.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Columns a row's main text keeps before the right-hand part is cut.
pub const MIN_TEXT: usize = 8;

pub fn width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Cut `s` to at most `max` columns, ending with `…` when cut.
pub fn fit(s: &str, max: usize) -> String {
    if width(s) <= max {
        return s.to_owned();
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = c.width().unwrap_or(0);
        if w + cw > max - 1 {
            break;
        }
        out.push(c);
        w += cw;
    }
    out.truncate(out.trim_end().len());
    out.push('…');
    out
}

/// One row: `prefix` spans (fixed), `text` cut to what is left, then `right`
/// pushed to the right edge. Never wider than `total`: when space runs out the
/// text keeps at least [`MIN_TEXT`] columns and `right` is cut instead.
pub fn row(
    prefix: Vec<Span<'static>>,
    text: &str,
    text_style: Style,
    right: Option<Span<'static>>,
    total: usize,
) -> Line<'static> {
    let used: usize = prefix.iter().map(|s| width(&s.content)).sum();
    let keep = width(text).min(MIN_TEXT);
    let right = right.map(|r| {
        let allowed = total.saturating_sub(used + keep + 1);
        if width(&r.content) > allowed {
            Span::styled(fit(&r.content, allowed), r.style)
        } else {
            r
        }
    });
    let right_w = right.as_ref().map_or(0, |r| width(&r.content) + 1);
    let room = total.saturating_sub(used + right_w);
    let text = fit(text, room);
    let gap = room.saturating_sub(width(&text));
    let mut spans = prefix;
    spans.push(Span::styled(text, text_style));
    if let Some(r) = right {
        spans.push(Span::raw(" ".repeat(gap + 1)));
        spans.push(r);
    }
    Line::from(spans)
}

/// Word-wrap `s` into lines of at most `max` columns; words longer than a
/// line are split. Whitespace runs collapse to one space.
pub fn wrap(s: &str, max: usize) -> Vec<String> {
    let max = max.max(1);
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let mut word = word;
        loop {
            let need = if cur.is_empty() {
                width(word)
            } else {
                width(&cur) + 1 + width(word)
            };
            if need <= max {
                if !cur.is_empty() {
                    cur.push(' ');
                }
                cur.push_str(word);
                break;
            }
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
                continue;
            }
            // A single word wider than the line: cut it.
            let mut w = 0;
            let cut = word
                .char_indices()
                .find(|(_, c)| {
                    w += c.width().unwrap_or(0);
                    w > max
                })
                .map_or(word.len(), |(i, _)| {
                    i.max(word.chars().next().map_or(1, char::len_utf8))
                });
            lines.push(word[..cut].to_owned());
            word = &word[cut..];
            if word.is_empty() {
                break;
            }
        }
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Plain text of a line (tests).
#[cfg(test)]
pub fn plain(line: &Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_counts_columns() {
        assert_eq!(fit("привет", 6), "привет");
        assert_eq!(fit("привет мир", 7), "привет…");
        assert_eq!(fit("日本語テキスト", 7), "日本語…");
        assert_eq!(width(&fit("日本語テキスト", 7)), 7);
        assert_eq!(fit("abc", 0), "");
    }

    #[test]
    fn wrap_by_columns() {
        assert_eq!(wrap("one two three", 7), ["one two", "three"]);
        assert_eq!(wrap("абвгдеёжз ok", 4), ["абвг", "деёж", "з ok"]);
        assert_eq!(wrap("日本語テキスト", 6), ["日本語", "テキス", "ト"]);
        assert_eq!(wrap("", 10), [""]);
        for l in wrap("UI прогресса импорта с очень длинным названием задачи", 12)
        {
            assert!(width(&l) <= 12, "{l}");
        }
    }

    #[test]
    fn row_is_never_wider_than_total() {
        for total in [10, 30, 60] {
            let l = row(
                vec![Span::raw("  ● ")],
                "очень длинное имя проекта, которое не влезает",
                Style::new(),
                Some(Span::raw("waiting · 2m")),
                total,
            );
            assert!(width(&plain(&l)) <= total, "{total}: {}", plain(&l));
        }
        let l = row(
            vec![Span::raw(" ")],
            "a",
            Style::new(),
            Some(Span::raw("1/2")),
            10,
        );
        assert_eq!(plain(&l), " a     1/2");
    }
}
