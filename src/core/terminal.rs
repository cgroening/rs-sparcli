//! Terminal capability and size detection.
//!
//! Honors `NO_COLOR`, `CLICOLOR`/`CLICOLOR_FORCE` and the test override
//! `SPARCLI_NO_TTY`.

use std::env;
use std::io::{self, IsTerminal};

/// Fallback terminal width when the real size cannot be queried.
const DEFAULT_WIDTH: u16 = 80;
/// Fallback terminal height when the real size cannot be queried.
const DEFAULT_HEIGHT: u16 = 24;

/// How much color the output terminal supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSupport {
    /// No color should be emitted.
    None,
    /// The sixteen named ANSI colors.
    Ansi16,
    /// Full 24-bit truecolor.
    TrueColor,
}

/// Returns the terminal size as `(columns, rows)`, falling back to 80x24.
///
/// The `COLUMNS` and `LINES` environment variables override each dimension when
/// set to a positive integer; a dimension left unset is queried from the
/// terminal, then falls back to the default.
pub fn terminal_size() -> (u16, u16) {
    let env_cols = env_dimension("COLUMNS");
    let env_rows = env_dimension("LINES");
    if let (Some(cols), Some(rows)) = (env_cols, env_rows) {
        return (cols, rows);
    }
    let (cols, rows) =
        crossterm::terminal::size().unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT));
    (env_cols.unwrap_or(cols), env_rows.unwrap_or(rows))
}

/// Reads a positive terminal dimension from the environment, if present.
fn env_dimension(key: &str) -> Option<u16> {
    let value = env::var(key).ok()?;
    value.trim().parse::<u16>().ok().filter(|&n| n > 0)
}

/// Returns the terminal width in columns (fallback 80).
pub fn term_width() -> u16 {
    terminal_size().0
}

/// Returns the terminal height in rows (fallback 24).
pub fn term_height() -> u16 {
    terminal_size().1
}

/// Returns `true` if `SPARCLI_NO_TTY` forces non-terminal behavior.
fn no_tty_override() -> bool {
    matches!(env::var("SPARCLI_NO_TTY"), Ok(value)
        if !value.is_empty() && value != "0")
}

/// Returns `true` if standard output is an interactive terminal.
pub fn is_output_tty() -> bool {
    !no_tty_override() && io::stdout().is_terminal()
}

/// Returns `true` if both standard input and output are terminals.
pub fn is_input_tty() -> bool {
    !no_tty_override()
        && io::stdin().is_terminal()
        && io::stdout().is_terminal()
}

/// Returns `true` if the environment value is set and not disabled.
fn env_enabled(key: &str) -> bool {
    matches!(env::var(key), Ok(value) if !value.is_empty() && value != "0")
}

/// Returns `true` if `COLORTERM` advertises truecolor support.
fn colorterm_truecolor() -> bool {
    let Ok(value) = env::var("COLORTERM") else {
        return false;
    };
    let value = value.to_lowercase();
    value.contains("truecolor") || value.contains("24bit")
}

/// Detects the color support of the output terminal.
///
/// `CLICOLOR_FORCE` enables color even when piped; `NO_COLOR` always wins.
pub fn color_support() -> ColorSupport {
    if env::var_os("NO_COLOR").is_some() {
        return ColorSupport::None;
    }
    let forced = env_enabled("CLICOLOR_FORCE");
    if !forced && !is_output_tty() {
        return ColorSupport::None;
    }
    if colorterm_truecolor() {
        ColorSupport::TrueColor
    } else {
        ColorSupport::Ansi16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_size_has_sensible_fallback() {
        let (width, height) = terminal_size();
        assert!(width >= 1);
        assert!(height >= 1);
    }
}
