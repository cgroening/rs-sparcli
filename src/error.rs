//! Error and result types shared across the crate.

use std::io;

use thiserror::Error;

/// Errors that can occur while rendering output or running input prompts.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SparcliError {
    /// An I/O error occurred while writing to the terminal or a stream.
    #[error("terminal I/O failed: {0}")]
    Io(#[from] io::Error),

    /// An interactive prompt was requested without a usable terminal.
    #[error("no interactive terminal available")]
    NoTerminal,

    /// A widget or prompt was configured with an invalid value.
    #[error("invalid configuration: {0}")]
    Config(String),
}

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, SparcliError>;
