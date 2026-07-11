//! Rich-style inline markup parsing (`[bold red]text[/]`).
//!
//! Available with the `markup` feature. Tags are space-separated specs of
//! attributes (`bold`, `dim`, `italic`, `underline`, `strike`) and colors
//! (named or `#rrggbb`); `on <color>` sets the background. `[/]` closes the
//! most recent tag. Backtick-delimited `` `code` `` spans get a code style.
//! A backslash escapes the next character (`\[` yields a literal `[`).

use crate::core::render::Renderable;
use crate::core::style::{Color, Style};
use crate::core::text::{Line, Span, Text};
use crate::error::Result;

/// Parses markup into rich [`Text`].
///
/// Unknown tags are ignored (treated as empty styles). Malformed brackets are
/// emitted literally, so this never fails on arbitrary input.
///
/// # Examples
///
/// ```
/// # use sparcli::markup::parse;
/// let text = parse("[bold]hi[/] there");
/// assert_eq!(text.lines[0].spans[0].content, "hi");
/// ```
pub fn parse(markup: &str) -> Text {
    Parser::new().run(markup)
}

/// Parses `markup` and prints it to standard output (no trailing newline rules
/// beyond the text's own lines).
///
/// # Errors
///
/// Returns [`crate::SparcliError::Io`] if writing fails.
pub fn markup_print(markup: &str) -> Result<()> {
    parse(markup).print()
}

/// Parses `markup`, prints it and appends a final newline.
///
/// # Errors
///
/// Returns [`crate::SparcliError::Io`] if writing fails.
pub fn markup_println(markup: &str) -> Result<()> {
    let mut text = parse(markup);
    text.push_line(Line::default());
    text.print()
}

/// Incremental markup parser holding the style stack and output buffers.
struct Parser {
    stack: Vec<Style>,
    lines: Vec<Line>,
    spans: Vec<Span>,
    buffer: String,
    in_code: bool,
}

impl Parser {
    fn new() -> Self {
        Self {
            stack: vec![Style::new()],
            lines: Vec::new(),
            spans: Vec::new(),
            buffer: String::new(),
            in_code: false,
        }
    }

    /// Parses the whole input and returns the assembled text.
    fn run(mut self, markup: &str) -> Text {
        let mut chars = markup.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\\' => self.push_escaped(chars.next()),
                '[' => self.handle_tag(&mut chars),
                '`' => self.toggle_code(),
                '\n' => self.break_line(),
                _ => self.buffer.push(ch),
            }
        }
        self.finish()
    }

    /// Pushes an escaped character literally (backslash kept if at end).
    fn push_escaped(&mut self, next: Option<char>) {
        match next {
            Some(ch) => self.buffer.push(ch),
            None => self.buffer.push('\\'),
        }
    }

    /// Handles a `[...]` tag, or emits `[` literally if unterminated.
    fn handle_tag(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        let mut tag = String::new();
        let mut closed = false;
        for ch in chars.by_ref() {
            if ch == ']' {
                closed = true;
                break;
            }
            tag.push(ch);
        }
        if !closed {
            self.buffer.push('[');
            self.buffer.push_str(&tag);
            return;
        }
        self.apply_tag(&tag);
    }

    /// Applies an open or close tag, or emits an unrecognized bracket literally.
    fn apply_tag(&mut self, raw: &str) {
        let tag = raw.trim();
        if tag == "/" {
            self.flush_span();
            if self.stack.len() > 1 {
                self.stack.pop();
            }
            return;
        }
        let specs = parse_specs(tag);
        if specs == Style::new() {
            // A closed bracket that names no known style or attribute (e.g.
            // "array[0]") is content, not markup, so emit it literally.
            self.buffer.push('[');
            self.buffer.push_str(raw);
            self.buffer.push(']');
            return;
        }
        self.flush_span();
        let current = self.current_style();
        self.stack.push(current.patch(specs));
    }

    /// Toggles a code span on or off using the code style.
    fn toggle_code(&mut self) {
        self.flush_span();
        if self.in_code {
            if self.stack.len() > 1 {
                self.stack.pop();
            }
        } else {
            self.stack.push(code_style(self.current_style()));
        }
        self.in_code = !self.in_code;
    }

    /// Returns the cumulative style at the top of the stack.
    fn current_style(&self) -> Style {
        *self.stack.last().unwrap_or(&Style::new())
    }

    /// Flushes the text buffer into a span with the current style.
    fn flush_span(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.buffer);
        self.spans.push(Span::styled(content, self.current_style()));
    }

    /// Ends the current visual line.
    fn break_line(&mut self) {
        self.flush_span();
        self.lines.push(Line::new(std::mem::take(&mut self.spans)));
    }

    /// Flushes remaining content and returns the text.
    fn finish(mut self) -> Text {
        self.flush_span();
        self.lines.push(Line::new(std::mem::take(&mut self.spans)));
        Text::new(self.lines)
    }
}

/// Parses a space-separated style spec into a [`Style`].
///
/// Attribute names and the `on` background keyword are matched
/// case-insensitively; color tokens keep their original casing (hex and named
/// colors are parsed case-insensitively anyway).
fn parse_specs(tag: &str) -> Style {
    let mut style = Style::new();
    let mut expect_background = false;
    for token in tag.split_whitespace() {
        let lowered = token.to_ascii_lowercase();
        if lowered == "on" {
            expect_background = true;
            continue;
        }
        if expect_background {
            if let Some(color) = parse_color(token) {
                style = style.bg(color);
            }
            expect_background = false;
            continue;
        }
        style = apply_token(style, &lowered, token);
    }
    style
}

/// Applies a single attribute or foreground color token to `style`.
///
/// `lowered` is matched against the attribute names; `token` keeps its casing
/// for color parsing.
fn apply_token(style: Style, lowered: &str, token: &str) -> Style {
    match lowered {
        "bold" | "b" => style.bold(),
        "dim" | "d" => style.dim(),
        "italic" | "i" => style.italic(),
        "underline" | "underlined" | "u" => style.underlined(),
        "strike" | "strikethrough" | "s" => style.strikethrough(),
        _ => match parse_color(token) {
            Some(color) => style.fg(color),
            None => style,
        },
    }
}

/// Parses a color token (named or `#rrggbb`).
fn parse_color(token: &str) -> Option<Color> {
    if token.starts_with('#') {
        Color::from_hex(token)
    } else {
        Color::from_name(token)
    }
}

/// The style used for backtick code spans, layered on `base`.
fn code_style(base: Style) -> Style {
    base.patch(Style::new().fg(Color::Cyan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::style::Attribute;

    #[test]
    fn parses_attribute_and_color_tag() {
        let text = parse("[bold red]warn[/]");
        let span = &text.lines[0].spans[0];
        assert_eq!(span.content, "warn");
        assert_eq!(span.style.fg, Some(Color::Red));
        assert!(span.style.attrs.contains(Attribute::BOLD));
    }

    #[test]
    fn parses_background_with_on_keyword() {
        let text = parse("[white on blue]x[/]");
        let style = text.lines[0].spans[0].style;
        assert_eq!(style.bg, Some(Color::Blue));
    }

    #[test]
    fn parses_hex_color() {
        let text = parse("[#ff8800]x[/]");
        assert_eq!(
            text.lines[0].spans[0].style.fg,
            Some(Color::Rgb(255, 136, 0))
        );
    }

    #[test]
    fn unterminated_bracket_is_literal() {
        let text = parse("a [b c");
        assert_eq!(text.lines[0].plain(), "a [b c");
    }

    #[test]
    fn closed_unknown_tag_is_literal() {
        // A closed bracket naming no style/attribute is content, not markup.
        assert_eq!(parse("array[0]").lines[0].plain(), "array[0]");
        assert_eq!(parse("[hello world]").lines[0].plain(), "[hello world]");
    }

    #[test]
    fn recognized_tag_still_applies_alongside_literal_bracket() {
        let text = parse("[red]x[/] array[0]");
        assert_eq!(text.lines[0].plain(), "x array[0]");
        assert_eq!(text.lines[0].spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn escaped_bracket_is_literal() {
        let text = parse("\\[bold\\]");
        assert_eq!(text.lines[0].plain(), "[bold]");
    }

    #[test]
    fn newlines_split_into_lines() {
        let text = parse("a\nb");
        assert_eq!(text.lines.len(), 2);
    }

    #[test]
    fn code_span_gets_code_style() {
        let text = parse("`x`");
        assert_eq!(text.lines[0].spans[0].style.fg, Some(Color::Cyan));
    }

    #[test]
    fn attribute_tokens_are_case_insensitive() {
        let text = parse("[BOLD]x[/]");
        assert!(text.lines[0].spans[0].style.attrs.contains(Attribute::BOLD));
    }

    #[test]
    fn background_keyword_is_case_insensitive() {
        let text = parse("[white ON blue]x[/]");
        assert_eq!(text.lines[0].spans[0].style.bg, Some(Color::Blue));
    }

    #[test]
    fn tag_inside_code_span_still_closes() {
        // Opening a tag inside a backtick span must not defeat the closing
        // backtick: after it, the tag's layer is popped, so trailing text no
        // longer carries the bold attribute.
        let text = parse("`[bold]a`b");
        let trailing = text.lines[0].spans.last().unwrap();
        assert_eq!(trailing.content, "b");
        assert!(!trailing.style.attrs.contains(Attribute::BOLD));
    }
}
