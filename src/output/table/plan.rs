//! Cell placement and column-width computation for tables.
//!
//! Resolves rows into a grid honoring `colspan`/`rowspan` and derives each
//! column's display width. Consumed by [`render`](super::render).

use super::{Cell, Column, Row, Table};

/// A cell placed at a concrete column position (or a rowspan continuation).
pub(super) struct PlacedCell<'a> {
    pub(super) cell: Option<&'a Cell>,
    pub(super) start: usize,
    pub(super) colspan: usize,
}

/// One visual row: every column filled, including rowspan continuations.
pub(super) struct RowPlan<'a> {
    pub(super) cells: Vec<PlacedCell<'a>>,
    pub(super) footer: bool,
}

/// Resolves rows into a grid, honoring colspan and rowspan occupancy.
pub(super) fn build_plan(rows: &[Row], cols: usize) -> Vec<RowPlan<'_>> {
    let mut occupied = vec![0usize; cols];
    let mut plan = Vec::with_capacity(rows.len());
    for row in rows {
        let mut placed = Vec::new();
        let mut cells = row.cells.iter();
        let mut col = 0;
        while col < cols {
            if occupied[col] > 0 {
                occupied[col] -= 1;
                placed.push(PlacedCell {
                    cell: None,
                    start: col,
                    colspan: 1,
                });
                col += 1;
                continue;
            }
            match cells.next() {
                Some(cell) => {
                    let span = cell.colspan.min(cols - col).max(1);
                    if cell.rowspan > 1 {
                        for slot in occupied.iter_mut().skip(col).take(span) {
                            *slot = cell.rowspan - 1;
                        }
                    }
                    placed.push(PlacedCell {
                        cell: Some(cell),
                        start: col,
                        colspan: span,
                    });
                    col += span;
                }
                None => {
                    placed.push(PlacedCell {
                        cell: None,
                        start: col,
                        colspan: 1,
                    });
                    col += 1;
                }
            }
        }
        plan.push(RowPlan {
            cells: placed,
            footer: row.footer,
        });
    }
    plan
}

/// Computes the display width of each column from the placement plan.
pub(super) fn column_widths(table: &Table, plan: &[RowPlan]) -> Vec<usize> {
    let mut widths = vec![0usize; table.columns.len()];
    for (index, column) in table.columns.iter().enumerate() {
        if table.header {
            widths[index] = column.header.width();
        }
    }
    for row in plan {
        for placed in &row.cells {
            if let Some(cell) = placed.cell
                && placed.colspan == 1
            {
                widths[placed.start] =
                    widths[placed.start].max(cell.content.width());
            }
        }
    }
    for (index, column) in table.columns.iter().enumerate() {
        widths[index] = clamp_width(widths[index], column);
    }
    widths
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
