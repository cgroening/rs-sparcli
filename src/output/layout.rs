//! Internal line-level layout helpers shared by output widgets.

use crate::core::geometry::Align;
use crate::core::style::Style;
use crate::core::text::{Line, Span};

/// Builds a line of `width` spaces with the given fill style.
pub(crate) fn blank_line(width: usize, fill: Style) -> Line {
    if width == 0 {
        return Line::default();
    }
    Line::new(vec![Span::styled(" ".repeat(width), fill)])
}

/// Pads `line` to exactly `width` columns according to `align`.
///
/// Lines already at or beyond `width` are returned unchanged.
pub(crate) fn pad_line(
    line: Line,
    width: usize,
    align: Align,
    fill: Style,
) -> Line {
    let current = line.width();
    if current >= width {
        return line;
    }
    let total = width - current;
    let (left, right) = match align {
        Align::Left => (0, total),
        Align::Right => (total, 0),
        Align::Center => (total / 2, total - total / 2),
    };
    let mut spans = Vec::with_capacity(line.spans.len() + 2);
    if left > 0 {
        spans.push(Span::styled(" ".repeat(left), fill));
    }
    spans.extend(line.spans);
    if right > 0 {
        spans.push(Span::styled(" ".repeat(right), fill));
    }
    Line::new(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_line_left_aligns_by_default() {
        let line = pad_line(Line::raw("hi"), 5, Align::Left, Style::new());
        assert_eq!(line.plain(), "hi   ");
    }

    #[test]
    fn pad_line_centers_content() {
        let line = pad_line(Line::raw("hi"), 6, Align::Center, Style::new());
        assert_eq!(line.plain(), "  hi  ");
    }

    #[test]
    fn pad_line_right_aligns_content() {
        let line = pad_line(Line::raw("hi"), 5, Align::Right, Style::new());
        assert_eq!(line.plain(), "   hi");
    }
}
