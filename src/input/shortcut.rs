//! Custom keyboard shortcuts and footer hint rendering.
//!
//! A small, reusable registry: match a key to an action id and render a footer
//! hint line in the shared style. Prompts that accept shortcuts consult
//! [`find`] and show [`hint_line`].

use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::theme::theme;
use crate::input::event::{KeyCode, KeyPress};

/// A bound shortcut: a key, a caller-defined id and a footer label.
#[derive(Debug, Clone)]
pub struct Shortcut {
    /// The key that triggers the action.
    pub key: KeyPress,
    /// The caller-defined action id reported when fired.
    pub id: i32,
    /// The label shown in the footer hint line.
    pub label: String,
}

impl Shortcut {
    /// Creates a shortcut bound to a key.
    pub fn new(key: KeyPress, id: i32, label: impl Into<String>) -> Self {
        Self {
            key,
            id,
            label: label.into(),
        }
    }
}

/// Returns the id of the shortcut matching `key`, if any.
pub fn find(key: KeyPress, shortcuts: &[Shortcut]) -> Option<i32> {
    shortcuts.iter().find(|s| s.key == key).map(|s| s.id)
}

/// Builds a footer hint line: `key label · key label · …`.
pub fn hint_line(shortcuts: &[Shortcut]) -> Line {
    let theme = theme();
    let mut spans = Vec::new();
    for (index, shortcut) in shortcuts.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" · ".to_string(), theme.secondary));
        }
        spans.push(Span::styled(
            key_name(shortcut.key),
            Style::new().fg(theme.accent),
        ));
        spans.push(Span::styled(
            format!(" {}", shortcut.label),
            theme.secondary,
        ));
    }
    Line::new(spans)
}

/// Returns a human-readable name for a key press.
pub fn key_name(key: KeyPress) -> String {
    let mut name = String::new();
    if key.ctrl {
        name.push_str("Ctrl-");
    }
    if key.alt {
        name.push_str("Alt-");
    }
    name.push_str(&code_name(key.code));
    name
}

/// Returns the base name for a key code.
fn code_name(code: KeyCode) -> String {
    match code {
        KeyCode::Char(c) => c.to_uppercase().to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "Shift-Tab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Del".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::Function(n) => format!("F{n}"),
        KeyCode::Unknown => "?".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_matches_bound_key() {
        let shortcuts = vec![Shortcut::new(KeyPress::ctrl('s'), 1, "save")];
        assert_eq!(find(KeyPress::ctrl('s'), &shortcuts), Some(1));
        assert_eq!(find(KeyPress::ctrl('x'), &shortcuts), None);
    }

    #[test]
    fn key_name_includes_modifiers() {
        assert_eq!(key_name(KeyPress::ctrl('s')), "Ctrl-S");
        assert_eq!(key_name(KeyPress::new(KeyCode::Function(2))), "F2");
    }

    #[test]
    fn hint_line_lists_shortcuts() {
        let shortcuts = vec![
            Shortcut::new(KeyPress::ctrl('s'), 1, "save"),
            Shortcut::new(KeyPress::new(KeyCode::Esc), 2, "cancel"),
        ];
        let line = hint_line(&shortcuts).plain();
        assert!(line.contains("Ctrl-S save"));
        assert!(line.contains("Esc cancel"));
    }
}
