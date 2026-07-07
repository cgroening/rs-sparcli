//! Line assembly for tables: borders, padding, alignment and wrapping.
//!
//! Turns a [`Table`], its computed widths and the placement
//! [`plan`](super::plan) into the final [`Rendered`] lines.

use crate::core::border::BorderChars;
use crate::core::geometry::Align;
use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::width::{truncate, wrap};
use crate::output::layout::pad_line;
use crate::output::table::plan::{PlacedCell, RowPlan};
use crate::output::table::{Cell, Table};

/// Assembles the table lines from columns, the placement plan and widths.
pub(super) struct Builder<'a> {
    table: &'a Table,
    widths: &'a [usize],
    plan: &'a [RowPlan<'a>],
    chars: BorderChars,
}

impl<'a> Builder<'a> {
    pub(super) fn new(
        table: &'a Table,
        widths: &'a [usize],
        plan: &'a [RowPlan<'a>],
    ) -> Self {
        Self {
            table,
            widths,
            plan,
            chars: table.border.chars(),
        }
    }

    /// Builds the full rendered table.
    pub(super) fn build(&self) -> Rendered {
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
            self.table.title_style,
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
        let placed: Vec<PlacedCell> = cells
            .iter()
            .enumerate()
            .map(|(index, cell)| PlacedCell {
                cell: Some(cell),
                start: index,
                colspan: 1,
            })
            .collect();
        self.push_row(lines, &placed, self.table.header_style, false);
    }

    /// Pushes all body and footer rows from the placement plan.
    fn push_body(&self, lines: &mut Vec<Line>) {
        let last_body = self.plan.iter().rposition(|r| !r.footer);
        let mut body_index = 0;
        let mut footer_started = false;
        for (index, row) in self.plan.iter().enumerate() {
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
            self.push_row(lines, &row.cells, style, striped);
            if !row.footer {
                body_index += 1;
                if self.table.row_separators && Some(index) != last_body {
                    self.push_border(lines, Edge::Middle);
                }
            }
        }
    }

    /// Renders one placed row (possibly multiple physical lines).
    fn push_row(
        &self,
        lines: &mut Vec<Line>,
        placed: &[PlacedCell],
        style: Style,
        fill_bg: bool,
    ) {
        let cell_lines = self.cell_lines(placed, style);
        let height = cell_lines.iter().map(Vec::len).max().unwrap_or(1).max(1);
        for physical in 0..height {
            lines.push(self.row_line(placed, &cell_lines, physical, fill_bg));
        }
    }

    /// Wraps/truncates each placed cell into its display lines.
    fn cell_lines(
        &self,
        placed: &[PlacedCell],
        style: Style,
    ) -> Vec<Vec<Line>> {
        placed
            .iter()
            .map(|slot| match slot.cell {
                None => vec![Line::default()],
                Some(cell) => {
                    let width = self.span_width(slot.start, slot.colspan);
                    let wrap = self.column_wraps(slot.start);
                    format_cell(cell, width, style, wrap)
                }
            })
            .collect()
    }

    /// Returns whether the column at `index` wraps its content.
    fn column_wraps(&self, index: usize) -> bool {
        self.table.columns.get(index).is_some_and(|c| c.wrap)
    }

    /// Builds one physical line of a placed row.
    fn row_line(
        &self,
        placed: &[PlacedCell],
        cell_lines: &[Vec<Line>],
        physical: usize,
        fill_bg: bool,
    ) -> Line {
        let pad = self.table.pad as usize;
        let mut spans = vec![self.vbar()];
        for (slot_index, slot) in placed.iter().enumerate() {
            let width = self.span_width(slot.start, slot.colspan);
            let align = slot
                .cell
                .and_then(|c| c.align)
                .unwrap_or(self.align_for(slot.start));
            let content = blank_or(cell_lines, slot_index, physical);
            let fill = if fill_bg {
                self.table.stripe_style
            } else {
                Style::new()
            };
            push_cell(&mut spans, content, width, pad, align, fill);
            spans.push(self.vbar());
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
