//! Hiding and restoring the terminal hardware cursor around in-place redraws.
//!
//! During spinner and progress animations, and during interactive prompts, the
//! library rewrites frames in place; a blinking hardware cursor at the end of a
//! frame is distracting, and prompts draw their own styled cursor anyway. These
//! helpers hide the cursor on the first request and restore it on the last,
//! both idempotently. Restoration is also guaranteed by the RAII drops of the
//! `TerminalGuard` and the in-place engine, so the cursor is shown again even
//! on an early return or panic.
//!
//! The escape sequence goes to the stream that is being animated. Writing it
//! to standard output unconditionally would inject control codes into piped
//! payload whenever a progress bar animates on standard error.

use std::sync::atomic::{AtomicU8, Ordering};

use crossterm::cursor::{Hide, Show};
use crossterm::execute;

use crate::core::terminal::Stream;

/// Sentinel for "the cursor is not hidden by this module".
const VISIBLE: u8 = 0;
/// Marks the cursor as hidden on standard output.
const HIDDEN_STDOUT: u8 = 1;
/// Marks the cursor as hidden on standard error.
const HIDDEN_STDERR: u8 = 2;

/// Which stream this module hid the cursor on, if any.
static HIDDEN_ON: AtomicU8 = AtomicU8::new(VISIBLE);

/// Hides the terminal cursor on `stream` once; a no-op if already hidden.
pub(crate) fn hide(stream: Stream) {
    let marker = marker_for(stream);
    if HIDDEN_ON.swap(marker, Ordering::SeqCst) != VISIBLE {
        return;
    }
    // Errors (e.g. a closed stream at shutdown) are ignored: this runs in
    // redraw loops and a failure to hide the cursor is never fatal.
    let _ = write_cursor(stream, true);
}

/// Restores the terminal cursor if this module hid it; a no-op otherwise.
pub(crate) fn show() {
    let marker = HIDDEN_ON.swap(VISIBLE, Ordering::SeqCst);
    if marker == VISIBLE {
        return;
    }
    let stream = if marker == HIDDEN_STDERR {
        Stream::Stderr
    } else {
        Stream::Stdout
    };
    let _ = write_cursor(stream, false);
}

/// Returns the marker value standing for `stream`.
fn marker_for(stream: Stream) -> u8 {
    match stream {
        Stream::Stdout => HIDDEN_STDOUT,
        Stream::Stderr => HIDDEN_STDERR,
    }
}

/// Writes the hide or show sequence to `stream`.
fn write_cursor(stream: Stream, hide: bool) -> std::io::Result<()> {
    match (stream, hide) {
        (Stream::Stdout, true) => execute!(std::io::stdout(), Hide),
        (Stream::Stdout, false) => execute!(std::io::stdout(), Show),
        (Stream::Stderr, true) => execute!(std::io::stderr(), Hide),
        (Stream::Stderr, false) => execute!(std::io::stderr(), Show),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_round_trips_through_the_stream() {
        assert_eq!(marker_for(Stream::Stdout), HIDDEN_STDOUT);
        assert_eq!(marker_for(Stream::Stderr), HIDDEN_STDERR);
        assert_ne!(HIDDEN_STDOUT, VISIBLE);
        assert_ne!(HIDDEN_STDERR, VISIBLE);
    }
}
