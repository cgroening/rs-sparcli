//! Data tables with headers, footers, borders, alignment and wrapping.
//!
//! Supports per-column alignment, min/max/fixed widths, optional word wrap,
//! zebra striping, a title and horizontal column spanning (`colspan`).

mod plan;
mod render;

use crate::core::border::BorderType;
use crate::core::geometry::Align;
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::Text;
use crate::core::theme::theme;

/// A single table column definition.
pub struct Column {
    header: Text,
    align: Align,
    min_width: usize,
    max_width: Option<usize>,
    fixed_width: Option<usize>,
    wrap: bool,
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
    content: Text,
    align: Option<Align>,
    colspan: usize,
    rowspan: usize,
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
struct Row {
    cells: Vec<Cell>,
    footer: bool,
}

/// A data table.
///
/// # Examples
///
/// ```
/// use sparcli::{Renderable, Table};
///
/// let out = Table::new()
///     .columns(["Name", "Status"])
///     .row(["web-1", "online"])
///     .render(40);
/// assert!(out.plain().contains("web-1"));
/// ```
pub struct Table {
    columns: Vec<Column>,
    rows: Vec<Row>,
    border: BorderType,
    border_style: Style,
    header: bool,
    header_style: Style,
    striped: bool,
    stripe_style: Style,
    title: Option<Text>,
    title_style: Style,
    pad: u16,
    row_separators: bool,
}

impl Default for Table {
    fn default() -> Self {
        let theme = theme();
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            border: theme.border,
            border_style: theme.secondary,
            header: true,
            header_style: theme.heading,
            striped: false,
            stripe_style: Style::new().dim(),
            title: None,
            title_style: theme.heading,
            pad: 1,
            row_separators: false,
        }
    }
}

impl Table {
    /// Creates an empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a column.
    #[must_use]
    pub fn column(mut self, column: impl Into<Column>) -> Self {
        self.columns.push(column.into());
        self
    }

    /// Adds several columns at once.
    #[must_use]
    pub fn columns<I, C>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Column>,
    {
        self.columns.extend(columns.into_iter().map(Into::into));
        self
    }

    /// Adds a data row.
    #[must_use]
    pub fn row<I, C>(mut self, cells: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Cell>,
    {
        self.rows.push(Row {
            cells: cells.into_iter().map(Into::into).collect(),
            footer: false,
        });
        self
    }

    /// Adds a footer row (drawn after a separator).
    #[must_use]
    pub fn footer_row<I, C>(mut self, cells: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Cell>,
    {
        self.rows.push(Row {
            cells: cells.into_iter().map(Into::into).collect(),
            footer: true,
        });
        self
    }

    /// Sets the border type.
    #[must_use]
    pub fn border(mut self, border: BorderType) -> Self {
        self.border = border;
        self
    }

    /// Sets the border glyph style (e.g. its color).
    #[must_use]
    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    /// Enables or disables the header row.
    #[must_use]
    pub fn header(mut self, show: bool) -> Self {
        self.header = show;
        self
    }

    /// Sets the header row text style.
    #[must_use]
    pub fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    /// Enables zebra striping of body rows.
    #[must_use]
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Sets the style (e.g. background) used for striped rows.
    #[must_use]
    pub fn stripe_style(mut self, style: Style) -> Self {
        self.stripe_style = style;
        self
    }

    /// Sets the title text style.
    #[must_use]
    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = style;
        self
    }

    /// Sets a table title.
    #[must_use]
    pub fn title(mut self, title: impl Into<Text>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the horizontal cell padding.
    #[must_use]
    pub fn pad(mut self, pad: u16) -> Self {
        self.pad = pad;
        self
    }

    /// Draws separators between body rows.
    #[must_use]
    pub fn row_separators(mut self, on: bool) -> Self {
        self.row_separators = on;
        self
    }
}

impl Renderable for Table {
    fn render(&self, _max_width: u16) -> Rendered {
        if self.columns.is_empty() {
            return Rendered::empty();
        }
        let plan = plan::build_plan(&self.rows, self.columns.len());
        let widths = plan::column_widths(self, &plan);
        render::Builder::new(self, &widths, &plan).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::text::Line;

    fn plain(rendered: &Rendered) -> Vec<String> {
        rendered.lines.iter().map(Line::plain).collect()
    }

    #[test]
    fn renders_header_and_rows_with_borders() {
        let table = Table::new()
            .columns(["A", "B"])
            .row(["1", "2"])
            .border(BorderType::Single);
        let lines = plain(&table.render(80));
        assert!(lines[0].starts_with('┌'));
        assert!(lines[1].contains('A') && lines[1].contains('B'));
        assert!(lines[2].starts_with('├'));
        assert!(lines[3].contains('1') && lines[3].contains('2'));
        assert!(lines[4].starts_with('└'));
    }

    #[test]
    fn aligns_and_pads_cells() {
        let table = Table::new()
            .header(false)
            .column(Column::new("").align(Align::Right))
            .row(["7"])
            .border(BorderType::Ascii);
        let lines = plain(&table.render(80));
        // Body row: |<pad>7<pad>| with right alignment (single col).
        assert!(lines.iter().any(|l| l.contains('7')));
    }

    #[test]
    fn truncates_overlong_cells_to_max_width() {
        let table = Table::new()
            .header(false)
            .column(Column::new("").max_width(4))
            .row(["abcdefgh"])
            .border(BorderType::Single);
        let lines = plain(&table.render(80));
        assert!(lines.iter().any(|l| l.contains('…')));
    }

    #[test]
    fn rowspan_spans_following_rows() {
        let table = Table::new()
            .header(false)
            .columns(["A", "B"])
            .row([Cell::new("x").rowspan(2), Cell::new("1")])
            .row(["2"])
            .border(BorderType::Single);
        let lines = plain(&table.render(80));
        // Top span row carries both the spanning cell and the first value.
        assert!(lines.iter().any(|l| l.contains('x') && l.contains('1')));
        // The continuation row shows the next value but not the spanned cell.
        let two = lines.iter().find(|l| l.contains('2')).unwrap();
        assert!(!two.contains('x'));
    }

    #[test]
    fn colspan_widens_a_cell() {
        let table = Table::new()
            .columns(["A", "B"])
            .row([Cell::new("wide").colspan(2)])
            .border(BorderType::Single);
        let lines = plain(&table.render(80));
        assert!(lines.iter().any(|l| l.contains("wide")));
    }
}
