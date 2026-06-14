//! Bordered panels framing content with an optional title and subtitle.

use crate::core::border::BorderType;
use crate::core::geometry::{Align, Edges, Position, Title};
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::{Line, Span, Text};
use crate::core::theme::theme;
use crate::output::layout::{blank_line, pad_line};

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

/// A bordered panel around rich content.
pub struct Panel {
    content: Rendered,
    opts: BoxOpts,
}

impl Panel {
    /// Creates a panel around text content.
    pub fn new(content: impl Into<Text>) -> Self {
        let text = content.into();
        Self {
            content: Rendered::new(text.lines),
            opts: BoxOpts::default(),
        }
    }

    /// Creates a panel around an already rendered block.
    pub fn from_rendered(content: Rendered) -> Self {
        Self {
            content,
            opts: BoxOpts::default(),
        }
    }

    /// Sets the border type.
    #[must_use]
    pub fn border(mut self, border: BorderType) -> Self {
        self.opts.border = border;
        self
    }

    /// Sets the border glyph style.
    #[must_use]
    pub fn border_style(mut self, style: Style) -> Self {
        self.opts.border_style = style;
        self
    }

    /// Sets the interior fill style (e.g. a background color).
    #[must_use]
    pub fn fill(mut self, style: Style) -> Self {
        self.opts.fill = style;
        self
    }

    /// Sets the inner padding.
    #[must_use]
    pub fn padding(mut self, padding: Edges) -> Self {
        self.opts.padding = padding;
        self
    }

    /// Sets the title.
    #[must_use]
    pub fn title(mut self, title: impl Into<Title>) -> Self {
        self.opts.title = Some(title.into());
        self
    }

    /// Sets the subtitle (placed on the bottom edge unless overridden).
    #[must_use]
    pub fn subtitle(mut self, mut subtitle: Title) -> Self {
        subtitle.position = Position::Bottom;
        self.opts.subtitle = Some(subtitle);
        self
    }

    /// Sets a fixed outer width in columns.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.opts.width = Some(width);
        self
    }

    /// Sets the horizontal content alignment.
    #[must_use]
    pub fn content_align(mut self, align: Align) -> Self {
        self.opts.content_align = align;
        self
    }
}

impl Renderable for Panel {
    fn render(&self, _max_width: u16) -> Rendered {
        draw_box(&self.content, &self.opts)
    }
}

/// Frames `content` according to `opts`, returning the boxed block.
pub(crate) fn draw_box(content: &Rendered, opts: &BoxOpts) -> Rendered {
    if opts.border.is_none() {
        return frame_borderless(content, opts);
    }
    let area_width = compute_area_width(content, opts);
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

/// Computes the interior width between the vertical borders.
fn compute_area_width(content: &Rendered, opts: &BoxOpts) -> usize {
    let base = match opts.width {
        Some(total) => (total as usize).saturating_sub(2),
        None => content.width() + opts.padding.horizontal() as usize,
    };
    let title_min = title_width(opts.title.as_ref());
    let subtitle_min = title_width(opts.subtitle.as_ref());
    base.max(title_min).max(subtitle_min)
}

/// Returns the column width a title occupies including its padding.
fn title_width(title: Option<&Title>) -> usize {
    match title {
        None => 0,
        Some(title) => {
            let text = title.content.lines.first().map_or(0, Line::width);
            text + 2 * title.pad as usize + 2
        }
    }
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

/// Wraps one content line with padding and vertical borders.
fn content_row(line: &Line, opts: &BoxOpts, content_area: usize) -> Line {
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
    wrap_with_borders(Line::new(spans), opts)
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

/// Embeds a title within a horizontal border run.
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
    let remaining = area_width.saturating_sub(title_w);
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

/// Frames content without borders: just padding around it.
fn frame_borderless(content: &Rendered, opts: &BoxOpts) -> Rendered {
    crate::output::compose::pad(content, opts.padding)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(rendered: &Rendered) -> Vec<String> {
        rendered.lines.iter().map(Line::plain).collect()
    }

    #[test]
    fn panel_frames_content_with_border() {
        let panel = Panel::new("hi").border(BorderType::Single);
        let lines = plain(&panel.render(40));
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with('┌'));
        assert!(lines[1].contains("hi"));
        assert!(lines[2].starts_with('└'));
    }

    #[test]
    fn panel_embeds_title_in_top_border() {
        let panel = Panel::new("body")
            .border(BorderType::Single)
            .title(Title::new("Info"));
        let lines = plain(&panel.render(40));
        assert!(lines[0].contains("Info"));
    }

    #[test]
    fn borderless_panel_only_pads() {
        let panel = Panel::new("x")
            .border(BorderType::None)
            .padding(Edges::all(1));
        let lines = plain(&panel.render(40));
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], " x ");
    }
}
