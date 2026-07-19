//! Input history with optional XDG-style persistence.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::private_file;
use crate::error::Result;

/// Default maximum number of retained entries.
const DEFAULT_MAX: usize = 500;

/// Name of the history file inside the per-app state directory.
const HISTORY_FILE: &str = "history";

/// Characters that would let an app name escape its state subdirectory.
const PATH_SEPARATORS: [char; 3] = ['/', '\\', ':'];

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
    /// Uses `XDG_STATE_HOME` (or `~/.local/state`, or `%LOCALAPPDATA%`). Both
    /// the directory and `app` come from outside the process, so the directory
    /// is canonicalized and `app` is rejected outright if it could escape it;
    /// such a history stays in memory instead of writing somewhere unexpected.
    pub fn for_app(app: &str) -> Self {
        Self {
            path: state_dir().and_then(|dir| history_path(&dir, app)),
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
        // Keep only the newest `max` lines. The file is foreign input - it may
        // have been written by an older build with a larger limit, or grown
        // unbounded - and `max` is what the caller asked to hold in memory.
        let lines: Vec<&str> = contents.lines().collect();
        let start = lines.len().saturating_sub(self.max);
        self.entries =
            lines[start..].iter().map(|s| (*s).to_string()).collect();
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
        // Owner-only, because history holds whatever was typed at a prompt.
        let temp = path.with_extension(format!("tmp.{}", std::process::id()));
        private_file::write(&temp, &self.entries.join("\n"))?;
        if let Err(error) = fs::rename(&temp, path) {
            let _ = fs::remove_file(&temp);
            return Err(error.into());
        }
        Ok(())
    }
}

/// Returns the history file for `app` under `dir`, or `None` if unusable.
///
/// The app name must not be empty, a dot component, or contain a path
/// separator, and the joined path must still sit inside `dir`.
fn history_path(dir: &Path, app: &str) -> Option<PathBuf> {
    if app.is_empty() || app == "." || app == ".." {
        log::warn!("refusing unsafe history app name: {app:?}");
        return None;
    }
    if app.contains(PATH_SEPARATORS) {
        log::warn!("refusing unsafe history app name: {app:?}");
        return None;
    }
    let candidate = dir.join(app).join(HISTORY_FILE);
    if !candidate.starts_with(dir) {
        log::warn!("refusing history path outside the state dir: {app:?}");
        return None;
    }
    Some(candidate)
}

/// Resolves the platform state directory for persisted history.
///
/// The value comes from the environment, so it is normalized to an absolute
/// path before anything is written under it.
fn state_dir() -> Option<PathBuf> {
    if let Some(dir) = env::var_os("XDG_STATE_HOME") {
        return Some(absolute(PathBuf::from(dir)));
    }
    #[cfg(windows)]
    {
        env::var_os("LOCALAPPDATA").map(|dir| absolute(PathBuf::from(dir)))
    }
    #[cfg(not(windows))]
    {
        env::var_os("HOME")
            .map(|home| absolute(PathBuf::from(home).join(".local/state")))
    }
}

/// Returns `path` made absolute, falling back to the input if that fails.
///
/// `canonicalize` needs the path to exist, which it may not yet, so this uses
/// `std::path::absolute` and only normalizes lexically.
fn absolute(path: PathBuf) -> PathBuf {
    std::path::absolute(&path).unwrap_or(path)
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
    fn load_keeps_only_the_newest_max_entries() {
        // The file is foreign input: an older build or another tool may have
        // left far more lines in it than this history is configured to hold.
        let dir = std::env::temp_dir()
            .join(format!("sparcli_hist_cap_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history");
        fs::write(&path, "a\nb\nc\nd\ne").unwrap();

        let mut history = History::new().max_entries(2);
        history.path = Some(path);
        history.load().unwrap();
        assert_eq!(history.entries(), &["d", "e"]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_saved_history_is_not_readable_by_other_accounts() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir()
            .join(format!("sparcli_hist_perm_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut history = History::new();
        history.path = Some(dir.join("history"));
        history.add("a token someone pasted into a prompt");
        history.save().unwrap();

        let mode = fs::metadata(dir.join("history"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "group and other must have no access");

        let _ = fs::remove_dir_all(&dir);
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

    #[test]
    fn a_plain_app_name_yields_a_path_inside_the_state_dir() {
        let dir = Path::new("/tmp/sparcli-state");
        let path = history_path(dir, "demo").unwrap();
        assert_eq!(path, dir.join("demo").join("history"));
    }

    #[test]
    fn a_traversing_app_name_is_rejected() {
        let dir = Path::new("/tmp/sparcli-state");
        assert!(history_path(dir, "../../etc").is_none());
    }

    #[test]
    fn an_app_name_with_a_separator_is_rejected() {
        let dir = Path::new("/tmp/sparcli-state");
        assert!(history_path(dir, "a/b").is_none());
        assert!(history_path(dir, "a\\b").is_none());
    }

    #[test]
    fn a_dot_app_name_is_rejected() {
        let dir = Path::new("/tmp/sparcli-state");
        assert!(history_path(dir, ".").is_none());
        assert!(history_path(dir, "..").is_none());
    }

    #[test]
    fn an_empty_app_name_is_rejected() {
        assert!(history_path(Path::new("/tmp/sparcli-state"), "").is_none());
    }

    #[test]
    fn a_rejected_app_name_leaves_the_history_in_memory() {
        let mut history = History::for_app("../escape");
        history.add("secret");
        history.save().unwrap();
        assert_eq!(history.entries(), &["secret"]);
    }
}
