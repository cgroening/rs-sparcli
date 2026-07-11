//! Side-by-side multi-column layout of rendered blocks.

use crate::core::border::BorderType;
use crate::core::geometry::{Align, VAlign};
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::theme::theme;
use crate::output::layout::{blank_line, pad_line};

/// One column in a [`Columns`] layout.
struct ColumnItem {
    block: Rendered,
    align: Align,
}

/// A horizontal arrangement of rendered blocks.
///
/// # Examples
///
/// ```
/// use sparcli::{Columns, Renderable, Text};
///
/// let out = Columns::new()
///     .add(&Text::raw("left"), 6)
///     .add(&Text::raw("right"), 6)
///     .render(20);
/// assert!(out.plain().contains("left"));
/// assert!(out.plain().contains("right"));
/// ```
pub struct Columns {
    items: Vec<ColumnItem>,
    gap: u16,
    separator: Option<BorderType>,
    separator_style: Style,
    valign: VAlign,
}

impl Default for Columns {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            gap: 2,
            separator: None,
            separator_style: theme().secondary,
            valign: VAlign::Top,
        }
    }
}

impl Columns {
    /// Creates an empty columns layout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a column from any renderable, laid out at `width` columns.
    #[must_use]
    pub fn add(mut self, content: &impl Renderable, width: u16) -> Self {
        self.items.push(ColumnItem {
            block: content.render(width),
            align: Align::Left,
        });
        self
    }

    /// Adds a column from an already rendered block.
    #[must_use]
    pub fn add_rendered(mut self, block: Rendered) -> Self {
        self.items.push(ColumnItem {
            block,
            align: Align::Left,
        });
        self
    }

    /// Sets the alignment of the most recently added column.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        if let Some(last) = self.items.last_mut() {
            last.align = align;
        }
        self
    }

    /// Sets the gap between columns in spaces.
    #[must_use]
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Draws a vertical separator of the given border type between columns.
    #[must_use]
    pub fn separator(mut self, border: BorderType) -> Self {
        self.separator = Some(border);
        self
    }

    /// Sets the vertical alignment of shorter columns.
    #[must_use]
    pub fn valign(mut self, valign: VAlign) -> Self {
        self.valign = valign;
        self
    }
}

impl Renderable for Columns {
    fn render(&self, _max_width: u16) -> Rendered {
        if self.items.is_empty() {
            return Rendered::empty();
        }
        let height = self
            .items
            .iter()
            .map(|i| i.block.height())
            .max()
            .unwrap_or(0);
        let widths: Vec<usize> =
            self.items.iter().map(|i| i.block.width()).collect();
        let aligned: Vec<Vec<Line>> = self
            .items
            .iter()
            .zip(&widths)
            .map(|(item, &width)| self.align_column(item, width, height))
            .collect();
        let lines = (0..height)
            .map(|row| self.compose_row(&aligned, &widths, row))
            .collect();
        Rendered::new(lines)
    }
}

impl Columns {
    /// Vertically aligns and pads a column's lines to a uniform block.
    fn align_column(
        &self,
        item: &ColumnItem,
        width: usize,
        height: usize,
    ) -> Vec<Line> {
        let pad_top = match self.valign {
            VAlign::Top => 0,
            VAlign::Middle => (height - item.block.height()) / 2,
            VAlign::Bottom => height - item.block.height(),
        };
        let mut lines = Vec::with_capacity(height);
        for _ in 0..pad_top {
            lines.push(blank_line(width, Style::new()));
        }
        for line in &item.block.lines {
            lines.push(pad_line(line.clone(), width, item.align, Style::new()));
        }
        while lines.len() < height {
            lines.push(blank_line(width, Style::new()));
        }
        lines
    }

    /// Composes one output row by joining the columns at index `row`.
    fn compose_row(
        &self,
        aligned: &[Vec<Line>],
        widths: &[usize],
        row: usize,
    ) -> Line {
        let mut spans = Vec::new();
        for (index, column) in aligned.iter().enumerate() {
            if index > 0 {
                self.push_gap(&mut spans);
            }
            let blank = blank_line(widths[index], Style::new());
            let line = column.get(row).unwrap_or(&blank);
            spans.extend(line.spans.iter().cloned());
        }
        Line::new(spans)
    }

    /// Pushes the inter-column gap and optional separator glyph.
    fn push_gap(&self, spans: &mut Vec<Span>) {
        let half = self.gap / 2;
        match self.separator {
            None => spans.push(Span::raw(" ".repeat(self.gap as usize))),
            Some(border) => {
                let glyph = border.chars().vertical;
                spans.push(Span::raw(" ".repeat(half as usize)));
                spans.push(Span::styled(
                    glyph.to_string(),
                    self.separator_style,
                ));
                let rest = self.gap.saturating_sub(half + 1);
                spans.push(Span::raw(" ".repeat(rest as usize)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(rendered: &Rendered) -> Vec<String> {
        rendered.lines.iter().map(Line::plain).collect()
    }

    #[test]
    fn places_blocks_side_by_side() {
        let left = Rendered::new(vec![Line::raw("a"), Line::raw("b")]);
        let right = Rendered::new(vec![Line::raw("x"), Line::raw("y")]);
        let columns =
            Columns::new().add_rendered(left).add_rendered(right).gap(2);
        let lines = plain(&columns.render(80));
        assert_eq!(lines[0], "a  x");
        assert_eq!(lines[1], "b  y");
    }

    #[test]
    fn pads_shorter_columns() {
        let left = Rendered::new(vec![Line::raw("a"), Line::raw("b")]);
        let right = Rendered::new(vec![Line::raw("x")]);
        let columns =
            Columns::new().add_rendered(left).add_rendered(right).gap(1);
        let lines = plain(&columns.render(80));
        assert_eq!(lines[0], "a x");
        assert_eq!(lines[1].trim_end(), "b");
    }

    #[test]
    fn draws_separator_between_columns() {
        let left = Rendered::new(vec![Line::raw("a")]);
        let right = Rendered::new(vec![Line::raw("b")]);
        let columns = Columns::new()
            .add_rendered(left)
            .add_rendered(right)
            .gap(3)
            .separator(BorderType::Single);
        let lines = plain(&columns.render(80));
        assert!(lines[0].contains('│'));
    }
}
