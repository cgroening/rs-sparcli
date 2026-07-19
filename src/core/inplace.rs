//! In-place frame redrawing at the cursor position.
//!
//! [`InPlace`] is the shared engine behind live displays, progress widgets and
//! interactive prompts: it redraws a [`Rendered`] frame over the previous one
//! instead of appending to the scrollback. On non-terminals (pipes, redirects)
//! no control codes are emitted and only the final frame is printed once, so
//! logs stay clean.
//!
//! It lives in `core` rather than next to [`Live`](crate::output::live::Live)
//! because both `output` and `input` need it, and `input` must not depend on
//! `output`.

use std::io::{self, Write};

use crossterm::cursor::{MoveToColumn, MoveUp};
use crossterm::queue;
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};

use crate::core::cursor;
use crate::core::render::{Rendered, write_line};
use crate::core::terminal::{Stream, color_support};
use crate::error::Result;

/// Shared in-place frame writer used by live displays and progress widgets.
pub(crate) struct InPlace {
    stream: Stream,
    interactive: bool,
    silent: bool,
    last_height: u16,
    last_frame: Rendered,
}

impl InPlace {
    /// Creates a writer that redraws standard output when it is a terminal.
    pub(crate) fn for_terminal() -> Self {
        Self::on(Stream::Stdout, Stream::Stdout.is_tty())
    }

    /// Creates a writer for a progress indicator.
    ///
    /// Progress belongs on standard error: it is not payload, and a caller
    /// piping standard output onward must not receive animation frames in the
    /// data. It is drawn only when standard error is itself a terminal.
    pub(crate) fn progress() -> Self {
        Self::on(Stream::Stderr, Stream::Stderr.is_tty())
    }

    /// Creates a writer that redraws even when output is not a terminal.
    pub(crate) fn forced() -> Self {
        Self::on(Stream::Stdout, true)
    }

    /// Creates a writer that never touches the terminal (headless prompts).
    pub(crate) fn silent() -> Self {
        Self {
            stream: Stream::Stdout,
            interactive: false,
            silent: true,
            last_height: 0,
            last_frame: Rendered::empty(),
        }
    }

    /// Builds a non-silent writer for `stream` with the given redraw mode.
    fn on(stream: Stream, interactive: bool) -> Self {
        Self {
            stream,
            interactive,
            silent: false,
            last_height: 0,
            last_frame: Rendered::empty(),
        }
    }

    /// Draws a frame in place, replacing the previous one.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing to the terminal fails.
    pub(crate) fn draw(&mut self, frame: &Rendered) -> Result<()> {
        if self.silent {
            return Ok(());
        }
        if !self.interactive {
            self.last_frame = frame.clone();
            return Ok(());
        }
        cursor::hide(self.stream);
        let mut out = StreamWriter::lock(self.stream);
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
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing to the terminal fails.
    pub(crate) fn finish(self) -> Result<()> {
        if self.silent {
            return Ok(());
        }
        {
            let mut out = StreamWriter::lock(self.stream);
            if self.interactive {
                queue!(out, Print("\r\n"))?;
            } else {
                write_frame(&mut out, &self.last_frame)?;
                queue!(out, Print("\n"))?;
            }
            out.flush()?;
        }
        cursor::show();
        Ok(())
    }

    /// Finishes the session, erasing the last frame (transient display).
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing to the terminal fails.
    pub(crate) fn clear(mut self) -> Result<()> {
        if !self.silent && self.interactive {
            {
                let mut out = StreamWriter::lock(self.stream);
                self.rewind(&mut out)?;
                out.flush()?;
            }
            self.last_height = 0;
            cursor::show();
        }
        Ok(())
    }
}

impl Drop for InPlace {
    fn drop(&mut self) {
        // Safety net mirroring Python's atexit restore: if a live display or
        // spinner is dropped mid-animation without finishing, still show the
        // cursor. Idempotent, so a prior finish/clear makes this a no-op.
        cursor::show();
    }
}

/// A locked handle to whichever standard stream is being drawn to.
///
/// The two locked handles are distinct types, so they are united here rather
/// than behind `dyn Write` - the `queue!` macro needs a sized writer.
enum StreamWriter {
    /// A locked standard output handle.
    Stdout(io::StdoutLock<'static>),
    /// A locked standard error handle.
    Stderr(io::StderrLock<'static>),
}

impl StreamWriter {
    /// Locks `stream` for writing.
    fn lock(stream: Stream) -> Self {
        match stream {
            Stream::Stdout => StreamWriter::Stdout(io::stdout().lock()),
            Stream::Stderr => StreamWriter::Stderr(io::stderr().lock()),
        }
    }
}

impl Write for StreamWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            StreamWriter::Stdout(out) => out.write(buf),
            StreamWriter::Stderr(out) => out.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            StreamWriter::Stdout(out) => out.flush(),
            StreamWriter::Stderr(out) => out.flush(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::text::Line;

    #[test]
    fn silent_writer_never_draws() {
        let mut inplace = InPlace::silent();
        let frame = Rendered::new(vec![Line::raw("x")]);
        assert!(inplace.draw(&frame).is_ok());
        assert!(inplace.finish().is_ok());
    }

    #[test]
    fn detached_writer_buffers_the_last_frame_instead_of_drawing() {
        let mut inplace = InPlace::on(Stream::Stdout, false);
        let frame = Rendered::new(vec![Line::raw("buffered")]);
        assert!(inplace.draw(&frame).is_ok());
        assert_eq!(inplace.last_frame.plain(), "buffered");
        assert_eq!(inplace.last_height, 0);
    }

    #[test]
    fn progress_draws_on_standard_error_not_the_payload_stream() {
        // A progress bar is not payload. Drawing it on standard output would
        // put animation frames into whatever a caller pipes the output into.
        assert_eq!(InPlace::progress().stream, Stream::Stderr);
        assert_eq!(InPlace::for_terminal().stream, Stream::Stdout);
        assert_eq!(InPlace::forced().stream, Stream::Stdout);
    }

    #[test]
    fn reset_forgets_the_previous_frame_height() {
        let mut inplace = InPlace::forced();
        inplace.last_height = 5;
        inplace.reset();
        assert_eq!(inplace.last_height, 0);
    }

    #[test]
    fn write_frame_joins_lines_and_counts_them() {
        let frame = Rendered::new(vec![Line::raw("one"), Line::raw("two")]);
        let mut out = Vec::new();
        let height = write_frame(&mut out, &frame).unwrap();
        assert_eq!(height, 2);
        assert!(String::from_utf8_lossy(&out).contains("one"));
        assert!(String::from_utf8_lossy(&out).contains("two"));
    }
}
