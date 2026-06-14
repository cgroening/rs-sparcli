//! In-place redrawing for live displays, progress bars and spinners.
//!
//! `InPlace` is the shared engine that redraws a [`Rendered`] frame at the
//! cursor position; [`Live`] is the public widget wrapper. On non-terminals
//! (pipes, redirects) no control codes are emitted: only the final frame is
//! printed once, so logs stay clean.

use std::io::{self, Write};

use crossterm::cursor::{MoveToColumn, MoveUp};
use crossterm::queue;
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};

use crate::core::render::{Renderable, Rendered, write_line};
use crate::core::terminal::{color_support, is_output_tty, term_width};
use crate::error::Result;

/// Shared in-place frame writer used by live displays and progress widgets.
pub(crate) struct InPlace {
    interactive: bool,
    silent: bool,
    last_height: u16,
    last_frame: Rendered,
}

impl InPlace {
    /// Creates an in-place writer. `always` forces redraws off-terminal.
    pub(crate) fn new(always: bool) -> Self {
        Self {
            interactive: always || is_output_tty(),
            silent: false,
            last_height: 0,
            last_frame: Rendered::empty(),
        }
    }

    /// Creates a writer that never touches the terminal (headless prompts).
    pub(crate) fn silent() -> Self {
        Self {
            interactive: false,
            silent: true,
            last_height: 0,
            last_frame: Rendered::empty(),
        }
    }

    /// Draws a frame in place, replacing the previous one.
    pub(crate) fn draw(&mut self, frame: &Rendered) -> Result<()> {
        if self.silent {
            return Ok(());
        }
        if !self.interactive {
            self.last_frame = frame.clone();
            return Ok(());
        }
        let mut out = io::stdout().lock();
        self.rewind(&mut out)?;
        self.last_height = write_frame(&mut out, frame)?;
        out.flush()?;
        Ok(())
    }

    /// Moves the cursor back to the top-left of the previous frame and clears.
    fn rewind<W: Write>(&self, out: &mut W) -> io::Result<()> {
        if self.last_height == 0 {
            return Ok(());
        }
        queue!(out, MoveToColumn(0))?;
        if self.last_height > 1 {
            queue!(out, MoveUp(self.last_height - 1))?;
        }
        queue!(out, Clear(ClearType::FromCursorDown))?;
        Ok(())
    }

    /// Forgets the previous frame so the next draw starts fresh.
    ///
    /// Used after handing the terminal to an external program (e.g. an editor).
    pub(crate) fn reset(&mut self) {
        self.last_height = 0;
    }

    /// Finishes the session, leaving the final frame on screen.
    pub(crate) fn finish(self) -> Result<()> {
        if self.silent {
            return Ok(());
        }
        let mut out = io::stdout().lock();
        if self.interactive {
            queue!(out, Print("\r\n"))?;
        } else {
            write_frame(&mut out, &self.last_frame)?;
            queue!(out, Print("\n"))?;
        }
        out.flush()?;
        Ok(())
    }

    /// Finishes the session, erasing the last frame (transient display).
    pub(crate) fn clear(mut self) -> Result<()> {
        if !self.silent && self.interactive {
            let mut out = io::stdout().lock();
            self.rewind(&mut out)?;
            self.last_height = 0;
            out.flush()?;
        }
        Ok(())
    }
}

/// Writes a frame without a trailing newline; returns its line count.
fn write_frame<W: Write>(out: &mut W, frame: &Rendered) -> io::Result<u16> {
    let support = color_support();
    for (index, line) in frame.lines.iter().enumerate() {
        if index > 0 {
            queue!(out, Print("\r\n"))?;
        }
        write_line(out, line, support)?;
    }
    Ok(frame.lines.len() as u16)
}

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
            inplace: InPlace::new(false),
        }
    }

    /// Starts a live display that redraws even off-terminal.
    pub fn always() -> Self {
        Self {
            inplace: InPlace::new(true),
        }
    }

    /// Redraws with new content.
    ///
    /// # Errors
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn update(&mut self, content: &impl Renderable) -> Result<()> {
        self.inplace.draw(&content.render(term_width()))
    }

    /// Ends the session, leaving the final frame visible.
    ///
    /// # Errors
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn finish(self) -> Result<()> {
        self.inplace.finish()
    }

    /// Ends the session, erasing the display.
    ///
    /// # Errors
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

    #[test]
    fn silent_writer_never_draws() {
        let mut inplace = InPlace::silent();
        let frame = Rendered::new(vec![crate::core::text::Line::raw("x")]);
        assert!(inplace.draw(&frame).is_ok());
        assert!(inplace.finish().is_ok());
    }
}
