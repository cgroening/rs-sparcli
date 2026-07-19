//! Style-preserving wrapping and truncation of [`Line`]s.
//!
//! The plain-string [`wrap`](super::wrap) and [`truncate`](super::truncate)
//! return `String`s and therefore drop every span's style and hyperlink. The
//! helpers here keep both, which is what filled widgets need: a hole in a
//! styled run shows up as a gap in the background, not just as lost color.

use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::width::{truncate, visible_width, wrap};

use unicode_width::UnicodeWidthChar;

/// Wraps `line` to lines no wider than `width` columns, preserving styles.
///
/// Word-aware and consistent with [`wrap`](super::wrap): runs of whitespace
/// collapse to a single space and words wider than `width` are hard-split. A
/// word that straddles a span boundary stays whole and keeps both styles. A
/// `width` of zero yields the line unchanged.
///
/// # Examples
///
/// ```
/// use sparcli::Line;
/// use sparcli::width::wrap_line;
///
/// let wrapped = wrap_line(&Line::raw("the quick brown fox"), 9);
/// assert_eq!(wrapped.len(), 2);
/// assert_eq!(wrapped[0].plain(), "the quick");
/// ```
pub fn wrap_line(line: &Line, width: usize) -> Vec<Line> {
    if width == 0 {
        return vec![line.clone()];
    }
    if line.spans.len() <= 1 {
        return wrap_single_span(line, width);
    }
    pack_words(&split_words(line), width)
}

/// Truncates `line` to at most `max_cols` columns, preserving styles.
///
/// Appends `ellipsis` in the style of the last surviving span when content
/// was dropped. Never splits a wide glyph, and the result never exceeds
/// `max_cols` columns.
///
/// # Examples
///
/// ```
/// use sparcli::Line;
/// use sparcli::width::truncate_line;
///
/// let short = truncate_line(&Line::raw("hello world"), 7, "…");
/// assert_eq!(short.plain(), "hello …");
/// ```
pub fn truncate_line(line: &Line, max_cols: usize, ellipsis: &str) -> Line {
    if max_cols == 0 {
        return Line::default();
    }
    if line.width() <= max_cols {
        return line.clone();
    }
    let ellipsis_width = visible_width(ellipsis);
    let fallback = line
        .spans
        .first()
        .map(|span| span.style)
        .unwrap_or_default();
    if ellipsis_width >= max_cols {
        let clipped = truncate(ellipsis, max_cols, "");
        return Line::new(vec![Span::styled(clipped, fallback)]);
    }
    let mut spans = clip_spans(line, max_cols - ellipsis_width);
    let style = spans.last().map(|span| span.style).unwrap_or(fallback);
    spans.push(Span::styled(ellipsis.to_string(), style));
    Line::new(merge_adjacent(spans))
}

/// Wraps a line of at most one span by delegating to the plain-string
/// [`wrap`](super::wrap), so single-span input behaves identically.
fn wrap_single_span(line: &Line, width: usize) -> Vec<Line> {
    let Some(span) = line.spans.first() else {
        return vec![Line::default()];
    };
    wrap(&span.content, width)
        .into_iter()
        .map(|chunk| {
            Line::new(vec![Span {
                content: chunk,
                style: span.style,
                link: span.link.clone(),
            }])
        })
        .collect()
}

/// One styled fragment of a word; a word may span several styles.
#[derive(Clone)]
struct WordPart {
    /// The fragment's text.
    content: String,
    /// The style inherited from the originating span.
    style: Style,
    /// The hyperlink inherited from the originating span.
    link: Option<String>,
}

/// A whitespace-delimited word, possibly assembled from several spans.
#[derive(Clone, Default)]
struct Word {
    /// The fragments the word is made of, in order.
    parts: Vec<WordPart>,
    /// The word's total column width.
    width: usize,
}

impl Word {
    /// Appends one character carrying the style of `span`.
    fn push(&mut self, ch: char, span: &Span) {
        self.width += UnicodeWidthChar::width(ch).unwrap_or(0);
        if let Some(last) = self.parts.last_mut()
            && last.style == span.style
            && last.link == span.link
        {
            last.content.push(ch);
            return;
        }
        self.parts.push(WordPart {
            content: ch.to_string(),
            style: span.style,
            link: span.link.clone(),
        });
    }

    /// Returns `true` if the word has no characters.
    fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Converts the word's parts into spans.
    fn to_spans(&self) -> Vec<Span> {
        self.parts.iter().map(part_to_span).collect()
    }
}

/// Splits `line` into words, keeping words that straddle a span boundary
/// whole so they are never wrapped apart.
fn split_words(line: &Line) -> Vec<Word> {
    let mut words = Vec::new();
    let mut current = Word::default();
    for span in &line.spans {
        for ch in span.content.chars() {
            if ch.is_whitespace() {
                push_word(&mut words, &mut current);
                continue;
            }
            current.push(ch, span);
        }
    }
    push_word(&mut words, &mut current);
    words
}

/// Moves a non-empty `current` word into `words`.
fn push_word(words: &mut Vec<Word>, current: &mut Word) {
    if current.is_empty() {
        return;
    }
    words.push(std::mem::take(current));
}

/// The line currently being assembled by [`pack_words`].
#[derive(Default)]
struct Pending {
    /// The spans collected so far.
    spans: Vec<Span>,
    /// Their total column width.
    width: usize,
}

impl Pending {
    /// Returns `true` if no span has been collected yet.
    fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Emits the collected spans as a line and starts a fresh one.
    fn flush(&mut self, out: &mut Vec<Line>) {
        let spans = std::mem::take(&mut self.spans);
        self.width = 0;
        out.push(Line::new(merge_adjacent(spans)));
    }
}

/// Greedily packs `words` into lines of at most `width` columns.
fn pack_words(words: &[Word], width: usize) -> Vec<Line> {
    let mut out = Vec::new();
    let mut pending = Pending::default();
    for word in words {
        let separator = usize::from(!pending.is_empty());
        if pending.width + separator + word.width > width && !pending.is_empty()
        {
            pending.flush(&mut out);
        }
        if word.width > width {
            hard_split_word(word, width, &mut pending, &mut out);
            continue;
        }
        push_separator(&mut pending);
        pending.spans.extend(word.to_spans());
        pending.width += word.width;
    }
    pending.flush(&mut out);
    out
}

/// Adds the single space that joins two words on the same line.
fn push_separator(pending: &mut Pending) {
    if pending.is_empty() {
        return;
    }
    let style = pending
        .spans
        .last()
        .map(|span| span.style)
        .unwrap_or_default();
    pending.spans.push(Span::styled(" ", style));
    pending.width += 1;
}

/// Splits a word wider than `width` across several lines, keeping styles.
fn hard_split_word(
    word: &Word,
    width: usize,
    pending: &mut Pending,
    out: &mut Vec<Line>,
) {
    if !pending.is_empty() {
        pending.flush(out);
    }
    for part in &word.parts {
        for ch in part.content.chars() {
            let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if pending.width + char_width > width && !pending.is_empty() {
                pending.flush(out);
            }
            pending.spans.push(Span {
                content: ch.to_string(),
                style: part.style,
                link: part.link.clone(),
            });
            pending.width += char_width;
        }
    }
}

/// Clips `line`'s spans to at most `budget` columns without splitting a wide
/// glyph.
fn clip_spans(line: &Line, budget: usize) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut used = 0;
    for span in &line.spans {
        let mut content = String::new();
        for ch in span.content.chars() {
            let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + char_width > budget {
                break;
            }
            content.push(ch);
            used += char_width;
        }
        if !content.is_empty() {
            spans.push(Span {
                content,
                style: span.style,
                link: span.link.clone(),
            });
        }
        if used >= budget {
            break;
        }
    }
    spans
}

/// Merges consecutive spans that share style and hyperlink.
fn merge_adjacent(spans: Vec<Span>) -> Vec<Span> {
    let mut merged: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans {
        if let Some(last) = merged.last_mut()
            && last.style == span.style
            && last.link == span.link
        {
            last.content.push_str(&span.content);
            continue;
        }
        merged.push(span);
    }
    merged
}

/// Converts one word fragment into a span.
fn part_to_span(part: &WordPart) -> Span {
    Span {
        content: part.content.clone(),
        style: part.style,
        link: part.link.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::style::Color;

    /// A two-span line whose spans carry distinct colors.
    fn two_colors(first: &str, second: &str) -> Line {
        Line::new(vec![
            Span::styled(first, Style::new().fg(Color::Red)),
            Span::styled(second, Style::new().fg(Color::Blue)),
        ])
    }

    #[test]
    fn wrap_line_matches_wrap_for_single_span_lines() {
        let text = "the quick brown fox jumps";
        let wrapped: Vec<String> = wrap_line(&Line::raw(text), 9)
            .iter()
            .map(Line::plain)
            .collect();
        assert_eq!(wrapped, wrap(text, 9));
    }

    #[test]
    fn wrap_line_preserves_span_styles() {
        let line = two_colors("alpha ", "beta gamma");
        let wrapped = wrap_line(&line, 11);
        assert_eq!(wrapped.len(), 2);
        assert_eq!(wrapped[0].plain(), "alpha beta");
        // The joining space merges into the preceding red span, so the line
        // keeps exactly one span per original style.
        assert_eq!(wrapped[0].spans.len(), 2);
        assert_eq!(wrapped[0].spans[0].style.fg, Some(Color::Red));
        assert_eq!(wrapped[0].spans[1].style.fg, Some(Color::Blue));
    }

    #[test]
    fn wrap_line_keeps_words_split_across_spans_whole() {
        // Without word-aware span handling this would wrap as "err or".
        let wrapped = wrap_line(&two_colors("err", "or"), 10);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].plain(), "error");
    }

    #[test]
    fn wrap_line_hard_splits_overlong_words_keeping_style() {
        let wrapped = wrap_line(&two_colors("abcd", "efgh"), 3);
        let plain: Vec<String> = wrapped.iter().map(Line::plain).collect();
        assert_eq!(plain, vec!["abc", "def", "gh"]);
        assert_eq!(wrapped[0].spans[0].style.fg, Some(Color::Red));
        assert_eq!(wrapped[2].spans[0].style.fg, Some(Color::Blue));
    }

    #[test]
    fn wrap_line_preserves_hyperlinks() {
        let line = Line::new(vec![
            Span::raw("see "),
            Span::raw("docs").link("https://example.org"),
        ]);
        let wrapped = wrap_line(&line, 20);
        let linked = wrapped[0]
            .spans
            .iter()
            .find(|span| span.content == "docs")
            .expect("the linked span survives a wrap that does not split it");
        assert_eq!(linked.link.as_deref(), Some("https://example.org"));
    }

    #[test]
    fn wrap_line_with_zero_width_returns_the_line_unchanged() {
        let line = two_colors("alpha ", "beta");
        assert_eq!(wrap_line(&line, 0), vec![line]);
    }

    #[test]
    fn truncate_line_appends_ellipsis_and_keeps_style() {
        let line = two_colors("alpha ", "beta gamma");
        let short = truncate_line(&line, 9, "…");
        assert_eq!(short.plain(), "alpha be…");
        assert_eq!(short.spans[0].style.fg, Some(Color::Red));
        let last = short.spans.last().expect("truncation keeps some content");
        assert_eq!(last.style.fg, Some(Color::Blue));
    }

    #[test]
    fn truncate_line_leaves_fitting_lines_untouched() {
        let line = two_colors("ab", "cd");
        assert_eq!(truncate_line(&line, 10, "…"), line);
    }

    #[test]
    fn truncate_line_never_exceeds_max_cols() {
        // Each CJK glyph is two columns, so a budget of 3 fits one plus the
        // one-column ellipsis.
        let line = Line::raw("中文字");
        let short = truncate_line(&line, 3, "…");
        assert_eq!(short.plain(), "中…");
        assert_eq!(short.width(), 3);
    }

    #[test]
    fn truncate_line_clamps_an_ellipsis_wider_than_the_budget() {
        let short = truncate_line(&Line::raw("hello"), 2, "...");
        assert_eq!(short.width(), 2);
    }
}
