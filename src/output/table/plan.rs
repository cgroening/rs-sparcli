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
///
/// The result never exceeds `max_width`: a table that already fits is returned
/// unchanged, while an overflowing one has its flexible columns shrunk (see
/// [`fit_to_width`]).
pub(super) fn column_widths(
    table: &Table,
    plan: &[RowPlan],
    max_width: usize,
) -> Vec<usize> {
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
    fit_to_width(table, &mut widths, max_width);
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

/// Which columns a shrink pass may take width from.
#[derive(Clone, Copy)]
enum ShrinkPass {
    /// Columns marked `wrap`, which reflow onto more lines without losing text.
    Wrapping,
    /// The remaining flexible columns, which truncate their content.
    NonWrapping,
}

/// Shrinks flexible columns until the table fits `max_width`.
///
/// A table that already fits is left untouched, so its layout is identical to
/// the pre-fitting behaviour. When it overflows, wrapping columns give up width
/// first (they only reflow), then the rest (they truncate). `fixed_width`
/// columns never shrink, and no column falls below its `min_width` floor, so a
/// table too narrow even at the floors overflows rather than turn unreadable.
fn fit_to_width(table: &Table, widths: &mut [usize], max_width: usize) {
    let pad = table.pad as usize;
    let cells = widths.len();
    let overhead = cells * (2 * pad + 1) + 1;
    let budget = max_width.saturating_sub(overhead);
    let content: usize = widths.iter().sum();
    if content <= budget {
        return;
    }
    let mut deficit = content - budget;
    deficit = shrink(table, widths, deficit, ShrinkPass::Wrapping);
    shrink(table, widths, deficit, ShrinkPass::NonWrapping);
}

/// Takes one cell at a time from the widest eligible column, returning the
/// width still to be recovered once no eligible column has slack left.
fn shrink(
    table: &Table,
    widths: &mut [usize],
    mut deficit: usize,
    pass: ShrinkPass,
) -> usize {
    while deficit > 0 {
        let Some(index) = widest_shrinkable(table, widths, pass) else {
            break;
        };
        widths[index] -= 1;
        deficit -= 1;
    }
    deficit
}

/// The eligible column with the most slack above its floor, ties to the lowest
/// index so the outcome is deterministic.
fn widest_shrinkable(
    table: &Table,
    widths: &[usize],
    pass: ShrinkPass,
) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    for (index, &width) in widths.iter().enumerate() {
        let column = &table.columns[index];
        if !eligible(column, pass) || width <= floor(column) {
            continue;
        }
        // Replace only on a strictly wider column, so equal widths keep the
        // earlier index.
        if best.is_none_or(|(_, best_width)| width > best_width) {
            best = Some((index, width));
        }
    }
    best.map(|(index, _)| index)
}

/// Whether `column` may give up width during `pass`.
fn eligible(column: &Column, pass: ShrinkPass) -> bool {
    if column.fixed_width.is_some() {
        return false;
    }
    match pass {
        ShrinkPass::Wrapping => column.wrap,
        ShrinkPass::NonWrapping => !column.wrap,
    }
}

/// The narrowest a column may be shrunk to, mirroring [`clamp_width`].
fn floor(column: &Column) -> usize {
    column.min_width.max(1)
}
