//! RAII terminal guard that restores the terminal on drop.
//!
//! Enables raw mode (and bracketed paste) for the lifetime of a prompt and
//! reliably restores cooked mode on drop, even on early return or panic, so a
//! cancelled or crashing prompt never leaves the terminal broken.

use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::error::Result;

/// Restores raw mode and bracketed paste when dropped.
pub struct TerminalGuard {
    bracketed_paste: bool,
}

impl TerminalGuard {
    /// Enables raw mode and bracketed paste.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if raw mode cannot be enabled.
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let bracketed_paste =
            execute!(std::io::stdout(), EnableBracketedPaste).is_ok();
        Ok(Self { bracketed_paste })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort restore: errors during cleanup are intentionally
        // ignored so dropping never panics.
        if self.bracketed_paste {
            let _ = execute!(std::io::stdout(), DisableBracketedPaste);
        }
        let _ = disable_raw_mode();
    }
}
