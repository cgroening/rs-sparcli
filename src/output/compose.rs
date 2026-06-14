//! Composition helpers: align, pad and vertically stack rendered blocks.
//!
//! These operate on [`Rendered`] values, the common currency of all output
//! widgets, so any widget output can be aligned, padded or stacked.

use crate::core::geometry::{Align, Edges};
use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::output::layout::{blank_line, pad_line};

/// Aligns each line of `block` within `width` columns.
pub fn align(block: &Rendered, width: u16, how: Align) -> Rendered {
    let width = width as usize;
    let lines = block
        .lines
        .iter()
        .cloned()
        .map(|line| pad_line(line, width, how, Style::new()))
        .collect();
    Rendered::new(lines)
}

/// Surrounds `block` with the given padding (blank rows and space columns).
pub fn pad(block: &Rendered, edges: Edges) -> Rendered {
    let content_width = block.width();
    let inner_width = content_width + edges.horizontal() as usize;
    let mut lines = Vec::new();
    for _ in 0..edges.top {
        lines.push(blank_line(inner_width, Style::new()));
    }
    for line in &block.lines {
        lines.push(pad_content_line(line, edges, content_width));
    }
    for _ in 0..edges.bottom {
        lines.push(blank_line(inner_width, Style::new()));
    }
    Rendered::new(lines)
}

/// Adds left/right space columns around one content line.
///
/// The line is only padded to the block width when a right margin must align;
/// otherwise it is kept as-is to avoid trailing spaces.
fn pad_content_line(line: &Line, edges: Edges, content_width: usize) -> Line {
    let mut spans = Vec::with_capacity(line.spans.len() + 2);
    if edges.left > 0 {
        spans.push(Span::raw(" ".repeat(edges.left as usize)));
    }
    if edges.right > 0 {
        let padded =
            pad_line(line.clone(), content_width, Align::Left, Style::new());
        spans.extend(padded.spans);
        spans.push(Span::raw(" ".repeat(edges.right as usize)));
    } else {
        spans.extend(line.spans.iter().cloned());
    }
    Line::new(spans)
}

/// Stacks blocks vertically, separated by `gap` blank lines.
pub fn vstack(parts: &[Rendered], gap: u16) -> Rendered {
    let mut lines = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            for _ in 0..gap {
                lines.push(Line::default());
            }
        }
        lines.extend(part.lines.iter().cloned());
    }
    Rendered::new(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_pads_lines_to_width() {
        let block = Rendered::new(vec![Line::raw("hi")]);
        let aligned = align(&block, 5, Align::Right);
        assert_eq!(aligned.lines[0].plain(), "   hi");
    }

    #[test]
    fn pad_adds_rows_and_columns() {
        let block = Rendered::new(vec![Line::raw("ab")]);
        let padded = pad(&block, Edges::all(1));
        assert_eq!(padded.height(), 3);
        assert_eq!(padded.lines[1].plain(), " ab ");
    }

    #[test]
    fn vstack_inserts_gap_lines() {
        let a = Rendered::new(vec![Line::raw("a")]);
        let b = Rendered::new(vec![Line::raw("b")]);
        let stacked = vstack(&[a, b], 1);
        assert_eq!(stacked.height(), 3);
        assert_eq!(stacked.lines[1].plain(), "");
    }
}
