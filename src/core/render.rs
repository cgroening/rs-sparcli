//! The render model ([`Rendered`]) and the inline ANSI flusher.
//!
//! Every output widget produces a [`Rendered`] (a list of styled [`Line`]s of
//! known width). The [`Renderable`] trait turns that into terminal output, and
//! [`Rendered`] is itself the composable unit for stacking and columns.

use std::io::{self, Write};

use crossterm::queue;
use crossterm::style::{
    Attribute as CtAttr, Print, ResetColor, SetAttribute, SetBackgroundColor,
    SetForegroundColor,
};

use crate::core::style::Attribute;
use crate::core::terminal::{ColorSupport, color_support, term_width};
use crate::core::text::{Line, Span, Text};
use crate::error::Result;

/// A fully laid-out block of styled lines, ready to print or compose.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rendered {
    /// The styled lines, top to bottom.
    pub lines: Vec<Line>,
}

impl Rendered {
    /// Creates a rendered block from lines.
    pub fn new(lines: Vec<Line>) -> Self {
        Self { lines }
    }

    /// Creates an empty rendered block.
    pub fn empty() -> Self {
        Self { lines: Vec::new() }
    }

    /// Appends a line.
    pub fn push(&mut self, line: impl Into<Line>) {
        self.lines.push(line.into());
    }

    /// Returns the width of the widest line in columns.
    pub fn width(&self) -> usize {
        self.lines.iter().map(Line::width).max().unwrap_or(0)
    }

    /// Returns the number of lines.
    pub fn height(&self) -> usize {
        self.lines.len()
    }

    /// Returns the plain text (no styles), lines joined by `\n`.
    pub fn plain(&self) -> String {
        self.lines
            .iter()
            .map(Line::plain)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Anything that can be laid out into a [`Rendered`] and printed.
pub trait Renderable {
    /// Lays the value out for a terminal at most `max_width` columns wide.
    fn render(&self, max_width: u16) -> Rendered;

    /// Renders at the current terminal width and prints to standard output.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing to stdout fails.
    fn print(&self) -> Result<()> {
        let rendered = self.render(term_width());
        let mut out = io::stdout().lock();
        write_rendered(&mut out, &rendered, color_support())?;
        out.flush()?;
        Ok(())
    }

    /// Renders at the current terminal width and writes to `writer`.
    ///
    /// Useful for redirecting output to a file or in-memory buffer.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    fn print_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let rendered = self.render(term_width());
        write_rendered(writer, &rendered, color_support())?;
        writer.flush()?;
        Ok(())
    }
}

impl Renderable for Rendered {
    fn render(&self, _max_width: u16) -> Rendered {
        self.clone()
    }
}

impl Renderable for Text {
    fn render(&self, _max_width: u16) -> Rendered {
        Rendered::new(self.lines.clone())
    }
}

/// Writes a [`Rendered`] block as ANSI to `writer`, downgrading colors to the
/// given support level. Each line ends with `\n`.
pub fn write_rendered<W: Write>(
    writer: &mut W,
    rendered: &Rendered,
    support: ColorSupport,
) -> io::Result<()> {
    for line in &rendered.lines {
        for span in &line.spans {
            write_span(writer, span, support)?;
        }
        queue!(writer, Print("\n"))?;
    }
    Ok(())
}

/// Writes a single styled line (no trailing newline) to `writer`.
pub(crate) fn write_line<W: Write>(
    writer: &mut W,
    line: &Line,
    support: ColorSupport,
) -> io::Result<()> {
    for span in &line.spans {
        write_span(writer, span, support)?;
    }
    Ok(())
}

/// Writes one styled span, including optional OSC-8 hyperlink wrapping.
///
/// The trailing reset is only emitted when some style was actually applied,
/// so plain text stays free of escape codes.
fn write_span<W: Write>(
    writer: &mut W,
    span: &Span,
    support: ColorSupport,
) -> io::Result<()> {
    // No-color terminals (and NO_COLOR / pipes) get plain text only, without
    // even OSC-8 hyperlink escapes.
    if support == ColorSupport::None {
        queue!(writer, Print(&span.content))?;
        return Ok(());
    }
    let colored = apply_colors(writer, span, support)?;
    let attributed = apply_attributes(writer, span.style.attrs)?;
    write_content(writer, span)?;
    if colored || attributed {
        queue!(writer, SetAttribute(CtAttr::Reset), ResetColor)?;
    }
    Ok(())
}

/// Emits foreground/background color codes; returns whether any were written.
fn apply_colors<W: Write>(
    writer: &mut W,
    span: &Span,
    support: ColorSupport,
) -> io::Result<bool> {
    let mut emitted = false;
    if let Some(color) = span.style.fg.and_then(|c| c.resolve(support)) {
        queue!(writer, SetForegroundColor(color))?;
        emitted = true;
    }
    if let Some(color) = span.style.bg.and_then(|c| c.resolve(support)) {
        queue!(writer, SetBackgroundColor(color))?;
        emitted = true;
    }
    Ok(emitted)
}

/// Emits SGR attribute codes; returns whether any were written.
fn apply_attributes<W: Write>(
    writer: &mut W,
    attrs: Attribute,
) -> io::Result<bool> {
    let mappings = [
        (Attribute::BOLD, CtAttr::Bold),
        (Attribute::DIM, CtAttr::Dim),
        (Attribute::ITALIC, CtAttr::Italic),
        (Attribute::UNDERLINED, CtAttr::Underlined),
        (Attribute::STRIKETHROUGH, CtAttr::CrossedOut),
    ];
    let mut emitted = false;
    for (flag, command) in mappings {
        if attrs.contains(flag) {
            queue!(writer, SetAttribute(command))?;
            emitted = true;
        }
    }
    Ok(emitted)
}

/// Writes the span text, wrapping it in OSC-8 codes when a link is present.
fn write_content<W: Write>(writer: &mut W, span: &Span) -> io::Result<()> {
    match &span.link {
        None => queue!(writer, Print(&span.content))?,
        Some(url) => {
            queue!(writer, Print(format!("\x1b]8;;{url}\x1b\\")))?;
            queue!(writer, Print(&span.content))?;
            queue!(writer, Print("\x1b]8;;\x1b\\"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::style::{Color, Style};

    fn render_to_string(rendered: &Rendered, support: ColorSupport) -> String {
        let mut buffer = Vec::new();
        write_rendered(&mut buffer, rendered, support).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    #[test]
    fn plain_text_has_no_escapes_without_color() {
        let rendered = Rendered::new(vec![Line::raw("hello")]);
        let output = render_to_string(&rendered, ColorSupport::None);
        assert_eq!(output, "hello\n");
    }

    #[test]
    fn colored_span_emits_escape_codes() {
        let line = Line::styled("hi", Style::new().fg(Color::Red));
        let rendered = Rendered::new(vec![line]);
        let output = render_to_string(&rendered, ColorSupport::TrueColor);
        assert!(output.contains('\u{1b}'));
        assert!(output.contains("hi"));
    }

    #[test]
    fn rendered_reports_width_and_height() {
        let rendered = Rendered::new(vec![Line::raw("ab"), Line::raw("abcd")]);
        assert_eq!(rendered.width(), 4);
        assert_eq!(rendered.height(), 2);
    }

    #[test]
    fn hyperlink_span_emits_osc8_when_colored() {
        let line = Line::new(vec![Span::raw("x").link("http://e.com")]);
        let rendered = Rendered::new(vec![line]);
        let output = render_to_string(&rendered, ColorSupport::TrueColor);
        assert!(output.contains("\x1b]8;;http://e.com"));
    }

    #[test]
    fn hyperlink_is_plain_without_color() {
        let line = Line::new(vec![Span::raw("x").link("http://e.com")]);
        let rendered = Rendered::new(vec![line]);
        let output = render_to_string(&rendered, ColorSupport::None);
        assert_eq!(output, "x\n");
    }
}
