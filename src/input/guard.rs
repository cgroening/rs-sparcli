//! RAII terminal guard that restores the terminal on drop.
//!
//! Enables raw mode (and bracketed paste) for the lifetime of a prompt and
//! reliably restores cooked mode on drop, even on early return or panic, so a
//! cancelled or crashing prompt never leaves the terminal broken.

use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::core::cursor;
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
        cursor::hide();
        Ok(Self { bracketed_paste })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort restore that never panics: cleanup errors are logged
        // and otherwise ignored so dropping stays infallible.
        cursor::show();
        if self.bracketed_paste
            && let Err(error) =
                execute!(std::io::stdout(), DisableBracketedPaste)
        {
            log::warn!("could not disable bracketed paste: {error}");
        }
        if let Err(error) = disable_raw_mode() {
            log::warn!("could not restore terminal from raw mode: {error}");
        }
    }
}
