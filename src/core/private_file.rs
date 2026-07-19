//! Creating files that only their owner can read.
//!
//! Two places write user input to disk: the input history and the temp file
//! handed to `$EDITOR`. Both can carry whatever was typed into a prompt -
//! including a token pasted into a text field - so neither may land at the
//! default umask, where any local account can read it.
//!
//! The temp file additionally has to be created exclusively. Its directory is
//! world-writable, so a predictable name that is merely opened for writing can
//! be pre-created as a symlink by another user and redirect the write.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Permission bits for a file only its owner may read or write.
#[cfg(unix)]
const OWNER_ONLY: u32 = 0o600;

/// Creates `path` for writing, failing if it already exists.
///
/// The file is owner-readable only. Failing on an existing path is the point:
/// it is what makes a pre-created symlink an error rather than a redirect.
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] if the file exists or cannot be
/// created.
pub(crate) fn create_new(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(OWNER_ONLY);
    }
    options.open(path)
}

/// Writes `contents` to `path`, replacing it, with owner-only permissions.
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] if the file cannot be written.
pub(crate) fn write(path: &Path, contents: &str) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(OWNER_ONLY);
    }
    let mut file = options.open(path)?;
    file.write_all(contents.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a unique scratch path inside the temp directory.
    fn scratch(name: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("sparcli-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn write_replaces_the_previous_contents() {
        let path = scratch("write");
        write(&path, "first").expect("a fresh file is writable");
        write(&path, "second").expect("an existing file is replaced");
        let read = std::fs::read_to_string(&path).expect("it reads back");
        assert_eq!(read, "second");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn create_new_refuses_an_existing_path() {
        // This is the symlink guard: an attacker-planted path must fail the
        // create rather than silently become the write target.
        let path = scratch("exclusive");
        let _ = std::fs::remove_file(&path);
        create_new(&path).expect("the first create succeeds");
        assert!(create_new(&path).is_err(), "the second must not clobber");
        let _ = std::fs::remove_file(&path);
    }

    #[cfg(unix)]
    #[test]
    fn files_are_readable_only_by_their_owner() {
        use std::os::unix::fs::PermissionsExt;

        let written = scratch("perm-write");
        write(&written, "secret").expect("it is writable");
        let mode = std::fs::metadata(&written)
            .expect("it exists")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, OWNER_ONLY, "group and other must be denied");
        let _ = std::fs::remove_file(&written);

        let created = scratch("perm-create");
        let _ = std::fs::remove_file(&created);
        create_new(&created).expect("it is creatable");
        let mode = std::fs::metadata(&created)
            .expect("it exists")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, OWNER_ONLY);
        let _ = std::fs::remove_file(&created);
    }
}
