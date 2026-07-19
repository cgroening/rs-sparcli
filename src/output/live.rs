//! Live display that redraws its content in place.
//!
//! The redraw engine itself lives in [`crate::core::inplace`], because the
//! input prompts need it too and `input` must not depend on `output`.

use crate::core::inplace::InPlace;
use crate::core::render::Renderable;
use crate::core::terminal::term_width;
use crate::error::Result;

/// A live display that redraws content in place.
pub struct Live {
    inplace: InPlace,
}

impl Default for Live {
    fn default() -> Self {
        Self::new()
    }
}

impl Live {
    /// Starts a live display (no-op redraws when output is not a terminal).
    pub fn new() -> Self {
        Self {
            inplace: InPlace::for_terminal(),
        }
    }

    /// Starts a live display that redraws even off-terminal.
    pub fn always() -> Self {
        Self {
            inplace: InPlace::forced(),
        }
    }

    /// Redraws with new content.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn update(&mut self, content: &impl Renderable) -> Result<()> {
        self.inplace.draw(&content.render(term_width()))
    }

    /// Ends the session, leaving the final frame visible.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn finish(self) -> Result<()> {
        self.inplace.finish()
    }

    /// Ends the session, erasing the display.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn clear(self) -> Result<()> {
        self.inplace.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_without_drawing() {
        // `update`/`finish` write directly to stdout (bypassing libtest
        // capture), so the test only exercises construction.
        let _live = Live::new();
        let _always = Live::always();
    }
}
