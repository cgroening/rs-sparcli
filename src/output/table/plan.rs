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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_width_prefers_a_fixed_width_over_everything() {
        let column = Column::new("h").fixed_width(5).min_width(20).max_width(2);
        assert_eq!(clamp_width(100, &column), 5);
    }

    #[test]
    fn clamp_width_applies_the_min_then_the_max() {
        let column = Column::new("h").min_width(4).max_width(8);
        assert_eq!(clamp_width(1, &column), 4, "raised to the minimum");
        assert_eq!(clamp_width(6, &column), 6, "left alone in between");
        assert_eq!(clamp_width(99, &column), 8, "capped at the maximum");
    }

    #[test]
    fn clamp_width_never_returns_zero() {
        // A zero-width column would render as an unusable sliver.
        assert_eq!(clamp_width(0, &Column::new("")), 1);
        assert_eq!(clamp_width(0, &Column::new("").max_width(0)), 1);
    }

    #[test]
    fn floor_is_at_least_one_column() {
        assert_eq!(floor(&Column::new("h")), 1);
        assert_eq!(floor(&Column::new("h").min_width(3)), 3);
    }

    #[test]
    fn a_fixed_column_is_eligible_for_no_shrink_pass() {
        let fixed = Column::new("h").fixed_width(4);
        assert!(!eligible(&fixed, ShrinkPass::Wrapping));
        assert!(!eligible(&fixed, ShrinkPass::NonWrapping));
    }

    #[test]
    fn each_shrink_pass_claims_its_own_columns() {
        let wrapping = Column::new("h").wrap();
        let plain = Column::new("h");
        assert!(eligible(&wrapping, ShrinkPass::Wrapping));
        assert!(!eligible(&wrapping, ShrinkPass::NonWrapping));
        assert!(!eligible(&plain, ShrinkPass::Wrapping));
        assert!(eligible(&plain, ShrinkPass::NonWrapping));
    }

    #[test]
    fn widest_shrinkable_breaks_ties_towards_the_lower_index() {
        // Determinism matters: the same table must always shrink the same way.
        let table = Table::new().columns(["a", "b"]);
        let widths = [7, 7];
        let picked =
            widest_shrinkable(&table, &widths, ShrinkPass::NonWrapping);
        assert_eq!(picked, Some(0));
    }

    #[test]
    fn widest_shrinkable_skips_columns_already_at_their_floor() {
        let table = Table::new()
            .column(Column::new("a").min_width(5))
            .column(Column::new("b"));
        let widths = [5, 3];
        let picked =
            widest_shrinkable(&table, &widths, ShrinkPass::NonWrapping);
        assert_eq!(picked, Some(1), "the wider column is at its floor");
    }

    #[test]
    fn widest_shrinkable_gives_up_when_nothing_has_slack() {
        let table = Table::new().column(Column::new("a").fixed_width(4));
        let widths = [4];
        assert_eq!(
            widest_shrinkable(&table, &widths, ShrinkPass::NonWrapping),
            None
        );
    }

    #[test]
    fn shrink_reports_the_width_it_could_not_recover() {
        // Both columns bottom out at 1, so only 8 of the 20 can be given up.
        let table = Table::new().columns(["a", "b"]);
        let mut widths = [5, 5];
        let left = shrink(&table, &mut widths, 20, ShrinkPass::NonWrapping);
        assert_eq!(widths, [1, 1]);
        assert_eq!(left, 12);
    }

    #[test]
    fn a_table_that_already_fits_is_left_untouched() {
        let table = Table::new().columns(["a", "b"]);
        let mut widths = [5, 5];
        fit_to_width(&table, &mut widths, 200);
        assert_eq!(widths, [5, 5]);
    }

    #[test]
    fn wrapping_columns_give_up_width_before_truncating_ones() {
        // A wrapping column only reflows, so it loses width first; the plain
        // column keeps its content intact for as long as possible.
        let table = Table::new()
            .column(Column::new("a").wrap())
            .column(Column::new("b"));
        let mut widths = [10, 10];
        // Overhead for two columns at pad 1 is 2*(2*1+1)+1 = 7.
        fit_to_width(&table, &mut widths, 7 + 16);
        assert_eq!(widths, [6, 10]);
    }

    #[test]
    fn a_fixed_column_keeps_its_width_while_others_shrink() {
        let table = Table::new()
            .column(Column::new("a").fixed_width(8))
            .column(Column::new("b"));
        let mut widths = [8, 10];
        fit_to_width(&table, &mut widths, 7 + 14);
        assert_eq!(widths, [8, 6]);
    }

    #[test]
    fn a_table_too_narrow_even_at_the_floors_overflows_rather_than_vanish() {
        let table = Table::new()
            .column(Column::new("a").min_width(6))
            .column(Column::new("b").min_width(6));
        let mut widths = [6, 6];
        fit_to_width(&table, &mut widths, 10);
        assert_eq!(widths, [6, 6], "floors win over the budget");
    }

    #[test]
    fn build_plan_fills_every_column_of_every_row() {
        let table = Table::new().columns(["a", "b", "c"]).row(["1"]);
        let plan = build_plan(&table.rows, 3);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].cells.len(), 3, "short rows are padded out");
        assert!(plan[0].cells[0].cell.is_some());
        assert!(plan[0].cells[1].cell.is_none());
    }

    #[test]
    fn build_plan_honors_colspan_and_clamps_it_to_the_grid() {
        let table = Table::new()
            .columns(["a", "b", "c"])
            .row([Cell::new("wide").colspan(9)]);
        let plan = build_plan(&table.rows, 3);
        assert_eq!(plan[0].cells.len(), 1);
        assert_eq!(plan[0].cells[0].colspan, 3, "clamped to the column count");
    }

    #[test]
    fn build_plan_reserves_the_spanned_slots_of_a_rowspan() {
        let table = Table::new()
            .columns(["a", "b"])
            .row([Cell::new("tall").rowspan(2), Cell::new("x")])
            .row(["y"]);
        let plan = build_plan(&table.rows, 2);
        // The second row's first column is occupied by the rowspan above, so
        // "y" is pushed into the second column.
        assert!(plan[1].cells[0].cell.is_none());
        assert!(plan[1].cells[1].cell.is_some());
    }
}
