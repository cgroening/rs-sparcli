//! Shared rendering helpers for text-entry prompts (block cursor, errors).

use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::theme::Theme;

/// Renders a labelled single-line field with a block cursor.
///
/// `display` is the already-prepared text (e.g. masked for passwords). The
/// cursor is drawn at character index `cursor`.
pub(crate) fn field_line(
    prompt: &str,
    display: &str,
    cursor: usize,
    style: Style,
    theme: &Theme,
) -> Line {
    let mut spans = Vec::new();
    if !prompt.is_empty() {
        spans.push(Span::styled(format!("{prompt} "), theme.title));
    }
    push_with_cursor(&mut spans, display, cursor, style, theme.cursor);
    Line::new(spans)
}

/// Renders a labelled value without any cursor (used for the final frame).
pub(crate) fn value_line(
    prompt: &str,
    display: &str,
    style: Style,
    theme: &Theme,
) -> Line {
    let mut spans = Vec::new();
    if !prompt.is_empty() {
        spans.push(Span::styled(format!("{prompt} "), theme.title));
    }
    if !display.is_empty() {
        spans.push(Span::styled(display.to_string(), style));
    }
    Line::new(spans)
}

/// Renders dim placeholder text with the cursor at the start.
pub(crate) fn placeholder_line(
    prompt: &str,
    placeholder: &str,
    theme: &Theme,
) -> Line {
    let mut spans = Vec::new();
    if !prompt.is_empty() {
        spans.push(Span::styled(format!("{prompt} "), theme.title));
    }
    spans.push(Span::styled(" ".to_string(), theme.cursor));
    spans.push(Span::styled(placeholder.to_string(), theme.secondary));
    Line::new(spans)
}

/// Renders an error message line in the theme's error style.
pub(crate) fn error_line(message: &str, theme: &Theme) -> Line {
    Line::styled(format!("  {message}"), theme.error)
}

/// Splits `display` at the cursor and pushes spans with a block cursor.
fn push_with_cursor(
    spans: &mut Vec<Span>,
    display: &str,
    cursor: usize,
    style: Style,
    cursor_style: Style,
) {
    let chars: Vec<char> = display.chars().collect();
    let before: String = chars.iter().take(cursor).collect();
    if !before.is_empty() {
        spans.push(Span::styled(before, style));
    }
    let at: String = chars
        .get(cursor)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    spans.push(Span::styled(at, cursor_style));
    if cursor + 1 < chars.len() {
        let after: String = chars.iter().skip(cursor + 1).collect();
        spans.push(Span::styled(after, style));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_line_includes_prompt_and_value() {
        let theme = Theme::default();
        let line = field_line("Name", "ab", 2, Style::new(), &theme);
        assert!(line.plain().starts_with("Name ab"));
    }

    #[test]
    fn cursor_in_middle_keeps_all_text() {
        let theme = Theme::default();
        let line = field_line("", "abc", 1, Style::new(), &theme);
        assert_eq!(line.plain(), "abc");
    }
}
