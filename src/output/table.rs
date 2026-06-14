//! Data tables with headers, footers, borders, alignment and wrapping.
//!
//! Supports per-column alignment, min/max/fixed widths, optional word wrap,
//! zebra striping, a title and horizontal column spanning (`colspan`).

use crate::core::border::{BorderChars, BorderType};
use crate::core::geometry::Align;
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::{Line, Span, Text};
use crate::core::theme::theme;
use crate::core::width::{truncate, wrap};
use crate::output::layout::pad_line;

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
}

impl Cell {
    /// Creates a cell from content.
    pub fn new(content: impl Into<Text>) -> Self {
        Self {
            content: content.into(),
            align: None,
            colspan: 1,
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

    /// Enables or disables the header row.
    #[must_use]
    pub fn header(mut self, show: bool) -> Self {
        self.header = show;
        self
    }

    /// Enables zebra striping of body rows.
    #[must_use]
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
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
        let widths = self.column_widths();
        Builder::new(self, &widths).build()
    }
}

impl Table {
    /// Computes the display width of each column.
    fn column_widths(&self) -> Vec<usize> {
        let mut widths = vec![0usize; self.columns.len()];
        for (index, column) in self.columns.iter().enumerate() {
            if self.header {
                widths[index] = column.header.width();
            }
        }
        for row in &self.rows {
            self.accumulate_row_widths(row, &mut widths);
        }
        for (index, column) in self.columns.iter().enumerate() {
            widths[index] = clamp_width(widths[index], column);
        }
        widths
    }

    /// Updates `widths` from the single-span cells of one row.
    fn accumulate_row_widths(&self, row: &Row, widths: &mut [usize]) {
        let mut col = 0;
        for cell in &row.cells {
            if col >= widths.len() {
                break;
            }
            if cell.colspan == 1 {
                widths[col] = widths[col].max(cell.content.width());
            }
            col += cell.colspan;
        }
    }
}

/// Clamps a natural width by a column's min/max/fixed constraints.
fn clamp_width(natural: usize, column: &Column) -> usize {
    if let Some(fixed) = column.fixed_width {
        return fixed;
    }
    let mut width = natural.max(column.min_width);
    if let Some(max) = column.max_width {
        width = width.min(max);
    }
    width.max(1)
}

/// Assembles the table lines from columns, rows and widths.
struct Builder<'a> {
    table: &'a Table,
    widths: &'a [usize],
    chars: BorderChars,
}

impl<'a> Builder<'a> {
    fn new(table: &'a Table, widths: &'a [usize]) -> Self {
        Self {
            table,
            widths,
            chars: table.border.chars(),
        }
    }

    /// Builds the full rendered table.
    fn build(&self) -> Rendered {
        let mut lines = Vec::new();
        self.push_title(&mut lines);
        self.push_border(&mut lines, Edge::Top);
        if self.table.header {
            self.push_header(&mut lines);
            self.push_border(&mut lines, Edge::Middle);
        }
        self.push_body(&mut lines);
        self.push_border(&mut lines, Edge::Bottom);
        Rendered::new(lines)
    }

    /// Pushes the optional title line above the table.
    fn push_title(&self, lines: &mut Vec<Line>) {
        let Some(title) = &self.table.title else {
            return;
        };
        let title_line = title.lines.first().cloned().unwrap_or_default();
        let width = self.total_width();
        lines.push(pad_line(
            title_line,
            width,
            Align::Center,
            self.table.header_style,
        ));
    }

    /// Pushes the header row built from the column headers.
    fn push_header(&self, lines: &mut Vec<Line>) {
        let cells: Vec<Cell> = self
            .table
            .columns
            .iter()
            .map(|column| Cell::new(column.header.clone()).align(column.align))
            .collect();
        let row = Row {
            cells,
            footer: false,
        };
        self.push_row(lines, &row, self.table.header_style, false);
    }

    /// Pushes all body and footer rows.
    fn push_body(&self, lines: &mut Vec<Line>) {
        let mut body_index = 0;
        let mut footer_started = false;
        for row in &self.table.rows {
            if row.footer && !footer_started {
                self.push_border(lines, Edge::Middle);
                footer_started = true;
            }
            let striped = self.table.striped && body_index % 2 == 1;
            let style = if striped {
                self.table.stripe_style
            } else {
                Style::new()
            };
            self.push_row(lines, row, style, striped);
            if !row.footer {
                body_index += 1;
                if self.table.row_separators && !self.is_last_body(row) {
                    self.push_border(lines, Edge::Middle);
                }
            }
        }
    }

    /// Returns whether `row` is the final body row.
    fn is_last_body(&self, row: &Row) -> bool {
        let last = self.table.rows.iter().rfind(|r| !r.footer);
        last.is_some_and(|candidate| std::ptr::eq(candidate, row))
    }

    /// Renders one row (possibly multiple physical lines) into `lines`.
    fn push_row(
        &self,
        lines: &mut Vec<Line>,
        row: &Row,
        style: Style,
        fill_bg: bool,
    ) {
        let cell_lines = self.cell_lines(row, style);
        let height = cell_lines.iter().map(Vec::len).max().unwrap_or(1);
        for physical in 0..height {
            lines.push(self.row_line(row, &cell_lines, physical, fill_bg));
        }
    }

    /// Wraps/truncates each cell into its display lines.
    fn cell_lines(&self, row: &Row, style: Style) -> Vec<Vec<Line>> {
        let mut out = Vec::new();
        let mut col = 0;
        for cell in &row.cells {
            let span_width = self.span_width(col, cell.colspan);
            let wrap = self.column_wraps(col);
            out.push(format_cell(cell, span_width, style, wrap));
            col += cell.colspan;
        }
        out
    }

    /// Returns whether the column at `index` wraps its content.
    fn column_wraps(&self, index: usize) -> bool {
        self.table.columns.get(index).is_some_and(|c| c.wrap)
    }

    /// Builds one physical line of a row.
    fn row_line(
        &self,
        row: &Row,
        cell_lines: &[Vec<Line>],
        physical: usize,
        fill_bg: bool,
    ) -> Line {
        let pad = self.table.pad as usize;
        let mut spans = vec![self.vbar()];
        let mut col = 0;
        for (cell_index, cell) in row.cells.iter().enumerate() {
            let width = self.span_width(col, cell.colspan);
            let align = cell.align.unwrap_or(self.align_for(col));
            let content = blank_or(cell_lines, cell_index, physical);
            let fill = if fill_bg {
                self.table.stripe_style
            } else {
                Style::new()
            };
            push_cell(&mut spans, content, width, pad, align, fill);
            spans.push(self.vbar());
            col += cell.colspan;
        }
        Line::new(spans)
    }

    /// Returns the alignment of the column at `index`.
    fn align_for(&self, index: usize) -> Align {
        self.table
            .columns
            .get(index)
            .map_or(Align::Left, |column| column.align)
    }

    /// Width spanned by `colspan` columns starting at `start`.
    fn span_width(&self, start: usize, colspan: usize) -> usize {
        let pad = self.table.pad as usize;
        let end = (start + colspan).min(self.widths.len());
        let content: usize = self.widths[start..end].iter().sum();
        let extra = colspan.saturating_sub(1) * (2 * pad + 1);
        content + extra
    }

    /// The total outer width of the table.
    fn total_width(&self) -> usize {
        let pad = self.table.pad as usize;
        let content: usize = self.widths.iter().sum();
        let cells = self.widths.len();
        content + cells * 2 * pad + cells + 1
    }

    /// A vertical border span.
    fn vbar(&self) -> Span {
        Span::styled(self.chars.vertical.to_string(), self.table.border_style)
    }

    /// Pushes a horizontal border line for the given edge.
    fn push_border(&self, lines: &mut Vec<Line>, edge: Edge) {
        if self.table.border.is_none() {
            return;
        }
        let (left, mid, right) = edge.corners(&self.chars);
        let pad = self.table.pad as usize;
        let mut content = String::new();
        content.push(left);
        for (index, width) in self.widths.iter().enumerate() {
            let segment = width + 2 * pad;
            content
                .push_str(&self.chars.horizontal.to_string().repeat(segment));
            if index + 1 < self.widths.len() {
                content.push(mid);
            }
        }
        content.push(right);
        lines.push(Line::styled(content, self.table.border_style));
    }
}

/// Which horizontal border line to draw.
#[derive(Clone, Copy)]
enum Edge {
    Top,
    Middle,
    Bottom,
}

impl Edge {
    /// Returns the (left, junction, right) glyphs for this edge.
    fn corners(self, chars: &BorderChars) -> (char, char, char) {
        match self {
            Edge::Top => (chars.top_left, chars.tee_down, chars.top_right),
            Edge::Middle => (chars.tee_right, chars.cross, chars.tee_left),
            Edge::Bottom => {
                (chars.bottom_left, chars.tee_up, chars.bottom_right)
            }
        }
    }
}

/// Formats one cell into styled lines, wrapping or truncating overflow.
fn format_cell(
    cell: &Cell,
    width: usize,
    style: Style,
    wrap_cell: bool,
) -> Vec<Line> {
    let mut out = Vec::new();
    for line in &cell.content.lines {
        let plain = line.plain();
        if line.width() <= width {
            out.push(restyle(line.clone(), style));
        } else if wrap_cell {
            for chunk in wrap(&plain, width) {
                out.push(Line::styled(chunk, style));
            }
        } else {
            let span_style = line.spans.first().map_or(style, |s| s.style);
            let cell_style = style.patch(span_style);
            out.push(Line::styled(truncate(&plain, width, "…"), cell_style));
        }
    }
    if out.is_empty() {
        out.push(Line::default());
    }
    out
}

/// Applies a base style underneath each span's own style.
fn restyle(line: Line, base: Style) -> Line {
    let spans = line
        .spans
        .into_iter()
        .map(|mut span| {
            span.style = base.patch(span.style);
            span
        })
        .collect();
    Line::new(spans)
}

/// Returns the cell's display line at `physical`, or a blank line.
fn blank_or(cell_lines: &[Vec<Line>], index: usize, physical: usize) -> Line {
    cell_lines
        .get(index)
        .and_then(|lines| lines.get(physical))
        .cloned()
        .unwrap_or_default()
}

/// Pushes one padded, aligned cell into `spans`.
fn push_cell(
    spans: &mut Vec<Span>,
    content: Line,
    width: usize,
    pad: usize,
    align: Align,
    fill: Style,
) {
    let truncated = clip(content, width);
    let padded = pad_line(truncated, width, align, fill);
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), fill));
    }
    spans.extend(padded.spans);
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), fill));
    }
}

/// Truncates a line to `width` columns, preserving the first span's style.
fn clip(line: Line, width: usize) -> Line {
    if line.width() <= width {
        return line;
    }
    let style = line.spans.first().map(|s| s.style).unwrap_or_default();
    Line::styled(truncate(&line.plain(), width, "…"), style)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn colspan_widens_a_cell() {
        let table = Table::new()
            .columns(["A", "B"])
            .row([Cell::new("wide").colspan(2)])
            .border(BorderType::Single);
        let lines = plain(&table.render(80));
        assert!(lines.iter().any(|l| l.contains("wide")));
    }
}
