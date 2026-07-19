//! Box drawing shared by the framed output widgets.
//!
//! [`draw_box`] frames a [`Rendered`] block with a border, padding and up to
//! two edge titles. [`Panel`](crate::output::panel::Panel) and
//! [`Alert`](crate::output::alert::Alert) both build on it, so the frame
//! geometry lives here instead of inside one of them.
//!
//! Named `box_draw` rather than `box` because `box` is a reserved Rust
//! keyword; the Python port calls the same module `box.py`.

use crate::core::border::BorderType;
use crate::core::geometry::{Align, Edges, Position, Title};
use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::theme::theme;
use crate::core::width::{ELLIPSIS, truncate};
use crate::output::layout::{blank_line, pad_line};

/// Columns consumed by the two vertical border glyphs.
const BORDER_COLUMNS: usize = 2;

/// Box-drawing options shared by panels and other framed widgets.
pub(crate) struct BoxOpts {
    /// Border style.
    pub border: BorderType,
    /// Style applied to the border glyphs.
    pub border_style: Style,
    /// Fill style for the interior (padding and content background).
    pub fill: Style,
    /// Inner padding between border and content.
    pub padding: Edges,
    /// Optional title.
    pub title: Option<Title>,
    /// Optional subtitle (bottom edge by default).
    pub subtitle: Option<Title>,
    /// Optional fixed outer width in columns.
    pub width: Option<u16>,
    /// Horizontal alignment of content within the panel.
    pub content_align: Align,
}

impl Default for BoxOpts {
    fn default() -> Self {
        Self {
            border: theme().border,
            border_style: Style::new(),
            fill: Style::new(),
            padding: Edges::symmetric(0, 1),
            title: None,
            subtitle: None,
            width: None,
            content_align: Align::Left,
        }
    }
}

/// Frames `content` according to `opts`, clamped to `max_width` columns.
///
/// The frame never exceeds `max_width`: a fixed width is capped, natural
/// content that overflows shrinks the frame, and a title too wide for the
/// interior is truncated rather than widening the box.
pub(crate) fn draw_box(
    content: &Rendered,
    opts: &BoxOpts,
    max_width: u16,
) -> Rendered {
    let max_width = max_width as usize;
    if opts.border.is_none() {
        return frame_borderless(content, opts, max_width);
    }
    let area_width = compute_area_width(content, opts, max_width);
    let content_area =
        area_width.saturating_sub(opts.padding.horizontal() as usize);

    let mut lines = Vec::new();
    let top_title = edge_title(opts, Position::Top);
    lines.push(edge_line(opts, area_width, Position::Top, top_title));
    push_padding_rows(&mut lines, opts, area_width, opts.padding.top);
    for line in &content.lines {
        lines.push(content_row(line, opts, content_area));
    }
    push_padding_rows(&mut lines, opts, area_width, opts.padding.bottom);
    let bottom_title = edge_title(opts, Position::Bottom);
    lines.push(edge_line(opts, area_width, Position::Bottom, bottom_title));
    Rendered::new(lines)
}

/// Returns the title that sits on the given edge, if any.
fn edge_title(opts: &BoxOpts, position: Position) -> Option<&Title> {
    [opts.title.as_ref(), opts.subtitle.as_ref()]
        .into_iter()
        .flatten()
        .find(|title| title.position == position)
}

/// Computes the interior width between the vertical borders, clamped so the
/// whole frame fits within `max_width`.
fn compute_area_width(
    content: &Rendered,
    opts: &BoxOpts,
    max_width: usize,
) -> usize {
    let border_cols = if opts.border.is_none() {
        0
    } else {
        BORDER_COLUMNS
    };
    let padding = opts.padding.horizontal() as usize;
    let overhead = border_cols + padding;
    let content_area = match opts.width {
        Some(total) => (total as usize).min(max_width).saturating_sub(overhead),
        None => {
            let natural = content.width();
            if natural + overhead <= max_width {
                natural
            } else {
                max_width.saturating_sub(overhead)
            }
        }
    };
    content_area + padding
}

/// Builds the content padding rows (blank interior lines).
fn push_padding_rows(
    lines: &mut Vec<Line>,
    opts: &BoxOpts,
    area_width: usize,
    count: u16,
) {
    for _ in 0..count {
        lines.push(wrap_with_borders(blank_line(area_width, opts.fill), opts));
    }
}

/// Builds one aligned, padded content line without border glyphs.
fn inner_row(line: &Line, opts: &BoxOpts, content_area: usize) -> Line {
    let mut spans = Vec::new();
    if opts.padding.left > 0 {
        spans.push(Span::styled(
            " ".repeat(opts.padding.left as usize),
            opts.fill,
        ));
    }
    let padded =
        pad_line(line.clone(), content_area, opts.content_align, opts.fill);
    spans.extend(padded.spans);
    if opts.padding.right > 0 {
        spans.push(Span::styled(
            " ".repeat(opts.padding.right as usize),
            opts.fill,
        ));
    }
    Line::new(spans)
}

/// Wraps one content line with padding and vertical borders.
fn content_row(line: &Line, opts: &BoxOpts, content_area: usize) -> Line {
    wrap_with_borders(inner_row(line, opts, content_area), opts)
}

/// Adds left/right vertical border glyphs around `inner`.
fn wrap_with_borders(inner: Line, opts: &BoxOpts) -> Line {
    let chars = opts.border.chars();
    let mut spans = Vec::with_capacity(inner.spans.len() + 2);
    spans.push(Span::styled(chars.vertical.to_string(), opts.border_style));
    spans.extend(inner.spans);
    spans.push(Span::styled(chars.vertical.to_string(), opts.border_style));
    Line::new(spans)
}

/// Builds a top or bottom border line, optionally embedding a title.
fn edge_line(
    opts: &BoxOpts,
    area_width: usize,
    position: Position,
    title: Option<&Title>,
) -> Line {
    let chars = opts.border.chars();
    let (left, right) = match position {
        Position::Bottom => (chars.bottom_left, chars.bottom_right),
        Position::Top => (chars.top_left, chars.top_right),
    };
    let mut spans = vec![Span::styled(left.to_string(), opts.border_style)];
    match title {
        None => spans.push(horizontal_fill(chars.horizontal, area_width, opts)),
        Some(title) => {
            push_titled_fill(&mut spans, title, area_width, opts);
        }
    }
    spans.push(Span::styled(right.to_string(), opts.border_style));
    Line::new(spans)
}

/// Builds a horizontal fill span of `width` border glyphs.
fn horizontal_fill(glyph: char, width: usize, opts: &BoxOpts) -> Span {
    Span::styled(glyph.to_string().repeat(width), opts.border_style)
}

/// Embeds a title within a horizontal border run so it reads as part of the
/// frame. A left-aligned title keeps exactly one connecting border glyph before
/// it (`┌─ Title ─`), never a flush `┌ Title`; the glyphs on both sides take the
/// border style so the seam stays uniform.
fn push_titled_fill(
    spans: &mut Vec<Span>,
    title: &Title,
    area_width: usize,
    opts: &BoxOpts,
) {
    let chars = opts.border.chars();
    let pad = title.pad as usize;
    let title_line = title.content.lines.first().cloned().unwrap_or_default();
    let title_w = title_line.width() + 2 * pad;
    if title_w >= area_width {
        // Too wide for the interior: truncate into the border run instead of
        // widening the frame.
        let text = truncate(&title_line.plain(), area_width, ELLIPSIS);
        spans.push(Span::styled(text, opts.border_style));
        return;
    }
    let remaining = area_width - title_w;
    let (left_fill, right_fill) = match title.align {
        Align::Left => (1.min(remaining), remaining.saturating_sub(1)),
        Align::Right => (remaining.saturating_sub(1), 1.min(remaining)),
        Align::Center => (remaining / 2, remaining - remaining / 2),
    };
    spans.push(horizontal_fill(chars.horizontal, left_fill, opts));
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    spans.extend(title_line.spans);
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    spans.push(horizontal_fill(chars.horizontal, right_fill, opts));
}

/// Frames content without borders: padding around it, clamped to `max_width`.
fn frame_borderless(
    content: &Rendered,
    opts: &BoxOpts,
    max_width: usize,
) -> Rendered {
    let area_width = compute_area_width(content, opts, max_width);
    let content_area =
        area_width.saturating_sub(opts.padding.horizontal() as usize);
    let mut lines = Vec::new();
    for _ in 0..opts.padding.top {
        lines.push(blank_line(area_width, opts.fill));
    }
    for line in &content.lines {
        lines.push(inner_row(line, opts, content_area));
    }
    for _ in 0..opts.padding.bottom {
        lines.push(blank_line(area_width, opts.fill));
    }
    Rendered::new(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(text: &str) -> Rendered {
        Rendered::new(vec![Line::raw(text)])
    }

    fn single(padding: Edges) -> BoxOpts {
        BoxOpts {
            border: BorderType::Single,
            padding,
            ..BoxOpts::default()
        }
    }

    #[test]
    fn draw_box_wraps_content_in_a_frame() {
        let out = draw_box(&body("hi"), &single(Edges::symmetric(0, 1)), 40);
        assert_eq!(out.lines.len(), 3);
        assert!(out.lines[1].plain().contains("hi"));
    }

    #[test]
    fn edge_title_picks_the_title_for_its_own_edge() {
        let mut opts = single(Edges::symmetric(0, 1));
        opts.title = Some(Title::new("top"));
        let mut bottom = Title::new("bottom");
        bottom.position = Position::Bottom;
        opts.subtitle = Some(bottom);
        assert!(edge_title(&opts, Position::Top).is_some());
        assert!(edge_title(&opts, Position::Bottom).is_some());
    }

    #[test]
    fn compute_area_width_caps_a_fixed_width_at_max_width() {
        let opts = BoxOpts {
            width: Some(200),
            ..single(Edges::symmetric(0, 1))
        };
        // 80 columns minus the two border glyphs leaves 78 of interior.
        assert_eq!(compute_area_width(&body("hi"), &opts, 80), 78);
    }

    #[test]
    fn compute_area_width_shrinks_to_natural_content() {
        let opts = single(Edges::symmetric(0, 0));
        assert_eq!(compute_area_width(&body("hi"), &opts, 80), 2);
    }

    #[test]
    fn borderless_frame_emits_no_glyphs() {
        let opts = BoxOpts {
            border: BorderType::None,
            padding: Edges::all(1),
            ..BoxOpts::default()
        };
        let out = draw_box(&body("x"), &opts, 40);
        assert_eq!(out.lines.len(), 3);
        assert_eq!(out.lines[1].plain(), " x ");
    }

    #[test]
    fn wrap_with_borders_adds_one_glyph_per_side() {
        let opts = single(Edges::symmetric(0, 0));
        let wrapped = wrap_with_borders(Line::raw("ab"), &opts);
        assert_eq!(wrapped.plain(), "│ab│");
    }
}
