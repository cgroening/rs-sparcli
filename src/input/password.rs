//! Masked password input.

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::terminal::is_input_tty;
use crate::core::theme::theme;
use crate::error::{Result, SparcliError};
use crate::input::Outcome;
use crate::input::event::{CrosstermSource, EventSource, InputEvent, KeyPress};
use crate::input::field::{error_line, field_line, value_line};
use crate::input::guard::TerminalGuard;
use crate::input::line_edit::LineEditor;
use crate::input::prompt::{Flow, run_prompt};
use crate::input::validate::{CharFilter, Validator};

/// Mutable state of a running password prompt.
struct State {
    editor: LineEditor,
    error: Option<String>,
}

/// A masked password input prompt.
///
/// # Examples
///
/// ```no_run
/// use sparcli::{Outcome, PasswordInput};
///
/// # fn main() -> sparcli::Result<()> {
/// if let Outcome::Submitted(secret) = PasswordInput::new("Password:").run()? {
///     println!("read {} characters", secret.chars().count());
/// }
/// # Ok(())
/// # }
/// ```
pub struct PasswordInput {
    prompt: String,
    initial: String,
    mask: String,
    max_chars: usize,
    validator: Option<Validator>,
    char_filter: Option<CharFilter>,
}

impl PasswordInput {
    /// Creates a password prompt with the given label.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            initial: String::new(),
            mask: "*".to_string(),
            max_chars: 0,
            validator: None,
            char_filter: None,
        }
    }

    /// Sets an initial value (mainly useful for previews/screenshots).
    #[must_use]
    pub fn initial(mut self, value: impl Into<String>) -> Self {
        self.initial = value.into();
        self
    }

    /// Sets the mask glyph. An empty mask hides the length entirely.
    #[must_use]
    pub fn mask(mut self, mask: impl Into<String>) -> Self {
        self.mask = mask.into();
        self
    }

    /// Limits the number of characters (0 = unlimited).
    #[must_use]
    pub fn max_chars(mut self, max: usize) -> Self {
        self.max_chars = max;
        self
    }

    /// Sets a full-value validator.
    #[must_use]
    pub fn validate(mut self, validator: Validator) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Sets a per-character filter.
    #[must_use]
    pub fn char_filter(mut self, filter: CharFilter) -> Self {
        self.char_filter = Some(filter);
        self
    }

    /// Runs the prompt on the real terminal.
    ///
    /// # Errors
    ///
    /// Returns [`SparcliError::NoTerminal`] without an interactive terminal,
    /// or [`SparcliError::Io`] on a terminal failure.
    pub fn run(self) -> Result<Outcome<String>> {
        if !is_input_tty() {
            return Err(SparcliError::NoTerminal);
        }
        let _guard = TerminalGuard::new()?;
        let mut source = CrosstermSource;
        self.run_with(&mut source)
    }

    /// Runs the prompt against any event source (used for tests).
    fn run_with(
        &self,
        source: &mut impl EventSource,
    ) -> Result<Outcome<String>> {
        let mut state = State {
            editor: LineEditor::new(&self.initial, false),
            error: None,
        };
        run_prompt(
            source,
            &mut state,
            |state, final_frame| self.render(state, final_frame),
            |state, event| self.handle(state, event),
        )
    }

    /// Renders the prompt's static frame without running it (for previews
    /// and README screenshots).
    pub fn frame(&self) -> Rendered {
        let state = State {
            editor: LineEditor::new(&self.initial, false),
            error: None,
        };
        self.render(&state, false)
    }

    /// Builds the prompt frame with the masked value.
    fn render(&self, state: &State, final_frame: bool) -> Rendered {
        let theme = theme();
        let (display, cursor) = self.masked(state);
        if final_frame {
            let line = value_line(&self.prompt, &display, Style::new(), &theme);
            return Rendered::new(vec![line]);
        }
        let mut lines = vec![field_line(
            &self.prompt,
            &display,
            cursor,
            Style::new(),
            &theme,
        )];
        if let Some(error) = &state.error {
            lines.push(error_line(error, &theme));
        }
        Rendered::new(lines)
    }

    /// Returns the masked display string and cursor index.
    fn masked(&self, state: &State) -> (String, usize) {
        if self.mask.is_empty() {
            return (String::new(), 0);
        }
        let glyph = self.mask.chars().next().unwrap_or('*');
        let display: String =
            std::iter::repeat_n(glyph, state.editor.len()).collect();
        (display, state.editor.cursor())
    }

    /// Handles one event.
    fn handle(&self, state: &mut State, event: InputEvent) -> Flow<String> {
        match event {
            InputEvent::Paste(text) => {
                for ch in text.chars() {
                    self.type_char(state, ch);
                }
                Flow::Continue
            }
            InputEvent::Key(key) => self.handle_key(state, key),
            InputEvent::Resize => Flow::Continue,
        }
    }

    /// Handles a single key press.
    fn handle_key(&self, state: &mut State, key: KeyPress) -> Flow<String> {
        use crate::input::event::KeyCode::{
            Backspace, Char, Delete, Enter, Esc, Left, Right,
        };
        if key.ctrl {
            if let Char(c) = key.code {
                match c {
                    'u' => state.editor.kill_to_line_start(),
                    'w' => state.editor.delete_word_back(),
                    _ => {}
                }
            }
            return Flow::Continue;
        }
        match key.code {
            Esc => return Flow::Cancel,
            Enter => return self.submit(state),
            Left => state.editor.move_left(false),
            Right => state.editor.move_right(false),
            Backspace => state.editor.backspace(),
            Delete => state.editor.delete(),
            Char(c) => self.type_char(state, c),
            _ => {}
        }
        Flow::Continue
    }

    /// Validates and submits the current value.
    fn submit(&self, state: &mut State) -> Flow<String> {
        let value = state.editor.value();
        if let Some(validator) = &self.validator
            && let Err(message) = validator(&value)
        {
            state.error = Some(message);
            return Flow::Continue;
        }
        Flow::Submit(value)
    }

    /// Types one character if it passes the filter and length limit.
    fn type_char(&self, state: &mut State, ch: char) {
        if let Some(filter) = &self.char_filter
            && !filter(ch)
        {
            return;
        }
        if self.max_chars > 0 && state.editor.len() >= self.max_chars {
            return;
        }
        state.editor.insert_char(ch);
        state.error = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::event::{KeyCode, ScriptedSource};

    #[test]
    fn types_and_submits_password() {
        let outcome = PasswordInput::new("pw")
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Char('s'),
                KeyCode::Char('e'),
                KeyCode::Char('c'),
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted("sec".to_string()));
    }

    #[test]
    fn masks_render_as_glyphs() {
        let input = PasswordInput::new("pw");
        let mut state = State {
            editor: LineEditor::new("abc", false),
            error: None,
        };
        let (display, _) = input.masked(&state);
        assert_eq!(display, "***");
        state.editor.set_value("");
        assert_eq!(input.masked(&state).0, "");
    }

    #[test]
    fn esc_cancels() {
        let outcome = PasswordInput::new("pw")
            .run_with(&mut ScriptedSource::keys([KeyCode::Esc]))
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
    }
}
