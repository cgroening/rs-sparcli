//! Aligned key-value lists.

use crate::core::geometry::Edges;
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::{Line, Span, Text};
use crate::core::theme::theme;
use crate::core::width::{visible_width, wrap};
use crate::output::compose::pad;

/// One key-value pair.
struct Pair {
    key: String,
    value: Text,
}

/// A list of aligned key-value pairs.
///
/// # Examples
///
/// ```
/// use sparcli::{KeyValue, Renderable};
///
/// let out = KeyValue::new()
///     .add("name", "sparcli")
///     .add("license", "MIT")
///     .render(40);
/// assert!(out.plain().contains("sparcli"));
/// ```
pub struct KeyValue {
    pairs: Vec<Pair>,
    separator: String,
    key_width: Option<u16>,
    key_style: Style,
    value_style: Style,
    item_gap: u16,
    wrap_values: bool,
    margin: Edges,
}

impl Default for KeyValue {
    fn default() -> Self {
        Self {
            pairs: Vec::new(),
            separator: "  ".to_string(),
            key_width: None,
            key_style: Style::new().bold(),
            value_style: theme().secondary,
            item_gap: 0,
            wrap_values: false,
            margin: Edges::default(),
        }
    }
}

impl KeyValue {
    /// Creates an empty key-value list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a key-value pair.
    #[must_use]
    pub fn add(
        mut self,
        key: impl Into<String>,
        value: impl Into<Text>,
    ) -> Self {
        self.pairs.push(Pair {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    /// Sets the separator between key and value.
    #[must_use]
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    /// Sets a fixed key column width.
    #[must_use]
    pub fn key_width(mut self, width: u16) -> Self {
        self.key_width = Some(width);
        self
    }

    /// Sets the key style.
    #[must_use]
    pub fn key_style(mut self, style: Style) -> Self {
        self.key_style = style;
        self
    }

    /// Sets the value style.
    #[must_use]
    pub fn value_style(mut self, style: Style) -> Self {
        self.value_style = style;
        self
    }

    /// Sets the number of blank lines between pairs.
    #[must_use]
    pub fn item_gap(mut self, gap: u16) -> Self {
        self.item_gap = gap;
        self
    }

    /// Enables wrapping of long values.
    #[must_use]
    pub fn wrap_values(mut self, wrap: bool) -> Self {
        self.wrap_values = wrap;
        self
    }

    /// Sets the outer margin.
    #[must_use]
    pub fn margin(mut self, margin: Edges) -> Self {
        self.margin = margin;
        self
    }

    /// Returns the effective key column width.
    fn resolved_key_width(&self) -> usize {
        match self.key_width {
            Some(width) => width as usize,
            None => self
                .pairs
                .iter()
                .map(|pair| visible_width(&pair.key))
                .max()
                .unwrap_or(0),
        }
    }
}

impl Renderable for KeyValue {
    fn render(&self, max_width: u16) -> Rendered {
        let key_width = self.resolved_key_width();
        let prefix_width = key_width + visible_width(&self.separator);
        let value_width =
            (max_width as usize).saturating_sub(prefix_width).max(1);
        let mut lines = Vec::new();
        for (index, pair) in self.pairs.iter().enumerate() {
            if index > 0 {
                push_gap(&mut lines, self.item_gap);
            }
            self.push_pair(&mut lines, pair, key_width, value_width);
        }
        pad(&Rendered::new(lines), self.margin)
    }
}

impl KeyValue {
    /// Renders one pair into `lines`, wrapping the value when enabled.
    fn push_pair(
        &self,
        lines: &mut Vec<Line>,
        pair: &Pair,
        key_width: usize,
        value_width: usize,
    ) {
        let value_lines = self.value_lines(pair, value_width);
        for (row, value_line) in value_lines.into_iter().enumerate() {
            let key_cell = if row == 0 {
                pair.key.clone()
            } else {
                String::new()
            };
            lines.push(self.compose_line(&key_cell, key_width, value_line));
        }
    }

    /// Splits a value into one or more display lines.
    fn value_lines(&self, pair: &Pair, value_width: usize) -> Vec<Line> {
        if !self.wrap_values {
            return pair.value.lines.clone();
        }
        let mut out = Vec::new();
        for line in &pair.value.lines {
            for chunk in wrap(&line.plain(), value_width) {
                out.push(Line::styled(chunk, self.value_style));
            }
        }
        out
    }

    /// Composes a key cell, separator and value line into one line.
    fn compose_line(
        &self,
        key: &str,
        key_width: usize,
        value_line: Line,
    ) -> Line {
        let key_pad = key_width.saturating_sub(visible_width(key));
        let mut spans = vec![
            Span::styled(key.to_string(), self.key_style),
            Span::raw(" ".repeat(key_pad)),
            Span::raw(self.separator.clone()),
        ];
        for mut span in value_line.spans {
            span.style = self.value_style.patch(span.style);
            spans.push(span);
        }
        Line::new(spans)
    }
}

/// Pushes `count` blank lines.
fn push_gap(lines: &mut Vec<Line>, count: u16) {
    for _ in 0..count {
        lines.push(Line::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(rendered: &Rendered) -> Vec<String> {
        rendered.lines.iter().map(Line::plain).collect()
    }

    #[test]
    fn aligns_keys_to_the_widest() {
        let kv = KeyValue::new().add("a", "1").add("name", "2");
        let lines = plain(&kv.render(40));
        assert_eq!(lines[0], "a     1");
        assert_eq!(lines[1], "name  2");
    }

    #[test]
    fn wraps_long_values_when_enabled() {
        let kv = KeyValue::new()
            .wrap_values(true)
            .add("k", "one two three four");
        let lines = plain(&kv.render(10));
        assert!(lines.len() > 1);
    }
}
