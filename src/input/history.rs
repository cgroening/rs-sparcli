//! Input history with optional XDG-style persistence.

use std::env;
use std::fs;
use std::path::PathBuf;

use crate::error::Result;

/// Default maximum number of retained entries.
const DEFAULT_MAX: usize = 500;

/// A bounded list of past input lines, optionally backed by a file.
pub struct History {
    entries: Vec<String>,
    max: usize,
    path: Option<PathBuf>,
    keep_duplicates: bool,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    /// Creates an in-memory history.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max: DEFAULT_MAX,
            path: None,
            keep_duplicates: false,
        }
    }

    /// Creates a history persisted under the app's state directory.
    ///
    /// Uses `XDG_STATE_HOME` (or `~/.local/state`, or `%LOCALAPPDATA%`).
    pub fn for_app(app: &str) -> Self {
        Self {
            path: state_dir().map(|dir| dir.join(app).join("history")),
            ..Self::new()
        }
    }

    /// Sets the maximum number of entries.
    #[must_use]
    pub fn max_entries(mut self, max: usize) -> Self {
        self.max = max.max(1);
        self
    }

    /// Keeps consecutive duplicate entries instead of collapsing them.
    #[must_use]
    pub fn keep_duplicates(mut self) -> Self {
        self.keep_duplicates = true;
        self
    }

    /// Returns the entries, oldest first.
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Adds a line, skipping blanks and (by default) consecutive duplicates.
    pub fn add(&mut self, line: &str) {
        if line.trim().is_empty() {
            return;
        }
        if !self.keep_duplicates
            && self.entries.last().map(String::as_str) == Some(line)
        {
            return;
        }
        self.entries.push(line.to_string());
        if self.entries.len() > self.max {
            let overflow = self.entries.len() - self.max;
            self.entries.drain(0..overflow);
        }
    }

    /// Loads entries from the backing file, if configured.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if the file exists but cannot be
    /// read.
    pub fn load(&mut self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if !path.exists() {
            return Ok(());
        }
        let contents = fs::read_to_string(path)?;
        self.entries = contents.lines().map(str::to_string).collect();
        Ok(())
    }

    /// Saves entries to the backing file, creating directories as needed.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn save(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Write to a sibling temp file and rename it over the target so a
        // crash or a concurrent writer never leaves a half-written file.
        let temp = path.with_extension(format!("tmp.{}", std::process::id()));
        fs::write(&temp, self.entries.join("\n"))?;
        if let Err(error) = fs::rename(&temp, path) {
            let _ = fs::remove_file(&temp);
            return Err(error.into());
        }
        Ok(())
    }
}

/// Resolves the platform state directory for persisted history.
fn state_dir() -> Option<PathBuf> {
    if let Some(dir) = env::var_os("XDG_STATE_HOME") {
        return Some(PathBuf::from(dir));
    }
    #[cfg(windows)]
    {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_skips_blank_and_consecutive_duplicates() {
        let mut history = History::new();
        history.add("a");
        history.add("a");
        history.add("   ");
        history.add("b");
        assert_eq!(history.entries(), &["a", "b"]);
    }

    #[test]
    fn add_respects_the_max_limit() {
        let mut history = History::new().max_entries(2);
        history.add("a");
        history.add("b");
        history.add("c");
        assert_eq!(history.entries(), &["b", "c"]);
    }

    #[test]
    fn save_is_atomic_and_leaves_no_temp_files() {
        let dir = std::env::temp_dir()
            .join(format!("sparcli_hist_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut history = History::new();
        history.path = Some(dir.join("history"));
        history.add("one");
        history.save().unwrap();

        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, vec!["history".to_string()]);
        assert_eq!(fs::read_to_string(dir.join("history")).unwrap(), "one");
        fs::remove_dir_all(&dir).unwrap();
    }
}
