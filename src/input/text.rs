//! Single-line text input with validation, filtering, history and ghost
//! autocomplete.

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::terminal::is_input_tty;
use crate::core::text::Span;
use crate::core::theme::theme;
use crate::error::{Result, SparcliError};
use crate::input::Outcome;
use crate::input::event::{CrosstermSource, EventSource, InputEvent, KeyPress};
use crate::input::field::{
    error_line, field_line, placeholder_line, value_line,
};
use crate::input::guard::TerminalGuard;
use crate::input::line_edit::LineEditor;
use crate::input::prompt::{Flow, run_prompt};
use crate::input::validate::{CharFilter, Validator};

/// Mutable state of a running text prompt.
struct State {
    editor: LineEditor,
    error: Option<String>,
    history_index: Option<usize>,
}

/// A single-line text input prompt.
pub struct TextInput {
    prompt: String,
    initial: String,
    placeholder: String,
    max_chars: usize,
    validator: Option<Validator>,
    char_filter: Option<CharFilter>,
    suggestions: Vec<String>,
    history: Vec<String>,
}

impl TextInput {
    /// Creates a text prompt with the given label.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            initial: String::new(),
            placeholder: String::new(),
            max_chars: 0,
            validator: None,
            char_filter: None,
            suggestions: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Sets the initial value.
    #[must_use]
    pub fn initial(mut self, value: impl Into<String>) -> Self {
        self.initial = value.into();
        self
    }

    /// Sets the placeholder shown when empty.
    #[must_use]
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = value.into();
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

    /// Sets autocomplete suggestions (prefix-matched ghost text).
    #[must_use]
    pub fn suggestions<I, S>(mut self, suggestions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.suggestions = suggestions.into_iter().map(Into::into).collect();
        self
    }

    /// Provides history entries recalled with Up/Down.
    #[must_use]
    pub fn history<I, S>(mut self, entries: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.history = entries.into_iter().map(Into::into).collect();
        self
    }

    /// Runs the prompt on the real terminal.
    ///
    /// # Errors
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
            history_index: None,
        };
        run_prompt(
            source,
            &mut state,
            |state, final_frame| self.render(state, final_frame),
            |state, event| self.handle(state, event),
        )
    }

    /// Builds the prompt frame.
    fn render(&self, state: &State, final_frame: bool) -> Rendered {
        let theme = theme();
        let value = state.editor.value();
        if final_frame {
            let line = value_line(&self.prompt, &value, Style::new(), &theme);
            return Rendered::new(vec![line]);
        }
        let mut lines = Vec::new();
        if value.is_empty() && !self.placeholder.is_empty() {
            lines.push(placeholder_line(
                &self.prompt,
                &self.placeholder,
                &theme,
            ));
        } else {
            let mut line = field_line(
                &self.prompt,
                &value,
                state.editor.cursor(),
                Style::new(),
                &theme,
            );
            if let Some(ghost) = self.ghost(&value) {
                line.spans.push(Span::styled(ghost, theme.secondary));
            }
            lines.push(line);
        }
        if let Some(error) = &state.error {
            lines.push(error_line(error, &theme));
        }
        Rendered::new(lines)
    }

    /// Returns the ghost completion suffix for `value`, if any.
    fn ghost(&self, value: &str) -> Option<String> {
        if value.is_empty() {
            return None;
        }
        self.suggestions
            .iter()
            .find(|s| s.starts_with(value) && s.len() > value.len())
            .map(|s| s[value.len()..].to_string())
    }

    /// Handles one event.
    fn handle(&self, state: &mut State, event: InputEvent) -> Flow<String> {
        match event {
            InputEvent::Paste(text) => {
                self.insert_filtered(state, &text);
                Flow::Continue
            }
            InputEvent::Key(key) => self.handle_key(state, key),
            InputEvent::Resize => Flow::Continue,
        }
    }

    /// Handles a single key press.
    fn handle_key(&self, state: &mut State, key: KeyPress) -> Flow<String> {
        use crate::input::event::KeyCode::{
            Backspace, Char, Delete, Down, End, Enter, Esc, Home, Left, Right,
            Tab, Up,
        };
        if key.ctrl {
            return self.handle_ctrl(state, key);
        }
        match key.code {
            Esc => return Flow::Cancel,
            Enter => return self.submit(state),
            Tab => self.accept_ghost(state),
            Up => self.history_prev(state),
            Down => self.history_next(state),
            Left => state.editor.move_left(key.shift),
            Right => state.editor.move_right(key.shift),
            Home => state.editor.move_home(key.shift),
            End => state.editor.move_end(key.shift),
            Backspace => state.editor.backspace(),
            Delete => state.editor.delete(),
            Char(c) => self.type_char(state, c),
            _ => {}
        }
        Flow::Continue
    }

    /// Handles Ctrl-modified editing keys.
    fn handle_ctrl(&self, state: &mut State, key: KeyPress) -> Flow<String> {
        use crate::input::event::KeyCode::Char;
        if let Char(c) = key.code {
            match c {
                'a' => state.editor.select_all(),
                'w' => state.editor.delete_word_back(),
                'u' => state.editor.kill_to_line_start(),
                'k' => state.editor.kill_to_line_end(),
                'c' => state.editor.copy(),
                'x' => state.editor.cut(),
                'v' => state.editor.paste(),
                _ => {}
            }
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

    /// Inserts pasted text, applying the character filter.
    fn insert_filtered(&self, state: &mut State, text: &str) {
        for ch in text.chars() {
            self.type_char(state, ch);
        }
    }

    /// Accepts the ghost completion, if present.
    fn accept_ghost(&self, state: &mut State) {
        let value = state.editor.value();
        if let Some(ghost) = self.ghost(&value) {
            state.editor.set_value(&format!("{value}{ghost}"));
        }
    }

    /// Recalls the previous history entry.
    fn history_prev(&self, state: &mut State) {
        if self.history.is_empty() {
            return;
        }
        let index = match state.history_index {
            None => self.history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        state.history_index = Some(index);
        state.editor.set_value(&self.history[index]);
    }

    /// Recalls the next history entry, clearing past the newest.
    fn history_next(&self, state: &mut State) {
        let Some(index) = state.history_index else {
            return;
        };
        if index + 1 < self.history.len() {
            state.history_index = Some(index + 1);
            state.editor.set_value(&self.history[index + 1]);
        } else {
            state.history_index = None;
            state.editor.set_value("");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::event::{KeyCode, ScriptedSource};
    use crate::input::validate::{min_len, non_empty};

    fn run(input: &TextInput, keys: Vec<KeyCode>) -> Outcome<String> {
        input.run_with(&mut ScriptedSource::keys(keys)).unwrap()
    }

    #[test]
    fn types_and_submits_value() {
        let outcome = run(
            &TextInput::new("Name"),
            vec![KeyCode::Char('h'), KeyCode::Char('i'), KeyCode::Enter],
        );
        assert_eq!(outcome, Outcome::Submitted("hi".to_string()));
    }

    #[test]
    fn backspace_edits_value() {
        let outcome = run(
            &TextInput::new("x").initial("ab"),
            vec![KeyCode::Backspace, KeyCode::Enter],
        );
        assert_eq!(outcome, Outcome::Submitted("a".to_string()));
    }

    #[test]
    fn validation_blocks_submit_until_valid() {
        // First Enter fails (empty), then type, then Enter succeeds.
        let outcome = run(
            &TextInput::new("x").validate(non_empty()),
            vec![KeyCode::Enter, KeyCode::Char('a'), KeyCode::Enter],
        );
        assert_eq!(outcome, Outcome::Submitted("a".to_string()));
    }

    #[test]
    fn min_len_validator_is_enforced() {
        let outcome = run(
            &TextInput::new("x").validate(min_len(2)),
            vec![
                KeyCode::Char('a'),
                KeyCode::Enter,
                KeyCode::Char('b'),
                KeyCode::Enter,
            ],
        );
        assert_eq!(outcome, Outcome::Submitted("ab".to_string()));
    }

    #[test]
    fn esc_cancels() {
        let outcome = run(&TextInput::new("x"), vec![KeyCode::Esc]);
        assert_eq!(outcome, Outcome::Cancelled);
    }

    #[test]
    fn tab_accepts_ghost_suggestion() {
        let outcome = run(
            &TextInput::new("x").suggestions(["hello"]),
            vec![KeyCode::Char('h'), KeyCode::Tab, KeyCode::Enter],
        );
        assert_eq!(outcome, Outcome::Submitted("hello".to_string()));
    }

    #[test]
    fn final_frame_drops_the_cursor() {
        let input = TextInput::new("Name");
        let state = State {
            editor: LineEditor::new("asd", false),
            error: None,
            history_index: None,
        };
        // Active frame draws a block cursor (trailing space past the value).
        let active = input.render(&state, false).lines[0].plain();
        assert!(active.ends_with(' '));
        // Final frame is the bare value, no cursor.
        let finished = input.render(&state, true).lines[0].plain();
        assert_eq!(finished, "Name asd");
    }

    #[test]
    fn history_recall_with_up() {
        let outcome = run(
            &TextInput::new("x").history(["first", "second"]),
            vec![KeyCode::Up, KeyCode::Enter],
        );
        assert_eq!(outcome, Outcome::Submitted("second".to_string()));
    }
}
