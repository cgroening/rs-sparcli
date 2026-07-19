//! External editor integration (`$VISUAL` / `$EDITOR`).
//!
//! Used by text prompts (Ctrl-G) and available standalone via [`edit_file`].

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::command::{program_and_args, resolve_from_env};
use crate::core::private_file;
use crate::error::Result;

/// Opens an external editor on `path`, blocking until it exits.
///
/// `command` overrides the editor; otherwise `$VISUAL`, then `$EDITOR`, then a
/// platform default is used. The command is split with
/// [`split_command`](crate::core::command::split_command), so a quoted path
/// containing spaces survives; it is never passed to a shell.
///
/// # Errors
///
/// Returns [`SparcliError::Io`] if the editor cannot be spawned, or
/// [`SparcliError::Config`] if the resolved command is empty.
pub fn edit_file(command: Option<&str>, path: &Path) -> Result<()> {
    let resolved = resolve_command(command);
    let (program, args) = program_and_args(&resolved, "editor")?;
    Command::new(program).args(args).arg(path).status()?;
    Ok(())
}

/// Edits `initial` in an external editor via a temp file, returning the result.
///
/// `suffix` sets the temp file extension (e.g. `.md`) for editor syntax
/// detection. Returns the edited contents.
///
/// # Errors
///
/// Returns [`SparcliError::Io`] on temp-file or spawn failure.
pub(crate) fn edit_text(
    command: Option<&str>,
    initial: &str,
    suffix: &str,
) -> Result<String> {
    let path = temp_path(suffix);
    // Created exclusively and owner-only: the temp directory is shared, and
    // this file carries whatever was typed into the prompt.
    let mut file = private_file::create_new(&path)?;
    file.write_all(initial.as_bytes())?;
    drop(file);
    let result = edit_file(command, &path);
    let contents = fs::read_to_string(&path);
    if let Err(error) = fs::remove_file(&path) {
        log::debug!("could not remove temp file {}: {error}", path.display());
    }
    result?;
    Ok(contents?)
}

/// Edits `initial` in an external editor while raw mode is suspended.
///
/// A prompt owns the terminal in raw mode, which an editor cannot share, so
/// raw mode is left before the handover and re-entered afterwards. It is only
/// toggled when it was already enabled, so a headless caller never alters the
/// terminal. A failure to toggle is logged rather than propagated: the edit
/// itself is what the caller asked for, and its result is still usable.
///
/// # Errors
///
/// Returns [`SparcliError::Io`] on temp-file or spawn failure.
pub(crate) fn edit_text_suspended(
    command: Option<&str>,
    initial: &str,
    suffix: &str,
) -> Result<String> {
    let was_raw = crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
    if was_raw && let Err(error) = crossterm::terminal::disable_raw_mode() {
        log::debug!("could not leave raw mode for the editor: {error}");
    }
    let result = edit_text(command, initial, suffix);
    if was_raw && let Err(error) = crossterm::terminal::enable_raw_mode() {
        log::debug!("could not re-enter raw mode after the editor: {error}");
    }
    result
}

/// Resolves the editor command from the override or environment.
fn resolve_command(command: Option<&str>) -> String {
    resolve_from_env(command, &["VISUAL", "EDITOR"], default_editor())
}

/// The platform's fallback editor.
fn default_editor() -> &'static str {
    if cfg!(windows) { "notepad" } else { "vi" }
}

/// Builds a unique temp-file path with the given suffix.
fn temp_path(suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = format!("sparcli-{}-{nanos}{suffix}", std::process::id());
    env::temp_dir().join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_prefers_explicit_command() {
        assert_eq!(resolve_command(Some("nano")), "nano");
        // A blank override falls through to the environment or default; the
        // exact value depends on $VISUAL/$EDITOR, so just require non-empty.
        assert!(!resolve_command(Some("  ")).trim().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn edit_text_round_trips_through_a_noop_editor() {
        // `true` ignores its argument, leaving the temp file unchanged.
        let result = edit_text(Some("true"), "hello", ".txt").unwrap();
        assert_eq!(result, "hello");
    }
}
