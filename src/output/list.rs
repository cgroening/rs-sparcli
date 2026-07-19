//! Bulleted and ordered lists with nesting and word wrapping.

use crate::core::geometry::Edges;
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::{Line, Span, Text};
use crate::core::theme::theme;
use crate::core::width::visible_width;
use crate::output::compose::pad;

/// The marker style for list items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Marker {
    /// A bullet glyph (`•`).
    #[default]
    Bullet,
    /// Arabic numbers (`1`, `2`, …).
    Number,
    /// Lowercase letters (`a`, `b`, …).
    AlphaLower,
    /// Uppercase letters (`A`, `B`, …).
    AlphaUpper,
    /// Lowercase roman numerals (`i`, `ii`, …).
    RomanLower,
    /// Uppercase roman numerals (`I`, `II`, …).
    RomanUpper,
}

/// One list entry with optional nested sub-list.
struct ListItem {
    content: Text,
    children: Option<List>,
}

/// A bulleted or ordered list.
///
/// # Examples
///
/// ```
/// use sparcli::{List, Renderable};
///
/// let out = List::new().item("First").item("Second").render(40);
/// assert!(out.plain().contains("First"));
/// assert!(out.plain().contains("Second"));
/// ```
pub struct List {
    marker: Marker,
    items: Vec<ListItem>,
    marker_style: Style,
    bullet: Option<String>,
    suffix: String,
    indent: u16,
    item_gap: u16,
    margin: Edges,
}

impl Default for List {
    fn default() -> Self {
        Self {
            marker: Marker::Bullet,
            items: Vec::new(),
            marker_style: theme().secondary,
            bullet: None,
            suffix: String::new(),
            indent: 0,
            item_gap: 0,
            margin: Edges::default(),
        }
    }
}

impl List {
    /// Creates an empty bulleted list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty list with the given marker style.
    ///
    /// Ordered markers default to a `.` suffix.
    pub fn ordered(marker: Marker) -> Self {
        let suffix = if matches!(marker, Marker::Bullet) {
            String::new()
        } else {
            ".".to_string()
        };
        Self {
            marker,
            suffix,
            ..Self::default()
        }
    }

    /// Adds a leaf item.
    #[must_use]
    pub fn item(mut self, content: impl Into<Text>) -> Self {
        self.items.push(ListItem {
            content: content.into(),
            children: None,
        });
        self
    }

    /// Adds an item with a nested sub-list.
    #[must_use]
    pub fn item_with(
        mut self,
        content: impl Into<Text>,
        children: List,
    ) -> Self {
        self.items.push(ListItem {
            content: content.into(),
            children: Some(children),
        });
        self
    }

    /// Sets a custom bullet glyph (bullet marker only).
    #[must_use]
    pub fn bullet(mut self, glyph: impl Into<String>) -> Self {
        self.bullet = Some(glyph.into());
        self
    }

    /// Sets the marker style.
    #[must_use]
    pub fn marker_style(mut self, style: Style) -> Self {
        self.marker_style = style;
        self
    }

    /// Sets the left indent in columns.
    #[must_use]
    pub fn indent(mut self, indent: u16) -> Self {
        self.indent = indent;
        self
    }

    /// Sets the number of blank lines between items.
    #[must_use]
    pub fn item_gap(mut self, gap: u16) -> Self {
        self.item_gap = gap;
        self
    }

    /// Sets the outer margin.
    #[must_use]
    pub fn margin(mut self, margin: Edges) -> Self {
        self.margin = margin;
        self
    }

    /// Returns the marker label for the item at `index`.
    fn marker_label(&self, index: usize) -> String {
        let core = match self.marker {
            Marker::Bullet => {
                let glyph = self.bullet.clone();
                return format!(
                    "{} ",
                    glyph.unwrap_or_else(|| theme().bullet().to_string())
                );
            }
            Marker::Number => (index + 1).to_string(),
            Marker::AlphaLower => to_alpha(index, false),
            Marker::AlphaUpper => to_alpha(index, true),
            Marker::RomanLower => to_roman(index + 1, false),
            Marker::RomanUpper => to_roman(index + 1, true),
        };
        format!("{core}{} ", self.suffix)
    }

    /// Renders the list into a flat list of lines (without margin).
    fn render_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        for (index, item) in self.items.iter().enumerate() {
            if index > 0 {
                for _ in 0..self.item_gap {
                    lines.push(Line::default());
                }
            }
            self.render_item(index, item, &mut lines);
        }
        lines
    }

    /// Renders one item (and its children) into `lines`.
    fn render_item(
        &self,
        index: usize,
        item: &ListItem,
        lines: &mut Vec<Line>,
    ) {
        let label = self.marker_label(index);
        let label_width = visible_width(&label);
        let indent = " ".repeat(self.indent as usize);
        let hang = " ".repeat(self.indent as usize + label_width);
        for (row, content_line) in item.content.lines.iter().enumerate() {
            let mut spans = Vec::new();
            if row == 0 {
                spans.push(Span::raw(indent.clone()));
                spans.push(Span::styled(label.clone(), self.marker_style));
            } else {
                spans.push(Span::raw(hang.clone()));
            }
            spans.extend(content_line.spans.iter().cloned());
            lines.push(Line::new(spans));
        }
        self.render_children(item, label_width, lines);
    }

    /// Renders nested children indented under their parent item.
    fn render_children(
        &self,
        item: &ListItem,
        label_width: usize,
        lines: &mut Vec<Line>,
    ) {
        let Some(children) = &item.children else {
            return;
        };
        let hang = self.indent as usize + label_width;
        for child_line in children.render_lines() {
            let mut spans = vec![Span::raw(" ".repeat(hang))];
            spans.extend(child_line.spans);
            lines.push(Line::new(spans));
        }
    }
}

impl Renderable for List {
    fn render(&self, _max_width: u16) -> Rendered {
        pad(&Rendered::new(self.render_lines()), self.margin)
    }
}

/// Converts a zero-based index to bijective base-26 letters.
fn to_alpha(index: usize, upper: bool) -> String {
    let mut value = index + 1;
    let mut chars = Vec::new();
    let base = if upper { b'A' } else { b'a' };
    while value > 0 {
        let rem = (value - 1) % 26;
        chars.push((base + rem as u8) as char);
        value = (value - 1) / 26;
    }
    chars.iter().rev().collect()
}

/// Converts a positive integer to a roman numeral.
fn to_roman(mut value: usize, upper: bool) -> String {
    const NUMERALS: [(usize, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for (amount, symbol) in NUMERALS {
        while value >= amount {
            out.push_str(symbol);
            value -= amount;
        }
    }
    if upper { out.to_ascii_uppercase() } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulleted_list_prefixes_each_item() {
        let list = List::new().item("a").item("b");
        let lines = list.render(40).plain_lines();
        assert_eq!(lines, vec!["• a", "• b"]);
    }

    #[test]
    fn numbered_list_counts_items() {
        let list = List::ordered(Marker::Number).item("x").item("y");
        let lines = list.render(40).plain_lines();
        assert_eq!(lines, vec!["1. x", "2. y"]);
    }

    #[test]
    fn nested_list_is_indented() {
        let child = List::new().item("child");
        let list = List::new().item_with("parent", child);
        let lines = list.render(40).plain_lines();
        assert_eq!(lines[0], "• parent");
        assert!(lines[1].starts_with("  • child"));
    }

    #[test]
    fn alpha_marker_wraps_after_z() {
        assert_eq!(to_alpha(0, false), "a");
        assert_eq!(to_alpha(25, false), "z");
        assert_eq!(to_alpha(26, false), "aa");
    }

    #[test]
    fn roman_marker_is_correct() {
        assert_eq!(to_roman(4, false), "iv");
        assert_eq!(to_roman(9, true), "IX");
    }
}
