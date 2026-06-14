//! Single-line text input with validation, filtering, history and ghost
//! autocomplete.

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::terminal::is_input_tty;
use crate::core::text::{Line, Span};
use crate::core::theme::theme;
use crate::error::{Result, SparcliError};
use crate::input::Outcome;
use crate::input::editor;
use crate::input::event::{CrosstermSource, EventSource, InputEvent, KeyPress};
use crate::input::field::{
    error_line, field_line, placeholder_line, value_line,
};
use crate::input::guard::TerminalGuard;
use crate::input::history::History;
use crate::input::line_edit::LineEditor;
use crate::input::prompt::{Flow, run_prompt};
use crate::input::validate::{CharFilter, Validator};

/// Maximum number of dropdown rows shown at once.
const MAX_DROPDOWN: usize = 5;

/// Mutable state of a running text prompt.
struct State {
    editor: LineEditor,
    error: Option<String>,
    history_index: Option<usize>,
    dropdown_index: Option<usize>,
    history_entries: Vec<String>,
    store: Option<History>,
}

/// How suggestions are matched against the typed value.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MatchMode {
    Prefix,
    Subsequence,
}

/// A single-line text input prompt.
pub struct TextInput {
    prompt: String,
    initial: String,
    placeholder: String,
    max_chars: usize,
    hide_char_count: bool,
    validator: Option<Validator>,
    char_filter: Option<CharFilter>,
    suggestions: Vec<String>,
    history: Vec<String>,
    history_app: Option<String>,
    dropdown: bool,
    match_mode: MatchMode,
    editor_enabled: bool,
    editor_command: Option<String>,
}

impl TextInput {
    /// Creates a text prompt with the given label.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            initial: String::new(),
            placeholder: String::new(),
            max_chars: 0,
            hide_char_count: false,
            validator: None,
            char_filter: None,
            suggestions: Vec::new(),
            history: Vec::new(),
            history_app: None,
            dropdown: false,
            match_mode: MatchMode::Prefix,
            editor_enabled: false,
            editor_command: None,
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
    ///
    /// History is unavailable while a navigable dropdown is enabled, since
    /// Up/Down then drive the suggestion list.
    #[must_use]
    pub fn history<I, S>(mut self, entries: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.history = entries.into_iter().map(Into::into).collect();
        self
    }

    /// Shows suggestions as a navigable dropdown instead of ghost text.
    ///
    /// Up/Down move the highlight, Tab/Enter accept it.
    #[must_use]
    pub fn dropdown(mut self) -> Self {
        self.dropdown = true;
        self
    }

    /// Matches suggestions by subsequence (fuzzy) instead of prefix.
    #[must_use]
    pub fn fuzzy_suggestions(mut self) -> Self {
        self.match_mode = MatchMode::Subsequence;
        self
    }

    /// Hides the `(n/max)` character counter shown when `max_chars` is set.
    #[must_use]
    pub fn hide_char_count(mut self) -> Self {
        self.hide_char_count = true;
        self
    }

    /// Persists history under the app's state dir, recalling and auto-adding.
    ///
    /// Loads previous entries for Up/Down recall and appends the submitted
    /// value on success. Overrides [`history`](Self::history).
    #[must_use]
    pub fn history_app(mut self, app: impl Into<String>) -> Self {
        self.history_app = Some(app.into());
        self
    }

    /// Enables opening the value in `$EDITOR` with Ctrl-G.
    #[must_use]
    pub fn editor(mut self) -> Self {
        self.editor_enabled = true;
        self
    }

    /// Sets the editor command (implies [`editor`](Self::editor)).
    #[must_use]
    pub fn editor_command(mut self, command: impl Into<String>) -> Self {
        self.editor_enabled = true;
        self.editor_command = Some(command.into());
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
        let (store, history_entries) = self.load_history();
        let mut state = State {
            editor: LineEditor::new(&self.initial, false),
            error: None,
            history_index: None,
            dropdown_index: None,
            history_entries,
            store,
        };
        run_prompt(
            source,
            &mut state,
            |state, final_frame| self.render(state, final_frame),
            |state, event| self.handle(state, event),
        )
    }

    /// Loads the persistent history store and the entries used for recall.
    fn load_history(&self) -> (Option<History>, Vec<String>) {
        let Some(app) = &self.history_app else {
            return (None, self.history.clone());
        };
        let mut store = History::for_app(app);
        let _ = store.load();
        let entries = store.entries().to_vec();
        (Some(store), entries)
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
            if !self.dropdown
                && let Some(ghost) = self.ghost(&value)
            {
                line.spans.push(Span::styled(ghost, theme.secondary));
            }
            lines.push(line);
        }
        if self.max_chars > 0
            && !self.hide_char_count
            && let Some(line) = lines.last_mut()
        {
            let count = format!(" ({}/{})", state.editor.len(), self.max_chars);
            line.spans.push(Span::styled(count, theme.secondary));
        }
        if self.dropdown {
            self.push_dropdown(&mut lines, state, &value, &theme);
        }
        if let Some(error) = &state.error {
            lines.push(error_line(error, &theme));
        }
        Rendered::new(lines)
    }

    /// Appends the dropdown rows for the current matches.
    fn push_dropdown(
        &self,
        lines: &mut Vec<Line>,
        state: &State,
        value: &str,
        theme: &crate::core::theme::Theme,
    ) {
        let matches = self.matches(value);
        for (row, &index) in matches.iter().take(MAX_DROPDOWN).enumerate() {
            let active = state.dropdown_index == Some(row);
            let marker = if active {
                theme.cursor_marker()
            } else {
                theme.marker()
            };
            let style = if active {
                theme.selection
            } else {
                theme.secondary
            };
            lines.push(Line::new(vec![
                Span::styled(marker.to_string(), theme.selection),
                Span::styled(self.suggestions[index].clone(), style),
            ]));
        }
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

    /// Returns the suggestion indices matching `value` (in declared order).
    fn matches(&self, value: &str) -> Vec<usize> {
        if value.is_empty() {
            return Vec::new();
        }
        let needle = value.to_lowercase();
        self.suggestions
            .iter()
            .enumerate()
            .filter(|(_, s)| matches_suggestion(&needle, s, self.match_mode))
            .map(|(index, _)| index)
            .collect()
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
            Enter => return self.on_enter(state),
            Tab => self.accept_completion(state),
            Up if self.dropdown => self.dropdown_move(state, -1),
            Down if self.dropdown => self.dropdown_move(state, 1),
            Up => self.history_prev(state),
            Down => self.history_next(state),
            Left => state.editor.move_left(key.shift),
            Right => state.editor.move_right(key.shift),
            Home => state.editor.move_home(key.shift),
            End => state.editor.move_end(key.shift),
            Backspace => {
                state.editor.backspace();
                state.dropdown_index = None;
            }
            Delete => {
                state.editor.delete();
                state.dropdown_index = None;
            }
            Char(c) => self.type_char(state, c),
            _ => {}
        }
        Flow::Continue
    }

    /// Enter accepts a highlighted dropdown row, otherwise submits.
    fn on_enter(&self, state: &mut State) -> Flow<String> {
        if self.dropdown && state.dropdown_index.is_some() {
            self.accept_completion(state);
            return Flow::Continue;
        }
        self.submit(state)
    }

    /// Moves the dropdown highlight, cycling over the current matches.
    fn dropdown_move(&self, state: &mut State, delta: isize) {
        let count = self.matches(&state.editor.value()).len().min(MAX_DROPDOWN);
        if count == 0 {
            state.dropdown_index = None;
            return;
        }
        let next = match state.dropdown_index {
            None if delta > 0 => 0,
            None => count - 1,
            Some(i) => (i as isize + delta).rem_euclid(count as isize) as usize,
        };
        state.dropdown_index = Some(next);
    }

    /// Fills the field from the highlighted match (or the ghost completion).
    fn accept_completion(&self, state: &mut State) {
        if self.dropdown {
            let matches = self.matches(&state.editor.value());
            let row = state.dropdown_index.unwrap_or(0);
            if let Some(&index) = matches.get(row) {
                state.editor.set_value(&self.suggestions[index]);
                state.dropdown_index = None;
            }
            return;
        }
        self.accept_ghost(state);
    }

    /// Handles Ctrl-modified editing keys.
    fn handle_ctrl(&self, state: &mut State, key: KeyPress) -> Flow<String> {
        use crate::input::event::KeyCode::Char;
        if let Char(c) = key.code {
            match c {
                'g' if self.editor_enabled => return self.launch_editor(state),
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

    /// Opens the value in an external editor, then refreshes the prompt.
    fn launch_editor(&self, state: &mut State) -> Flow<String> {
        let _ = crossterm::terminal::disable_raw_mode();
        let command = self.editor_command.as_deref();
        let result = editor::edit_text(command, &state.editor.value(), ".txt");
        let _ = crossterm::terminal::enable_raw_mode();
        if let Ok(text) = result {
            let single_line = text.replace('\n', " ");
            state.editor.set_value(single_line.trim_end());
            state.dropdown_index = None;
        }
        Flow::Refresh
    }

    /// Validates and submits the current value, persisting history.
    fn submit(&self, state: &mut State) -> Flow<String> {
        let value = state.editor.value();
        if let Some(validator) = &self.validator
            && let Err(message) = validator(&value)
        {
            state.error = Some(message);
            return Flow::Continue;
        }
        if let Some(store) = &mut state.store {
            store.add(&value);
            let _ = store.save();
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
        state.dropdown_index = None;
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
        if state.history_entries.is_empty() {
            return;
        }
        let index = match state.history_index {
            None => state.history_entries.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        state.history_index = Some(index);
        state.editor.set_value(&state.history_entries[index]);
    }

    /// Recalls the next history entry, clearing past the newest.
    fn history_next(&self, state: &mut State) {
        let Some(index) = state.history_index else {
            return;
        };
        if index + 1 < state.history_entries.len() {
            state.history_index = Some(index + 1);
            state.editor.set_value(&state.history_entries[index + 1]);
        } else {
            state.history_index = None;
            state.editor.set_value("");
        }
    }
}

/// Returns whether `suggestion` matches the lowercase `needle`.
fn matches_suggestion(needle: &str, suggestion: &str, mode: MatchMode) -> bool {
    let hay = suggestion.to_lowercase();
    match mode {
        MatchMode::Prefix => hay.starts_with(needle),
        MatchMode::Subsequence => is_subsequence(needle, &hay),
    }
}

/// Returns whether all chars of `needle` appear in `hay` in order.
fn is_subsequence(needle: &str, hay: &str) -> bool {
    let mut chars = hay.chars();
    needle.chars().all(|target| chars.any(|c| c == target))
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

    #[cfg(unix)]
    #[test]
    fn ctrl_g_round_trips_through_the_editor() {
        use crate::input::event::{InputEvent, KeyPress};
        // `true` does not modify the temp file, so the value is unchanged.
        let outcome = TextInput::new("x")
            .editor_command("true")
            .initial("hi")
            .run_with(&mut ScriptedSource::events([
                InputEvent::Key(KeyPress::ctrl('g')),
                InputEvent::Key(KeyPress::new(KeyCode::Enter)),
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted("hi".to_string()));
    }

    #[test]
    fn final_frame_drops_the_cursor() {
        let input = TextInput::new("Name");
        let state = State {
            editor: LineEditor::new("asd", false),
            error: None,
            history_index: None,
            dropdown_index: None,
            history_entries: Vec::new(),
            store: None,
        };
        // Active frame draws a block cursor (trailing space past the value).
        let active = input.render(&state, false).lines[0].plain();
        assert!(active.ends_with(' '));
        // Final frame is the bare value, no cursor.
        let finished = input.render(&state, true).lines[0].plain();
        assert_eq!(finished, "Name asd");
    }

    #[test]
    fn dropdown_navigates_and_enter_accepts() {
        // Type "ap" -> matches apple/apricot; Down highlights apple; Enter
        // fills it (stays open), second Enter submits.
        let outcome = run(
            &TextInput::new("x")
                .dropdown()
                .suggestions(["apple", "apricot", "banana"]),
            vec![
                KeyCode::Char('a'),
                KeyCode::Char('p'),
                KeyCode::Down,
                KeyCode::Enter,
                KeyCode::Enter,
            ],
        );
        assert_eq!(outcome, Outcome::Submitted("apple".to_string()));
    }

    #[test]
    fn fuzzy_suggestions_match_subsequence() {
        let outcome = run(
            &TextInput::new("x")
                .dropdown()
                .fuzzy_suggestions()
                .suggestions(["foobar"]),
            vec![
                KeyCode::Char('f'),
                KeyCode::Char('b'),
                KeyCode::Tab,
                KeyCode::Enter,
            ],
        );
        assert_eq!(outcome, Outcome::Submitted("foobar".to_string()));
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
