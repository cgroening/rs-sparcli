//! Pager integration: pipe long output through `$PAGER`, `less` or `more`.
//!
//! Available with the `pager` feature. When output is not a terminal (and the
//! pager is not forced), content is printed directly instead.

use std::env;
use std::process::{Command, Stdio};

use crate::core::command::split_command;
use crate::core::render::{Renderable, write_rendered};
use crate::core::terminal::{ColorSupport, is_output_tty, term_width};
use crate::error::{Result, SparcliError};

/// The default pager argument string on Unix-like systems.
#[cfg(not(windows))]
const DEFAULT_PAGER: &str = "less -R";
/// The default pager on Windows.
#[cfg(windows)]
const DEFAULT_PAGER: &str = "more";

/// Pages content through an external pager.
pub struct Pager {
    command: Option<String>,
    always: bool,
}

impl Default for Pager {
    fn default() -> Self {
        Self::new()
    }
}

impl Pager {
    /// Creates a pager using `$PAGER` or the platform default.
    pub fn new() -> Self {
        Self {
            command: None,
            always: false,
        }
    }

    /// Overrides the pager command (whitespace-split, no shell).
    #[must_use]
    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// Pages even when output is not a terminal.
    #[must_use]
    pub fn always(mut self) -> Self {
        self.always = true;
        self
    }

    /// Pages `content`, falling back to a direct print off-terminal.
    ///
    /// # Errors
    ///
    /// Returns [`SparcliError::Io`] if spawning the pager or writing fails,
    /// or [`SparcliError::Config`] if the pager command is empty or has an
    /// unbalanced quote.
    pub fn page(&self, content: &impl Renderable) -> Result<()> {
        if !self.always && !is_output_tty() {
            return content.print();
        }
        let rendered = content.render(term_width());
        let resolved = self.resolve_command();
        let argv = split_command(&resolved)
            .ok_or_else(|| SparcliError::Config("unparsable pager".into()))?;
        let (program, args) = argv
            .split_first()
            .ok_or_else(|| SparcliError::Config("empty pager".into()))?;
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .spawn()?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| SparcliError::Config("pager stdin".into()))?;
        write_rendered(&mut stdin, &rendered, ColorSupport::TrueColor)?;
        drop(stdin);
        child.wait()?;
        Ok(())
    }

    /// Resolves the pager command string.
    fn resolve_command(&self) -> String {
        if let Some(command) = &self.command {
            return command.clone();
        }
        env::var("PAGER")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_PAGER.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_explicit_command() {
        let pager = Pager::new().command("bat --paging always");
        assert_eq!(pager.resolve_command(), "bat --paging always");
    }
}
