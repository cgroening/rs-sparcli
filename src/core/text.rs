//! Rich text: styled [`Span`]s grouped into [`Line`]s and [`Text`].
//!
//! Mirrors ratatui's `Span`/`Line`/`Text` so the API feels familiar. A span
//! may carry an OSC-8 hyperlink target.

use crate::core::style::Style;
use crate::core::width::visible_width;

/// A run of text sharing one [`Style`] and an optional hyperlink.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Span {
    /// The text content (without ANSI escapes).
    pub content: String,
    /// The style applied at render time.
    pub style: Style,
    /// Optional OSC-8 hyperlink target.
    pub link: Option<String>,
}

impl Span {
    /// Creates an unstyled span.
    pub fn raw(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: Style::new(),
            link: None,
        }
    }

    /// Creates a styled span.
    pub fn styled(content: impl Into<String>, style: Style) -> Self {
        Self {
            content: content.into(),
            style,
            link: None,
        }
    }

    /// Attaches an OSC-8 hyperlink target to the span.
    #[must_use]
    pub fn link(mut self, url: impl Into<String>) -> Self {
        self.link = Some(url.into());
        self
    }

    /// Returns the visible column width of the span.
    pub fn width(&self) -> usize {
        visible_width(&self.content)
    }
}

impl From<&str> for Span {
    fn from(value: &str) -> Self {
        Span::raw(value)
    }
}

impl From<String> for Span {
    fn from(value: String) -> Self {
        Span::raw(value)
    }
}

/// One visual line: a sequence of [`Span`]s.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    /// The spans making up the line.
    pub spans: Vec<Span>,
}

impl Line {
    /// Creates a line from a list of spans.
    pub fn new(spans: Vec<Span>) -> Self {
        Self { spans }
    }

    /// Creates an unstyled line from a single string.
    pub fn raw(content: impl Into<String>) -> Self {
        Self::new(vec![Span::raw(content)])
    }

    /// Creates a styled line from a single string.
    pub fn styled(content: impl Into<String>, style: Style) -> Self {
        Self::new(vec![Span::styled(content, style)])
    }

    /// Returns the total visible width of the line.
    pub fn width(&self) -> usize {
        self.spans.iter().map(Span::width).sum()
    }

    /// Returns the concatenated plain text of the line.
    pub fn plain(&self) -> String {
        self.spans.iter().map(|s| s.content.as_str()).collect()
    }
}

impl From<&str> for Line {
    fn from(value: &str) -> Self {
        Line::raw(value)
    }
}

impl From<String> for Line {
    fn from(value: String) -> Self {
        Line::raw(value)
    }
}

impl From<Span> for Line {
    fn from(span: Span) -> Self {
        Line::new(vec![span])
    }
}

impl From<Vec<Span>> for Line {
    fn from(spans: Vec<Span>) -> Self {
        Line::new(spans)
    }
}

/// Multi-line rich text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Text {
    /// The lines of the text.
    pub lines: Vec<Line>,
}

impl Text {
    /// Creates text from a list of lines.
    pub fn new(lines: Vec<Line>) -> Self {
        Self { lines }
    }

    /// Creates unstyled text, splitting `content` on newlines.
    pub fn raw(content: impl Into<String>) -> Self {
        let content = content.into();
        let lines = content.split('\n').map(Line::raw).collect();
        Self { lines }
    }

    /// Creates styled text, splitting `content` on newlines.
    pub fn styled(content: impl Into<String>, style: Style) -> Self {
        let content = content.into();
        let lines = content
            .split('\n')
            .map(|l| Line::styled(l, style))
            .collect();
        Self { lines }
    }

    /// Appends a line.
    pub fn push_line(&mut self, line: impl Into<Line>) {
        self.lines.push(line.into());
    }

    /// Returns the width of the widest line.
    pub fn width(&self) -> usize {
        self.lines.iter().map(Line::width).max().unwrap_or(0)
    }

    /// Returns the number of lines.
    pub fn height(&self) -> usize {
        self.lines.len()
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Text::raw(value)
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Text::raw(value)
    }
}

impl From<Span> for Text {
    fn from(span: Span) -> Self {
        Text::new(vec![Line::from(span)])
    }
}

impl From<Line> for Text {
    fn from(line: Line) -> Self {
        Text::new(vec![line])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::style::Color;

    #[test]
    fn line_width_sums_span_widths() {
        let line = Line::new(vec![Span::raw("ab"), Span::raw("中")]);
        assert_eq!(line.width(), 4);
    }

    #[test]
    fn text_raw_splits_on_newlines() {
        let text = Text::raw("a\nbb\nccc");
        assert_eq!(text.height(), 3);
        assert_eq!(text.width(), 3);
    }

    #[test]
    fn span_link_is_attached() {
        let span = Span::raw("x").link("http://example.com");
        assert_eq!(span.link.as_deref(), Some("http://example.com"));
    }

    #[test]
    fn styled_helpers_carry_style() {
        let line = Line::styled("hi", Style::new().fg(Color::Red));
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
    }
}
