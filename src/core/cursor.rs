//! Hiding and restoring the terminal hardware cursor around in-place redraws.
//!
//! During spinner and progress animations, and during interactive prompts, the
//! library rewrites frames in place; a blinking hardware cursor at the end of a
//! frame is distracting, and prompts draw their own styled cursor anyway. These
//! helpers hide the cursor on the first request and restore it on the last,
//! both idempotently. Restoration is also guaranteed by the RAII drops of the
//! `TerminalGuard` and the in-place engine, so the cursor is shown again even
//! on an early return or panic.

use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::cursor::{Hide, Show};
use crossterm::execute;

/// Whether this module currently has the cursor hidden.
static HIDDEN: AtomicBool = AtomicBool::new(false);

/// Hides the terminal cursor once; a no-op if already hidden.
pub(crate) fn hide() {
    if HIDDEN.swap(true, Ordering::SeqCst) {
        return;
    }
    // Errors (e.g. a closed stream at shutdown) are ignored: this runs in
    // redraw loops and a failure to hide the cursor is never fatal.
    let _ = execute!(std::io::stdout(), Hide);
}

/// Restores the terminal cursor if this module hid it; a no-op otherwise.
pub(crate) fn show() {
    if !HIDDEN.swap(false, Ordering::SeqCst) {
        return;
    }
    let _ = execute!(std::io::stdout(), Show);
}
