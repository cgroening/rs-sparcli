//! Row construction for a card: regions, padding, borders and fitted text.
//!
//! Every row is assembled through [`region_row`], which materializes the
//! padding columns, the alignment slack and the border glyphs with the region's
//! background - and rebases the content spans onto it. That is what keeps the
//! surface gapless; see [`super::style`] for the style math it relies on.

use crate::core::border::{BorderType, TALL};
use crate::core::geometry::{Align, Edges, Position};
use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::width::{ELLIPSIS, truncate_line, wrap_line};
use crate::output::card::style::{rebase, swap_colors};
use crate::output::layout::{blank_line, pad_line};

/// Columns consumed by the two vertical border glyphs.
const BORDER_COLUMNS: usize = 2;

/// The geometry and styling one row of a card needs.
pub(super) struct Region {
    /// Width of the surface between the borders, in columns.
    pub width: usize,
    /// Padding between the surface edge and the text.
    pub padding: Edges,
    /// Horizontal alignment of the text.
    pub align: Align,
    /// Base style the row's text is rebased onto.
    pub text: Style,
    /// Background of every cell in the row.
    pub surface: Style,
    /// Style of the border glyphs, already carrying the surface background.
    pub border: Style,
    /// Border type; [`BorderType::None`] omits the vertical glyphs.
    pub border_type: BorderType,
    /// Whether overlong lines wrap instead of being truncated.
    pub wrap: bool,
}

impl Region {
    /// Returns the width available to text, after the horizontal padding.
    fn area(&self) -> usize {
        self.width
            .saturating_sub(self.padding.horizontal() as usize)
    }
}

/// Returns the columns the vertical border glyphs consume.
pub(super) fn border_columns(border: BorderType) -> usize {
    if border.is_none() { 0 } else { BORDER_COLUMNS }
}

/// Appends one region's padding rows, text rows and padding rows.
pub(super) fn push_block(
    lines: &mut Vec<Line>,
    source: &[Line],
    region: &Region,
) {
    for _ in 0..region.padding.top {
        lines.push(blank_row(region));
    }
    let area = region.area();
    if area > 0 {
        for line in source {
            for fitted in fit(line, area, region.wrap) {
                lines.push(region_row(fitted, region));
            }
        }
    }
    for _ in 0..region.padding.bottom {
        lines.push(blank_row(region));
    }
}

/// Fits one line into `area` columns, either by wrapping or by truncating.
fn fit(line: &Line, area: usize, wrap: bool) -> Vec<Line> {
    if wrap {
        return wrap_line(line, area);
    }
    vec![truncate_line(line, area, ELLIPSIS)]
}

/// Builds one full-width row: border glyph, padding, aligned text, padding,
/// border glyph - every cell carrying the region's background.
fn region_row(line: Line, region: &Region) -> Line {
    let mut spans = Vec::new();
    push_left_border(&mut spans, region);
    push_padding(&mut spans, region.padding.left, region.surface);
    let rebased = rebase(line, region.text);
    let padded = pad_line(rebased, region.area(), region.align, region.surface);
    spans.extend(padded.spans);
    push_padding(&mut spans, region.padding.right, region.surface);
    push_right_border(&mut spans, region);
    Line::new(spans)
}

/// Builds an empty row of the region's surface.
fn blank_row(region: &Region) -> Line {
    let mut spans = Vec::new();
    push_left_border(&mut spans, region);
    spans.extend(blank_line(region.width, region.surface).spans);
    push_right_border(&mut spans, region);
    Line::new(spans)
}

/// Builds the top or bottom border row of a region.
pub(super) fn edge_row(region: &Region, position: Position) -> Line {
    if region.border_type.is_tall() {
        return tall_edge_row(region, position);
    }
    let chars = region.border_type.chars();
    let (left, right) = match position {
        Position::Top => (chars.top_left, chars.top_right),
        Position::Bottom => (chars.bottom_left, chars.bottom_right),
    };
    let mut content = String::with_capacity(region.width + BORDER_COLUMNS);
    content.push(left);
    for _ in 0..region.width {
        content.push(chars.horizontal);
    }
    content.push(right);
    Line::new(vec![Span::styled(content, region.border)])
}

/// Builds a tall border's horizontal row.
///
/// The line runs across the corner cells as well, which is what closes the
/// corner, and it carries no surface behind it so the colored area begins at
/// the line rather than a row above it.
fn tall_edge_row(region: &Region, position: Position) -> Line {
    let glyph = match position {
        Position::Top => TALL.top,
        Position::Bottom => TALL.bottom,
    };
    let width = region.width + BORDER_COLUMNS;
    let style = Style {
        bg: None,
        ..region.border
    };
    Line::new(vec![Span::styled(glyph.to_string().repeat(width), style)])
}

/// Appends the left border glyph, unless the card is unframed.
fn push_left_border(spans: &mut Vec<Span>, region: &Region) {
    if region.border_type.is_none() {
        return;
    }
    let glyph = if region.border_type.is_tall() {
        TALL.left
    } else {
        region.border_type.chars().vertical
    };
    spans.push(Span::styled(glyph.to_string(), region.border));
}

/// Appends the right border glyph, unless the card is unframed.
fn push_right_border(spans: &mut Vec<Span>, region: &Region) {
    if region.border_type.is_none() {
        return;
    }
    if region.border_type.is_tall() {
        let style = swap_colors(region.border);
        spans.push(Span::styled(TALL.right.to_string(), style));
        return;
    }
    let glyph = region.border_type.chars().vertical;
    spans.push(Span::styled(glyph.to_string(), region.border));
}

/// Appends `columns` padding cells carrying the surface background.
fn push_padding(spans: &mut Vec<Span>, columns: u16, surface: Style) {
    if columns == 0 {
        return;
    }
    spans.push(Span::styled(" ".repeat(columns as usize), surface));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a plain region of `width` columns with the given border.
    fn region(width: usize, border: BorderType) -> Region {
        Region {
            width,
            padding: Edges::symmetric(0, 1),
            align: Align::Left,
            text: Style::new(),
            surface: Style::new(),
            border: Style::new(),
            border_type: border,
            wrap: true,
        }
    }

    #[test]
    fn area_subtracts_the_horizontal_padding() {
        assert_eq!(region(20, BorderType::Single).area(), 18);
    }

    #[test]
    fn area_of_an_overpadded_region_saturates_at_zero() {
        let mut narrow = region(2, BorderType::Single);
        narrow.padding = Edges::symmetric(0, 4);
        assert_eq!(narrow.area(), 0);
    }

    #[test]
    fn border_columns_are_zero_without_a_border() {
        assert_eq!(border_columns(BorderType::None), 0);
        assert_eq!(border_columns(BorderType::Rounded), BORDER_COLUMNS);
    }

    #[test]
    fn fit_wraps_when_wrapping_is_on() {
        let line = Line::raw("alpha beta gamma");
        assert_eq!(fit(&line, 10, true).len(), 2);
    }

    #[test]
    fn fit_truncates_with_an_ellipsis_when_wrapping_is_off() {
        let line = Line::raw("alpha beta gamma");
        let fitted = fit(&line, 10, false);
        assert_eq!(fitted.len(), 1);
        assert!(fitted[0].plain().contains(ELLIPSIS));
    }

    #[test]
    fn region_row_spans_the_full_outer_width() {
        let region = region(20, BorderType::Single);
        let row = region_row(Line::raw("hi"), &region);
        assert_eq!(row.width(), 20 + BORDER_COLUMNS);
    }

    #[test]
    fn blank_row_spans_the_full_outer_width() {
        let region = region(20, BorderType::Single);
        assert_eq!(blank_row(&region).width(), 20 + BORDER_COLUMNS);
    }

    #[test]
    fn an_unframed_region_omits_the_side_glyphs() {
        let region = region(20, BorderType::None);
        let row = region_row(Line::raw("hi"), &region);
        assert_eq!(row.width(), 20);
    }

    #[test]
    fn edge_row_closes_the_frame_with_corner_glyphs() {
        let region = region(4, BorderType::Single);
        assert_eq!(edge_row(&region, Position::Top).plain(), "┌────┐");
        assert_eq!(edge_row(&region, Position::Bottom).plain(), "└────┘");
    }

    #[test]
    fn a_tall_edge_row_runs_across_the_corner_cells() {
        let region = region(4, BorderType::Tall);
        let top = edge_row(&region, Position::Top);
        assert_eq!(top.plain(), TALL.top.to_string().repeat(6));
    }

    #[test]
    fn a_tall_edge_row_carries_no_surface() {
        let mut region = region(4, BorderType::Tall);
        region.border = Style::new().bg(crate::core::style::Color::Blue);
        let top = edge_row(&region, Position::Top);
        assert_eq!(top.spans[0].style.bg, None);
    }

    #[test]
    fn push_padding_emits_nothing_for_zero_columns() {
        let mut spans = Vec::new();
        push_padding(&mut spans, 0, Style::new());
        assert!(spans.is_empty());
    }
}
