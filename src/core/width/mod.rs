//! Display-width math: visible width, ANSI stripping, truncation, wrapping.
//!
//! All widths are measured in terminal columns and are aware of wide glyphs
//! (CJK, emoji), zero-width combining marks and ANSI escape sequences. The
//! plain-string helpers live here; their style-preserving counterparts for
//! [`Line`](crate::core::text::Line) live in the `line` submodule.

mod line;

pub use self::line::{truncate_line, wrap_line};

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

/// The ASCII escape byte that starts ANSI sequences.
const ESC: char = '\u{1b}';

/// Inclusive range of CSI final bytes (`0x40..=0x7e`) that end a sequence.
const CSI_FINAL_MIN: char = '\u{40}';
const CSI_FINAL_MAX: char = '\u{7e}';

/// Returns the visible column width of `text`, ignoring ANSI escapes.
pub fn visible_width(text: &str) -> usize {
    if !text.contains(ESC) {
        return UnicodeWidthStr::width(text);
    }
    UnicodeWidthStr::width(strip_ansi(text).as_str())
}

/// Removes all ANSI escape sequences (CSI and OSC) from `text`.
pub fn strip_ansi(text: &str) -> String {
    if !text.contains(ESC) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != ESC {
            out.push(ch);
            continue;
        }
        skip_escape_sequence(&mut chars);
    }
    out
}

/// Consumes one escape sequence after the leading `ESC` was read.
fn skip_escape_sequence(chars: &mut std::iter::Peekable<std::str::Chars>) {
    match chars.peek() {
        Some('[') => skip_csi(chars),
        Some(']') => skip_osc(chars),
        _ => {
            chars.next();
        }
    }
}

/// Skips a CSI sequence (`ESC [ … final-byte`).
fn skip_csi(chars: &mut std::iter::Peekable<std::str::Chars>) {
    chars.next(); // consume '['
    for ch in chars.by_ref() {
        if (CSI_FINAL_MIN..=CSI_FINAL_MAX).contains(&ch) {
            break;
        }
    }
}

/// Skips an OSC sequence (`ESC ] … BEL` or `ESC ] … ESC \`).
fn skip_osc(chars: &mut std::iter::Peekable<std::str::Chars>) {
    chars.next(); // consume ']'
    while let Some(ch) = chars.next() {
        if ch == '\u{7}' {
            break;
        }
        if ch == ESC && chars.peek() == Some(&'\\') {
            chars.next();
            break;
        }
    }
}

/// Truncates `text` to at most `max_cols` columns, appending `ellipsis` when
/// content was dropped.
///
/// Never splits a wide glyph; assumes `text` contains no ANSI escapes. The
/// result never exceeds `max_cols` columns: a `max_cols` of zero yields the
/// empty string, and an ellipsis wider than `max_cols` is clamped to fit.
pub fn truncate(text: &str, max_cols: usize, ellipsis: &str) -> String {
    if max_cols == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(text) <= max_cols {
        return text.to_string();
    }
    let ellipsis_width = UnicodeWidthStr::width(ellipsis);
    if ellipsis_width >= max_cols {
        return ellipsis.chars().take(max_cols).collect();
    }
    let budget = max_cols - ellipsis_width;
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + w > budget {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push_str(ellipsis);
    out
}

/// Wraps `text` to lines no wider than `width` columns (word-aware).
///
/// Words longer than `width` are hard-split. A `width` of zero yields the
/// original text as a single line.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    for raw_line in text.split('\n') {
        wrap_single_line(raw_line, width, &mut lines);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Wraps one logical line, pushing the resulting visual lines into `out`.
fn wrap_single_line(line: &str, width: usize, out: &mut Vec<String>) {
    let mut current = String::new();
    let mut current_width = 0;
    for word in line.split_whitespace() {
        let word_width = UnicodeWidthStr::width(word);
        let sep = usize::from(!current.is_empty());
        if current_width + sep + word_width > width && !current.is_empty() {
            out.push(std::mem::take(&mut current));
            current_width = 0;
        }
        if word_width > width {
            flush_long_word(word, width, &mut current, &mut current_width, out);
            continue;
        }
        if !current.is_empty() {
            current.push(' ');
            current_width += 1;
        }
        current.push_str(word);
        current_width += word_width;
    }
    out.push(current);
}

/// Hard-splits a word wider than `width` across multiple lines.
fn flush_long_word(
    word: &str,
    width: usize,
    current: &mut String,
    current_width: &mut usize,
    out: &mut Vec<String>,
) {
    if !current.is_empty() {
        out.push(std::mem::take(current));
        *current_width = 0;
    }
    for ch in word.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if *current_width + w > width && !current.is_empty() {
            out.push(std::mem::take(current));
            *current_width = 0;
        }
        current.push(ch);
        *current_width += w;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_width_counts_wide_glyphs_as_two() {
        assert_eq!(visible_width("ab"), 2);
        assert_eq!(visible_width("中"), 2);
    }

    #[test]
    fn visible_width_ignores_ansi_escapes() {
        assert_eq!(visible_width("\u{1b}[31mred\u{1b}[0m"), 3);
    }

    #[test]
    fn strip_ansi_removes_csi_and_osc() {
        assert_eq!(strip_ansi("\u{1b}[1mhi\u{1b}[0m"), "hi");
        let link = "\u{1b}]8;;http://x\u{1b}\\t\u{1b}]8;;\u{1b}\\";
        assert_eq!(strip_ansi(link), "t");
    }

    #[test]
    fn truncate_appends_ellipsis_only_when_needed() {
        assert_eq!(truncate("hello", 10, "…"), "hello");
        assert_eq!(truncate("hello world", 7, "…"), "hello …");
    }

    #[test]
    fn truncate_does_not_split_wide_glyphs() {
        // Each CJK glyph is two columns; budget 3 fits one glyph + ellipsis.
        assert_eq!(truncate("中文字", 3, "…"), "中…");
    }

    #[test]
    fn truncate_with_zero_columns_is_empty() {
        assert_eq!(truncate("hello", 0, "…"), "");
    }

    #[test]
    fn truncate_clamps_ellipsis_wider_than_max() {
        // The ellipsis alone exceeds the budget, so it is clamped to fit.
        assert_eq!(truncate("hello", 2, "..."), "..");
        assert_eq!(visible_width(&truncate("hello", 2, "...")), 2);
    }

    #[test]
    fn strip_ansi_handles_non_letter_csi_final_byte() {
        // `@` (0x40) is a valid CSI final byte and must end the sequence.
        assert_eq!(strip_ansi("\u{1b}[1@x"), "x");
    }

    #[test]
    fn wrap_breaks_on_word_boundaries() {
        let lines = wrap("the quick brown fox", 9);
        assert_eq!(lines, vec!["the quick", "brown fox"]);
    }

    #[test]
    fn wrap_hard_splits_overlong_words() {
        let lines = wrap("abcdefgh", 3);
        assert_eq!(lines, vec!["abc", "def", "gh"]);
    }
}
