//! Keyboard event abstraction over [`crossterm`], with a fake for tests.
//!
//! Prompts read [`InputEvent`]s from an [`EventSource`]. The real source wraps
//! crossterm; tests use a scripted source so prompts can be driven headlessly.

use crossterm::event::{
    self, Event, KeyCode as CtCode, KeyEvent, KeyEventKind, KeyModifiers,
};

use crate::error::Result;

/// A logical key, independent of the terminal backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    /// A printable character.
    Char(char),
    /// Enter / Return.
    Enter,
    /// Escape.
    Esc,
    /// Tab.
    Tab,
    /// Shift-Tab (back-tab).
    BackTab,
    /// Backspace.
    Backspace,
    /// Delete.
    Delete,
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// A function key (`F1`..`F12`).
    Function(u8),
    /// Any key not mapped above.
    Unknown,
}

/// A key press with its modifier flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyPress {
    /// The logical key.
    pub code: KeyCode,
    /// Control modifier.
    pub ctrl: bool,
    /// Alt modifier.
    pub alt: bool,
    /// Shift modifier.
    pub shift: bool,
}

impl KeyPress {
    /// Creates a key press without modifiers.
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    /// Creates a Ctrl + letter key press.
    pub fn ctrl(letter: char) -> Self {
        Self {
            code: KeyCode::Char(letter),
            ctrl: true,
            alt: false,
            shift: false,
        }
    }

    /// Returns `true` if this is Ctrl + the given letter.
    pub fn is_ctrl(&self, letter: char) -> bool {
        self.ctrl && self.code == KeyCode::Char(letter)
    }
}

/// An input event delivered to a prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    /// A key was pressed.
    Key(KeyPress),
    /// A bracketed-paste payload arrived.
    Paste(String),
    /// The terminal was resized.
    Resize,
}

/// A source of input events.
pub trait EventSource {
    /// Blocks until the next event is available.
    ///
    /// # Errors
    /// Returns an error if the underlying terminal read fails.
    fn next_event(&mut self) -> Result<InputEvent>;

    /// Whether this source drives a real, interactive terminal.
    ///
    /// Non-interactive sources (e.g. scripted tests) must not draw to the
    /// terminal, so the prompt loop skips all rendering for them.
    fn is_interactive(&self) -> bool {
        true
    }
}

/// The real event source, backed by crossterm.
pub struct CrosstermSource;

impl EventSource for CrosstermSource {
    fn next_event(&mut self) -> Result<InputEvent> {
        loop {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    return Ok(InputEvent::Key(map_key(key)));
                }
                Event::Paste(text) => return Ok(InputEvent::Paste(text)),
                Event::Resize(_, _) => return Ok(InputEvent::Resize),
                _ => continue,
            }
        }
    }
}

/// Maps a crossterm key event to a [`KeyPress`].
fn map_key(key: KeyEvent) -> KeyPress {
    let mods = key.modifiers;
    KeyPress {
        code: map_code(key.code),
        ctrl: mods.contains(KeyModifiers::CONTROL),
        alt: mods.contains(KeyModifiers::ALT),
        shift: mods.contains(KeyModifiers::SHIFT),
    }
}

/// Maps a crossterm key code to a [`KeyCode`].
fn map_code(code: CtCode) -> KeyCode {
    match code {
        CtCode::Char(c) => KeyCode::Char(c),
        CtCode::Enter => KeyCode::Enter,
        CtCode::Esc => KeyCode::Esc,
        CtCode::Tab => KeyCode::Tab,
        CtCode::BackTab => KeyCode::BackTab,
        CtCode::Backspace => KeyCode::Backspace,
        CtCode::Delete => KeyCode::Delete,
        CtCode::Up => KeyCode::Up,
        CtCode::Down => KeyCode::Down,
        CtCode::Left => KeyCode::Left,
        CtCode::Right => KeyCode::Right,
        CtCode::Home => KeyCode::Home,
        CtCode::End => KeyCode::End,
        CtCode::PageUp => KeyCode::PageUp,
        CtCode::PageDown => KeyCode::PageDown,
        CtCode::F(n) => KeyCode::Function(n),
        _ => KeyCode::Unknown,
    }
}

/// A scripted event source for tests: yields queued events in order.
#[cfg(test)]
pub(crate) struct ScriptedSource {
    events: std::collections::VecDeque<InputEvent>,
}

#[cfg(test)]
impl ScriptedSource {
    /// Builds a source from a sequence of key presses.
    pub(crate) fn keys(codes: impl IntoIterator<Item = KeyCode>) -> Self {
        let events = codes
            .into_iter()
            .map(|code| InputEvent::Key(KeyPress::new(code)))
            .collect();
        Self { events }
    }

    /// Builds a source from explicit events.
    #[allow(dead_code)]
    pub(crate) fn events(events: impl IntoIterator<Item = InputEvent>) -> Self {
        Self {
            events: events.into_iter().collect(),
        }
    }
}

#[cfg(test)]
impl EventSource for ScriptedSource {
    fn next_event(&mut self) -> Result<InputEvent> {
        // Exhaustion cancels the prompt, preventing infinite loops in tests.
        Ok(self
            .events
            .pop_front()
            .unwrap_or(InputEvent::Key(KeyPress::new(KeyCode::Esc))))
    }

    fn is_interactive(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_source_yields_then_cancels() {
        let mut source = ScriptedSource::keys([KeyCode::Char('a')]);
        assert_eq!(
            source.next_event().unwrap(),
            InputEvent::Key(KeyPress::new(KeyCode::Char('a')))
        );
        // Exhausted: yields Esc.
        assert_eq!(
            source.next_event().unwrap(),
            InputEvent::Key(KeyPress::new(KeyCode::Esc))
        );
    }

    #[test]
    fn ctrl_helper_matches() {
        assert!(KeyPress::ctrl('c').is_ctrl('c'));
        assert!(!KeyPress::new(KeyCode::Char('c')).is_ctrl('c'));
    }
}
