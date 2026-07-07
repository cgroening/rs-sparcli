//! Single-line text input with validation, filtering, history and ghost
//! autocomplete.

mod keys;
mod render;
mod suggest;

use crate::core::render::Rendered;
use crate::core::terminal::is_input_tty;
use crate::error::{Result, SparcliError};
use crate::input::Outcome;
use crate::input::event::{CrosstermSource, EventSource};
use crate::input::guard::TerminalGuard;
use crate::input::history::History;
use crate::input::line_edit::LineEditor;
use crate::input::prompt::run_prompt;
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
///
/// # Examples
///
/// ```no_run
/// use sparcli::{Outcome, TextInput};
/// use sparcli::validate::non_empty;
///
/// # fn main() -> sparcli::Result<()> {
/// if let Outcome::Submitted(name) =
///     TextInput::new("Your name?").validate(non_empty()).run()?
/// {
///     println!("Hello, {name}!");
/// }
/// # Ok(())
/// # }
/// ```
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
        if let Err(error) = store.load() {
            log::debug!("could not load input history: {error}");
        }
        let entries = store.entries().to_vec();
        (Some(store), entries)
    }

    /// Renders the prompt's static frame without running it (for previews
    /// and README screenshots).
    pub fn frame(&self) -> Rendered {
        let state = State {
            editor: LineEditor::new(&self.initial, false),
            error: None,
            history_index: None,
            dropdown_index: None,
            history_entries: self.history.clone(),
            store: None,
        };
        self.render(&state, false)
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
