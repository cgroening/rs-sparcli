//! Interactive input widgets (prompts).
//!
//! Each prompt runs a small event loop over an
//! [`EventSource`](event::EventSource) and redraws in place. Prompts return an
//! [`Outcome`] that is either a submitted value or a cancellation, and never
//! panic on input.

pub mod confirm;
pub mod datepicker;
pub mod editor;
pub mod event;
pub mod guard;
pub mod history;
pub mod line_edit;
pub mod number;
pub mod password;
pub mod select;
pub mod shortcut;
pub mod text;
pub mod textarea;
pub mod validate;

#[cfg(feature = "fuzzy")]
pub mod fuzzy;

pub(crate) mod field;
pub(crate) mod prompt;

/// The result of running an interactive prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<T> {
    /// The user submitted a value.
    Submitted(T),
    /// The user cancelled (Esc or Ctrl-C).
    Cancelled,
    /// The user pressed a registered shortcut; carries its id.
    Shortcut(i32),
}

impl<T> Outcome<T> {
    /// Returns the submitted value, or `None` otherwise.
    pub fn submitted(self) -> Option<T> {
        match self {
            Outcome::Submitted(value) => Some(value),
            Outcome::Cancelled | Outcome::Shortcut(_) => None,
        }
    }

    /// Returns `true` if the prompt was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Outcome::Cancelled)
    }

    /// Returns the fired shortcut id, if the prompt ended on a shortcut.
    pub fn shortcut_id(&self) -> Option<i32> {
        match self {
            Outcome::Shortcut(id) => Some(*id),
            _ => None,
        }
    }
}
