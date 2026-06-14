//! External editor integration (`$VISUAL` / `$EDITOR`).
//!
//! Used by text prompts (Ctrl-G) and available standalone via [`edit_file`].

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Result, SparcliError};

/// Opens an external editor on `path`, blocking until it exits.
///
/// `command` overrides the editor; otherwise `$VISUAL`, then `$EDITOR`, then a
/// platform default is used. The command is whitespace-split (no shell).
///
/// # Errors
/// Returns [`SparcliError::Io`] if the editor cannot be spawned, or
/// [`SparcliError::Config`] if the resolved command is empty.
pub fn edit_file(command: Option<&str>, path: &Path) -> Result<()> {
    let resolved = resolve_command(command);
    let mut parts = resolved.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| SparcliError::Config("empty editor command".into()))?;
    Command::new(program).args(parts).arg(path).status()?;
    Ok(())
}

/// Edits `initial` in an external editor via a temp file, returning the result.
///
/// `suffix` sets the temp file extension (e.g. `.md`) for editor syntax
/// detection. Returns the edited contents.
///
/// # Errors
/// Returns [`SparcliError::Io`] on temp-file or spawn failure.
pub(crate) fn edit_text(
    command: Option<&str>,
    initial: &str,
    suffix: &str,
) -> Result<String> {
    let path = temp_path(suffix);
    fs::write(&path, initial)?;
    let result = edit_file(command, &path);
    let contents = fs::read_to_string(&path);
    let _ = fs::remove_file(&path);
    result?;
    Ok(contents?)
}

/// Resolves the editor command from the override or environment.
fn resolve_command(command: Option<&str>) -> String {
    if let Some(command) = command
        && !command.trim().is_empty()
    {
        return command.to_string();
    }
    for key in ["VISUAL", "EDITOR"] {
        if let Ok(value) = env::var(key)
            && !value.trim().is_empty()
        {
            return value;
        }
    }
    default_editor().to_string()
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
