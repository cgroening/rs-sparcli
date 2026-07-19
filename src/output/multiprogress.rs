//! Multiple progress bars updated together in place.

use crate::core::inplace::InPlace;
use crate::core::render::Rendered;
use crate::error::Result;
use crate::output::progress::ProgressBar;

/// The live state of one bar within a [`MultiProgress`].
struct BarState {
    bar: ProgressBar,
    value: f64,
    max: f64,
}

/// A group of progress bars rendered as one block and updated in place.
pub struct MultiProgress {
    bars: Vec<BarState>,
    inplace: InPlace,
    transient: bool,
}

impl Default for MultiProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiProgress {
    /// Starts a multi-progress session.
    pub fn new() -> Self {
        Self {
            bars: Vec::new(),
            inplace: InPlace::progress(),
            transient: false,
        }
    }

    /// Erases all bars when the session ends instead of leaving them.
    #[must_use]
    pub fn transient(mut self) -> Self {
        self.transient = true;
        self
    }

    /// Adds a bar and returns its index.
    pub fn add(&mut self, bar: ProgressBar) -> usize {
        self.bars.push(BarState {
            bar,
            value: 0.0,
            max: 1.0,
        });
        self.bars.len() - 1
    }

    /// Updates the bar at `index` and redraws the whole group.
    ///
    /// Out-of-range indices are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn update(&mut self, index: usize, value: f64, max: f64) -> Result<()> {
        if let Some(state) = self.bars.get_mut(index) {
            state.value = value;
            state.max = max;
        }
        let frame = self.frame();
        self.inplace.draw(&frame)
    }

    /// Ends the session, leaving or erasing the bars.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn finish(self) -> Result<()> {
        if self.transient {
            self.inplace.clear()
        } else {
            self.inplace.finish()
        }
    }

    /// Builds the combined frame of all bars.
    fn frame(&self) -> Rendered {
        let mut lines = Vec::new();
        for state in &self.bars {
            let bar = state.bar.bar(state.value, state.max);
            lines.extend(bar.lines);
        }
        Rendered::new(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_has_one_line_per_bar() {
        let mut multi = MultiProgress::new();
        multi.add(ProgressBar::new().label("a"));
        multi.add(ProgressBar::new().label("b"));
        assert_eq!(multi.frame().height(), 2);
    }

    #[test]
    fn add_returns_sequential_indices() {
        // Avoids `update`, which would draw to the real terminal under a TTY.
        let mut multi = MultiProgress::new();
        assert_eq!(multi.add(ProgressBar::new()), 0);
        assert_eq!(multi.add(ProgressBar::new()), 1);
    }
}
