//! Horizontal rules (separators) with an optional embedded title.

use crate::core::border::BorderType;
use crate::core::geometry::{Align, Edges};
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::{Line, Span, Text};
use crate::core::theme::theme;
use crate::output::compose::pad;

/// A horizontal divider line, optionally labelled with a title.
///
/// # Examples
///
/// ```
/// use sparcli::{Renderable, Rule};
///
/// let out = Rule::with_title("Settings").render(30);
/// assert!(out.plain().contains("Settings"));
/// ```
pub struct Rule {
    title: Option<Text>,
    border: BorderType,
    style: Style,
    align: Align,
    width: Option<u16>,
    margin: Edges,
    pad: u16,
}

impl Default for Rule {
    fn default() -> Self {
        let theme = theme();
        Self {
            title: None,
            border: theme.border,
            style: theme.secondary,
            align: Align::Center,
            width: None,
            margin: Edges::default(),
            pad: 1,
        }
    }
}

impl Rule {
    /// Creates a plain rule with no title.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a rule with an embedded title.
    pub fn with_title(title: impl Into<Text>) -> Self {
        Self {
            title: Some(title.into()),
            ..Self::default()
        }
    }

    /// Sets the line style (border type).
    #[must_use]
    pub fn border(mut self, border: BorderType) -> Self {
        self.border = border;
        self
    }

    /// Sets the line/glyph style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the title alignment.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Sets a fixed width in columns.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets the outer margin.
    #[must_use]
    pub fn margin(mut self, margin: Edges) -> Self {
        self.margin = margin;
        self
    }
}

impl Renderable for Rule {
    fn render(&self, max_width: u16) -> Rendered {
        let total = self.width.unwrap_or(max_width) as usize;
        let inner = total.saturating_sub(self.margin.horizontal() as usize);
        let glyph = self.border.chars().horizontal;
        let line = match &self.title {
            None => fill_line(glyph, inner, self.style),
            Some(title) => self.titled_line(title, glyph, inner),
        };
        pad(&Rendered::new(vec![line]), self.margin)
    }
}

impl Rule {
    /// Builds a rule line with a centered/aligned title.
    fn titled_line(&self, title: &Text, glyph: char, width: usize) -> Line {
        let title_line = title.lines.first().cloned().unwrap_or_default();
        let pad = self.pad as usize;
        let title_w = title_line.width() + 2 * pad;
        if title_w >= width {
            return title_line;
        }
        let remaining = width - title_w;
        let (left, right) = match self.align {
            Align::Left => (1.min(remaining), remaining.saturating_sub(1)),
            Align::Right => (remaining.saturating_sub(1), 1.min(remaining)),
            Align::Center => (remaining / 2, remaining - remaining / 2),
        };
        let mut spans = vec![glyph_span(glyph, left, self.style)];
        spans.push(Span::raw(" ".repeat(pad)));
        spans.extend(title_line.spans);
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(glyph_span(glyph, right, self.style));
        Line::new(spans)
    }
}

/// Builds a full line of `width` glyphs.
fn fill_line(glyph: char, width: usize, style: Style) -> Line {
    Line::new(vec![glyph_span(glyph, width, style)])
}

/// Builds a span of `count` repeated glyphs.
fn glyph_span(glyph: char, count: usize, style: Style) -> Span {
    Span::styled(glyph.to_string().repeat(count), style)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_rule_fills_the_width() {
        let rendered = Rule::new().border(BorderType::Single).render(10);
        assert_eq!(rendered.lines[0].plain(), "──────────");
    }

    #[test]
    fn titled_rule_contains_title() {
        let rendered = Rule::with_title("Section")
            .border(BorderType::Single)
            .render(20);
        assert!(rendered.lines[0].plain().contains("Section"));
        assert_eq!(rendered.lines[0].width(), 20);
    }
}
