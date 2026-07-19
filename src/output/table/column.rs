//! The pieces a table is built from: columns, cells and rows.
//!
//! These carry the layout intent only. [`super::plan`] turns them into column
//! widths and [`super::render`] draws them; the fields are therefore visible
//! within the `table` module but never outside it.

use crate::core::geometry::Align;
use crate::core::text::Text;

/// A single table column definition.
pub struct Column {
    pub(super) header: Text,
    pub(super) align: Align,
    pub(super) min_width: usize,
    pub(super) max_width: Option<usize>,
    pub(super) fixed_width: Option<usize>,
    pub(super) wrap: bool,
}

impl Column {
    /// Creates a column with the given header.
    pub fn new(header: impl Into<Text>) -> Self {
        Self {
            header: header.into(),
            align: Align::Left,
            min_width: 0,
            max_width: None,
            fixed_width: None,
            wrap: false,
        }
    }

    /// Sets the column alignment.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Sets the minimum column width.
    #[must_use]
    pub fn min_width(mut self, width: usize) -> Self {
        self.min_width = width;
        self
    }

    /// Sets the maximum column width (content is wrapped or truncated).
    #[must_use]
    pub fn max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Sets a fixed column width.
    #[must_use]
    pub fn fixed_width(mut self, width: usize) -> Self {
        self.fixed_width = Some(width);
        self
    }

    /// Enables word wrapping instead of truncation.
    #[must_use]
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
}

impl From<&str> for Column {
    fn from(value: &str) -> Self {
        Column::new(value)
    }
}

impl From<String> for Column {
    fn from(value: String) -> Self {
        Column::new(value)
    }
}

/// A single table cell.
pub struct Cell {
    pub(super) content: Text,
    pub(super) align: Option<Align>,
    pub(super) colspan: usize,
    pub(super) rowspan: usize,
}

impl Cell {
    /// Creates a cell from content.
    pub fn new(content: impl Into<Text>) -> Self {
        Self {
            content: content.into(),
            align: None,
            colspan: 1,
            rowspan: 1,
        }
    }

    /// Overrides the cell alignment.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }

    /// Spans this cell across `columns` columns.
    #[must_use]
    pub fn colspan(mut self, columns: usize) -> Self {
        self.colspan = columns.max(1);
        self
    }

    /// Spans this cell across `rows` rows.
    ///
    /// Cells in the rows below skip the spanned column(s); content sits in the
    /// top row. Best paired with the default (no row separators).
    #[must_use]
    pub fn rowspan(mut self, rows: usize) -> Self {
        self.rowspan = rows.max(1);
        self
    }
}

impl From<&str> for Cell {
    fn from(value: &str) -> Self {
        Cell::new(value)
    }
}

impl From<String> for Cell {
    fn from(value: String) -> Self {
        Cell::new(value)
    }
}

impl From<Text> for Cell {
    fn from(value: Text) -> Self {
        Cell::new(value)
    }
}

/// A table row of cells.
pub(super) struct Row {
    pub(super) cells: Vec<Cell>,
    pub(super) footer: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_column_is_left_aligned_and_flexible() {
        let column = Column::new("Name");
        assert_eq!(column.align, Align::Left);
        assert_eq!(column.min_width, 0);
        assert_eq!(column.max_width, None);
        assert_eq!(column.fixed_width, None);
        assert!(!column.wrap);
    }

    #[test]
    fn column_builders_set_their_own_field() {
        let column = Column::new("Name")
            .align(Align::Right)
            .min_width(4)
            .max_width(20)
            .fixed_width(8)
            .wrap();
        assert_eq!(column.align, Align::Right);
        assert_eq!(column.min_width, 4);
        assert_eq!(column.max_width, Some(20));
        assert_eq!(column.fixed_width, Some(8));
        assert!(column.wrap);
    }

    #[test]
    fn a_column_can_be_built_from_a_string() {
        assert_eq!(Column::from("Name").header.lines[0].plain(), "Name");
        assert_eq!(
            Column::from("Name".to_string()).header.lines[0].plain(),
            "Name"
        );
    }

    #[test]
    fn a_new_cell_spans_exactly_one_row_and_column() {
        let cell = Cell::new("x");
        assert_eq!(cell.colspan, 1);
        assert_eq!(cell.rowspan, 1);
        assert_eq!(cell.align, None);
    }

    #[test]
    fn a_span_of_zero_is_clamped_to_one() {
        // A zero span would make the layout drop the cell entirely.
        assert_eq!(Cell::new("x").colspan(0).colspan, 1);
        assert_eq!(Cell::new("x").rowspan(0).rowspan, 1);
    }

    #[test]
    fn a_cell_can_be_built_from_a_string_or_text() {
        assert_eq!(Cell::from("x").content.lines[0].plain(), "x");
        assert_eq!(Cell::from("x".to_string()).content.lines[0].plain(), "x");
        assert_eq!(Cell::from(Text::raw("x")).content.lines[0].plain(), "x");
    }
}
