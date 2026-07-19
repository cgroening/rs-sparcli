//! Cursor and scroll-window bookkeeping for the list prompts.
//!
//! [`Select`](crate::input::select::Select) and
//! [`FuzzySelect`](crate::input::fuzzy::FuzzySelect) both show a window onto a
//! longer list, and both have to keep the cursor inside it. That is one rule,
//! so it lives in one place: [`SelectionCursor`] owns the index, the scroll
//! offset and the window size, and every movement goes through it.
//!
//! The movement contract, mirroring what the widgets promise their users:
//! stepping wraps at both ends (or clamps when cycling is off), a page jump
//! and a jump to either end always clamp, and the offset follows the cursor so
//! the list only scrolls once the cursor reaches an edge.

/// A cursor into a list, together with the visible window onto it.
pub(crate) struct SelectionCursor {
    index: usize,
    offset: usize,
    visible: usize,
    len: usize,
    cycle: bool,
}

impl SelectionCursor {
    /// Creates a cursor over `len` items showing `visible` rows at a time.
    ///
    /// `cycle` decides whether stepping past either end wraps around.
    pub(crate) fn new(len: usize, visible: usize, cycle: bool) -> Self {
        Self {
            index: 0,
            offset: 0,
            visible: visible.max(1),
            len,
            cycle,
        }
    }

    /// Returns the current index; `0` for an empty list.
    pub(crate) fn index(&self) -> usize {
        self.index
    }

    /// Returns the index range currently visible, as `start..end`.
    pub(crate) fn window(&self) -> std::ops::Range<usize> {
        let end = (self.offset + self.visible).min(self.len);
        self.offset..end
    }

    /// Points the cursor at `index`, clamped, and scrolls it into view.
    pub(crate) fn jump_to(&mut self, index: usize) {
        self.index = index.min(self.last());
        self.follow();
    }

    /// Replaces the item count, keeping the cursor and window in range.
    ///
    /// Used by the fuzzy prompt, whose list shrinks and grows as the query
    /// changes.
    pub(crate) fn set_len(&mut self, len: usize) {
        self.len = len;
        self.jump_to(self.index);
    }

    /// Resets to the first item, scrolled to the top.
    pub(crate) fn reset(&mut self) {
        self.index = 0;
        self.offset = 0;
    }

    /// Moves the cursor by `delta` items, wrapping or clamping per config.
    pub(crate) fn step(&mut self, delta: isize) {
        if self.is_empty() {
            return;
        }
        let len = self.len as isize;
        let target = self.index as isize + delta;
        let next = if self.cycle {
            target.rem_euclid(len)
        } else {
            target.clamp(0, len - 1)
        };
        self.index = next as usize;
        self.follow();
    }

    /// Moves the cursor by `pages` screen pages, always clamping at the ends.
    ///
    /// A page jump clamps even when stepping cycles: wrapping from the top
    /// straight to the bottom of a long list reads as a lost position rather
    /// than as navigation.
    pub(crate) fn page(&mut self, pages: isize) {
        if self.is_empty() {
            return;
        }
        let delta = pages.saturating_mul(self.visible as isize);
        let target = (self.index as isize).saturating_add(delta);
        self.index = target.clamp(0, self.len as isize - 1) as usize;
        self.follow();
    }

    /// Returns whether the list is empty.
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the last valid index, or `0` for an empty list.
    fn last(&self) -> usize {
        self.len.saturating_sub(1)
    }

    /// Scrolls the window so the cursor sits inside it.
    fn follow(&mut self) {
        if self.index < self.offset {
            self.offset = self.index;
        } else if self.index >= self.offset + self.visible {
            self.offset = self.index + 1 - self.visible;
        }
        let max_offset = self.len.saturating_sub(self.visible);
        self.offset = self.offset.min(max_offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor(len: usize) -> SelectionCursor {
        SelectionCursor::new(len, 3, true)
    }

    #[test]
    fn a_new_cursor_starts_at_the_top() {
        let cursor = cursor(10);
        assert_eq!(cursor.index(), 0);
        assert_eq!(cursor.window(), 0..3);
    }

    #[test]
    fn stepping_past_the_end_wraps_to_the_start() {
        let mut cursor = cursor(3);
        cursor.step(1);
        cursor.step(1);
        assert_eq!(cursor.index(), 2);
        cursor.step(1);
        assert_eq!(cursor.index(), 0);
    }

    #[test]
    fn stepping_before_the_start_wraps_to_the_end() {
        let mut cursor = cursor(3);
        cursor.step(-1);
        assert_eq!(cursor.index(), 2);
    }

    #[test]
    fn a_non_cycling_cursor_clamps_at_both_ends() {
        let mut cursor = SelectionCursor::new(3, 3, false);
        cursor.step(-1);
        assert_eq!(cursor.index(), 0);
        cursor.step(10);
        assert_eq!(cursor.index(), 2);
    }

    #[test]
    fn an_empty_list_stays_at_index_zero() {
        let mut cursor = cursor(0);
        cursor.step(1);
        cursor.step(-1);
        cursor.page(1);
        cursor.jump_to(5);
        assert_eq!(cursor.index(), 0);
        assert_eq!(cursor.window(), 0..0);
    }

    #[test]
    fn the_window_follows_the_cursor_one_row_at_a_time() {
        // The list scrolls only once the cursor reaches the bottom edge, so
        // rows 0..3 stay visible until the cursor passes row 2.
        let mut cursor = cursor(10);
        cursor.step(1);
        cursor.step(1);
        assert_eq!(cursor.window(), 0..3, "still on the first page");
        cursor.step(1);
        assert_eq!(cursor.window(), 1..4, "scrolled by exactly one row");
    }

    #[test]
    fn a_page_jump_clamps_even_when_stepping_cycles() {
        let mut cursor = cursor(10);
        cursor.page(-1);
        assert_eq!(cursor.index(), 0, "no wrap to the bottom");
        cursor.page(10);
        assert_eq!(cursor.index(), 9, "no wrap to the top");
    }

    #[test]
    fn a_page_moves_by_the_visible_row_count() {
        let mut cursor = cursor(10);
        cursor.page(1);
        assert_eq!(cursor.index(), 3);
    }

    #[test]
    fn jump_to_clamps_past_the_end() {
        let mut cursor = cursor(4);
        cursor.jump_to(99);
        assert_eq!(cursor.index(), 3);
        assert_eq!(cursor.window(), 1..4);
    }

    #[test]
    fn shrinking_the_list_pulls_the_cursor_back_into_range() {
        let mut cursor = cursor(10);
        cursor.jump_to(9);
        cursor.set_len(4);
        assert_eq!(cursor.index(), 3);
        assert_eq!(cursor.window(), 1..4);
    }

    #[test]
    fn reset_returns_to_the_top_of_the_list() {
        let mut cursor = cursor(10);
        cursor.jump_to(8);
        cursor.reset();
        assert_eq!(cursor.index(), 0);
        assert_eq!(cursor.window(), 0..3);
    }

    #[test]
    fn a_visible_count_of_zero_is_treated_as_one_row() {
        let mut cursor = SelectionCursor::new(5, 0, true);
        cursor.step(1);
        assert_eq!(cursor.window(), 1..2);
    }
}
