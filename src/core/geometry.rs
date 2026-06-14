//! Layout primitives: alignment, box-model edges and titles.

use crate::core::style::Style;
use crate::core::text::Text;

/// Horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Align to the left edge.
    #[default]
    Left,
    /// Center horizontally.
    Center,
    /// Align to the right edge.
    Right,
}

/// Vertical alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VAlign {
    /// Align to the top edge.
    #[default]
    Top,
    /// Center vertically.
    Middle,
    /// Align to the bottom edge.
    Bottom,
}

/// Where a title sits relative to its frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// Top edge.
    #[default]
    Top,
    /// Bottom edge.
    Bottom,
}

/// Box-model spacing (padding or margin) in terminal cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Edges {
    /// Top spacing in rows.
    pub top: u16,
    /// Right spacing in columns.
    pub right: u16,
    /// Bottom spacing in rows.
    pub bottom: u16,
    /// Left spacing in columns.
    pub left: u16,
}

impl Edges {
    /// Equal spacing on all four sides.
    pub fn all(value: u16) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Symmetric spacing: `vertical` for top/bottom, `horizontal` for sides.
    pub fn symmetric(vertical: u16, horizontal: u16) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Total horizontal spacing (`left + right`).
    pub fn horizontal(self) -> u16 {
        self.left + self.right
    }

    /// Total vertical spacing (`top + bottom`).
    pub fn vertical(self) -> u16 {
        self.top + self.bottom
    }
}

/// A framed title, e.g. on a panel, rule or table.
#[derive(Debug, Clone, Default)]
pub struct Title {
    /// The title content (rich text).
    pub content: Text,
    /// Horizontal placement along the edge.
    pub align: Align,
    /// Which edge the title sits on.
    pub position: Position,
    /// Spaces of padding on each side of the title text.
    pub pad: u16,
}

impl Title {
    /// Creates a left-aligned top title with single-space padding.
    pub fn new(content: impl Into<Text>) -> Self {
        Self {
            content: content.into(),
            align: Align::Left,
            position: Position::Top,
            pad: 1,
        }
    }

    /// Sets the horizontal alignment.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Sets the edge the title sits on.
    #[must_use]
    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Sets the padding on each side of the title text.
    #[must_use]
    pub fn pad(mut self, pad: u16) -> Self {
        self.pad = pad;
        self
    }

    /// Applies a style to every span of the title.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        for line in &mut self.content.lines {
            for span in &mut line.spans {
                span.style = span.style.patch(style);
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_all_sets_every_side() {
        let edges = Edges::all(2);
        assert_eq!(edges.horizontal(), 4);
        assert_eq!(edges.vertical(), 4);
    }

    #[test]
    fn edges_symmetric_splits_axes() {
        let edges = Edges::symmetric(1, 3);
        assert_eq!(edges.top, 1);
        assert_eq!(edges.left, 3);
    }

    #[test]
    fn title_defaults_to_top_left() {
        let title = Title::new("hello");
        assert_eq!(title.align, Align::Left);
        assert_eq!(title.position, Position::Top);
    }
}
