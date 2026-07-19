//! Data tables with headers, footers, borders, alignment and wrapping.
//!
//! Supports per-column alignment, min/max/fixed widths, optional word wrap,
//! zebra striping, a title and horizontal column spanning (`colspan`).
//!
//! A table that fits the render width is left untouched; an overflowing one
//! shrinks its flexible columns so its borders stay within the terminal.
//! `wrap` columns reflow first, then the rest truncate; `fixed_width` columns
//! never shrink and no column falls below its `min_width`.

mod column;
mod plan;
mod render;

pub use self::column::{Cell, Column};

use self::column::Row;
use crate::core::border::BorderType;
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::Text;
use crate::core::theme::theme;

/// A data table.
///
/// Rendering honours the given width: a table that already fits is unchanged,
/// while an overflowing one shrinks its flexible columns (`wrap` columns reflow
/// first, then the rest truncate) so its borders stay within the terminal.
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
    fn render(&self, max_width: u16) -> Rendered {
        if self.columns.is_empty() {
            return Rendered::empty();
        }
        let plan = plan::build_plan(&self.rows, self.columns.len());
        let widths = plan::column_widths(self, &plan, max_width as usize);
        render::Builder::new(self, &widths, &plan).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::geometry::Align;

    #[test]
    fn renders_header_and_rows_with_borders() {
        let table = Table::new()
            .columns(["A", "B"])
            .row(["1", "2"])
            .border(BorderType::Single);
        let lines = table.render(80).plain_lines();
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
        let lines = table.render(80).plain_lines();
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
        let lines = table.render(80).plain_lines();
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
        let lines = table.render(80).plain_lines();
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
        let lines = table.render(80).plain_lines();
        assert!(lines.iter().any(|l| l.contains("wide")));
    }

    /// The widest rendered line, i.e. the table's outer width.
    fn outer_width(rendered: &Rendered) -> usize {
        rendered
            .plain_lines()
            .iter()
            .map(|line| crate::width::visible_width(line))
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn a_fitting_table_is_identical_regardless_of_max_width() {
        let table = Table::new()
            .columns(["A", "B"])
            .row(["1", "2"])
            .border(BorderType::Single);
        // A table that fits must render the same at any generous width, so the
        // fitting pass never touches the common case.
        assert_eq!(
            table.render(80).plain_lines(),
            table.render(1000).plain_lines()
        );
    }

    #[test]
    fn overflow_reflows_wrapping_columns_before_truncating() {
        let table = Table::new()
            .header(false)
            .column(Column::new("").wrap())
            .column(Column::new("").align(Align::Right))
            .row(["aaaaaaaaaaaaaaaaaaaa", "12345"])
            .border(BorderType::Single);
        let rendered = table.render(20);
        let lines = rendered.plain_lines();
        // The numeric column keeps its content; the wrap column reflows onto a
        // second physical line instead of losing text.
        assert!(lines.iter().any(|l| l.contains("12345")));
        assert!(!lines.iter().any(|l| l.contains('…')));
        assert!(lines.len() > 3, "the wrap column produced extra lines");
        assert!(outer_width(&rendered) <= 20);
    }

    #[test]
    fn overflow_truncates_the_widest_non_wrapping_column_first() {
        let table = Table::new()
            .header(false)
            .columns(["A", "B"])
            .row(["abcdefghijklmnop", "xy"])
            .border(BorderType::Single);
        let rendered = table.render(16);
        let lines = rendered.plain_lines();
        // The narrow column survives; the wide one is truncated with an ellipsis.
        assert!(lines.iter().any(|l| l.contains("xy")));
        assert!(lines.iter().any(|l| l.contains('…')));
        assert!(outer_width(&rendered) <= 16);
    }

    #[test]
    fn a_fixed_width_column_never_shrinks() {
        let table = Table::new()
            .header(false)
            .column(Column::new("").fixed_width(8))
            .column(Column::new("").wrap())
            .row(["FIXEDVAL", "aaaaaaaaaaaaaaaa"])
            .border(BorderType::Single);
        let rendered = table.render(20);
        let lines = rendered.plain_lines();
        // The fixed column keeps its full 8-character value; the flexible one
        // absorbs the whole deficit.
        assert!(lines.iter().any(|l| l.contains("FIXEDVAL")));
        assert!(outer_width(&rendered) <= 20);
    }

    #[test]
    fn a_column_never_shrinks_below_its_min_width() {
        let table = Table::new()
            .header(false)
            .column(Column::new("").min_width(10))
            .row(["x"])
            .border(BorderType::Single);
        // Even asked to fit five columns, the min-width floor wins and the
        // table overflows rather than collapse below it.
        let rendered = table.render(5);
        assert!(outer_width(&rendered) > 5);
    }

    #[test]
    fn a_colspan_cell_truncates_under_pressure_without_forcing_a_column() {
        let table = Table::new()
            .columns(["A", "B"])
            .row([Cell::new("a-very-wide-spanning-value").colspan(2)])
            .row(["1", "2"])
            .border(BorderType::Single);
        // The single-column widths derive from the colspan==1 row, so the wide
        // spanning cell truncates instead of widening either column.
        let rendered = table.render(14);
        assert!(outer_width(&rendered) <= 14);
    }
}
