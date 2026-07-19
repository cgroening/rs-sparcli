//! Inline fuzzy-select prompt backed by `nucleo-matcher`.
//!
//! A lightweight fzf-style picker: type to filter, navigate the result list,
//! select one or several entries. The heavier fullscreen/modal/table variants
//! intentionally live in a separate ratatui-based crate.

// https://crates.io/crates/nucleo-matcher
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::theme::{Theme, theme};
use crate::error::Result;
use crate::input::Outcome;
use crate::input::event::{EventSource, InputEvent, KeyCode, KeyPress};
use crate::input::line_edit::LineEditor;
use crate::input::prompt::{Flow, run_interactive, run_prompt};
use crate::input::selection::SelectionCursor;
use crate::input::shortcut::{self, Shortcut};

/// Default number of visible result rows.
const DEFAULT_VISIBLE: usize = 10;

/// Mutable state of a running fuzzy prompt.
struct State {
    query: LineEditor,
    filtered: Vec<usize>,
    cursor: SelectionCursor,
    checked: Vec<bool>,
}

/// An inline fuzzy-select prompt.
///
/// # Examples
///
/// ```no_run
/// use sparcli::{FuzzySelect, Outcome};
///
/// # fn main() -> sparcli::Result<()> {
/// if let Outcome::Submitted(index) =
///     FuzzySelect::new("Find a fruit:").options(["apple", "banana"]).run()?
/// {
///     println!("chose option {index}");
/// }
/// # Ok(())
/// # }
/// ```
pub struct FuzzySelect {
    prompt: String,
    options: Vec<String>,
    max_visible: usize,
    multi: bool,
    shortcuts: Vec<Shortcut>,
    initial_query: String,
}

impl FuzzySelect {
    /// Creates a fuzzy prompt with the given label.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            options: Vec::new(),
            max_visible: DEFAULT_VISIBLE,
            multi: false,
            shortcuts: Vec::new(),
            initial_query: String::new(),
        }
    }

    /// Pre-fills the search query.
    #[must_use]
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.initial_query = query.into();
        self
    }

    /// Registers shortcuts shown in a footer hint.
    ///
    /// Pressing a bound key ends the prompt with [`Outcome::Shortcut`]. Use
    /// modified keys (e.g. Ctrl-…) so they do not collide with typing.
    #[must_use]
    pub fn shortcuts<I>(mut self, shortcuts: I) -> Self
    where
        I: IntoIterator<Item = Shortcut>,
    {
        self.shortcuts = shortcuts.into_iter().collect();
        self
    }

    /// Adds options from any string iterator.
    #[must_use]
    pub fn options<I, S>(mut self, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.options = options.into_iter().map(Into::into).collect();
        self
    }

    /// Enables multi-selection with checkboxes.
    #[must_use]
    pub fn multi(mut self) -> Self {
        self.multi = true;
        self
    }

    /// Sets the maximum number of visible result rows.
    #[must_use]
    pub fn max_visible(mut self, rows: usize) -> Self {
        self.max_visible = rows.max(1);
        self
    }

    /// Runs a single-select fuzzy prompt, returning the chosen index.
    ///
    /// # Errors
    ///
    /// Returns [`SparcliError::NoTerminal`] without an interactive terminal,
    /// or [`SparcliError::Io`] on a terminal failure.
    pub fn run(self) -> Result<Outcome<usize>> {
        let outcome = self.run_collect()?;
        Ok(match outcome {
            Outcome::Submitted(indices) => indices
                .first()
                .copied()
                .map_or(Outcome::Cancelled, Outcome::Submitted),
            Outcome::Cancelled => Outcome::Cancelled,
            Outcome::Shortcut(id) => Outcome::Shortcut(id),
        })
    }

    /// Runs a multi-select fuzzy prompt, returning all checked indices.
    ///
    /// # Errors
    ///
    /// Returns [`SparcliError::NoTerminal`] without an interactive terminal,
    /// or [`SparcliError::Io`] on a terminal failure.
    pub fn run_multi(self) -> Result<Outcome<Vec<usize>>> {
        self.run_collect()
    }

    /// Sets up the terminal and runs the loop.
    fn run_collect(self) -> Result<Outcome<Vec<usize>>> {
        run_interactive(|source| self.run_with(source))
    }

    /// Runs the prompt against any event source (used for tests).
    fn run_with(
        &self,
        source: &mut impl EventSource,
    ) -> Result<Outcome<Vec<usize>>> {
        let mut state = self.initial_state();
        run_prompt(
            source,
            &mut state,
            |state, final_frame| self.render(state, final_frame),
            |state, event| self.handle(state, event),
        )
    }

    /// Builds the starting state, applying the initial query.
    fn initial_state(&self) -> State {
        let filtered = self.filter(&self.initial_query);
        State {
            query: LineEditor::new(&self.initial_query, false),
            cursor: SelectionCursor::new(
                filtered.len(),
                self.max_visible,
                true,
            ),
            filtered,
            checked: vec![false; self.options.len()],
        }
    }

    /// Renders the prompt's static frame without running it (for previews
    /// and README screenshots).
    pub fn frame(&self) -> Rendered {
        self.render(&self.initial_state(), false)
    }

    /// Builds the prompt frame: query field plus filtered results.
    ///
    /// The final frame omits the query cursor.
    fn render(&self, state: &State, final_frame: bool) -> Rendered {
        let theme = theme();
        let mut lines =
            vec![query_line(&self.prompt, state, &theme, final_frame)];
        for row in state.cursor.window() {
            lines.push(self.result_line(state, row, &theme));
        }
        if !final_frame && !self.shortcuts.is_empty() {
            lines.push(shortcut::hint_line(&self.shortcuts));
        }
        Rendered::new(lines)
    }

    /// Renders one result row (at filtered position `row`).
    fn result_line(&self, state: &State, row: usize, theme: &Theme) -> Line {
        let option_index = state.filtered[row];
        let is_cursor = row == state.cursor.index();
        let marker = if is_cursor {
            theme.cursor_marker()
        } else {
            theme.marker()
        };
        let mut spans = vec![Span::styled(marker.to_string(), theme.selection)];
        if self.multi {
            let checkbox = if state.checked[option_index] {
                theme.checkbox_on()
            } else {
                theme.checkbox_off()
            };
            spans.push(Span::raw(checkbox.to_string()));
        }
        let style = if is_cursor {
            theme.selection
        } else {
            Style::new()
        };
        spans.push(Span::styled(self.options[option_index].clone(), style));
        Line::new(spans)
    }

    /// Handles one event.
    fn handle(&self, state: &mut State, event: InputEvent) -> Flow<Vec<usize>> {
        match event {
            InputEvent::Paste(text) => {
                state.query.insert_str(&text);
                self.refilter(state);
                Flow::Continue
            }
            InputEvent::Key(key) => self.handle_key(state, key),
            InputEvent::Resize => Flow::Continue,
        }
    }

    /// Handles a single key press.
    fn handle_key(&self, state: &mut State, key: KeyPress) -> Flow<Vec<usize>> {
        if let Some(id) = shortcut::find(key, &self.shortcuts) {
            return Flow::Shortcut(id);
        }
        match key.code {
            KeyCode::Esc => return Flow::Cancel,
            KeyCode::Enter => return self.submit(state),
            KeyCode::Up => state.cursor.step(-1),
            KeyCode::Down => state.cursor.step(1),
            KeyCode::Home => state.cursor.jump_to(0),
            KeyCode::End => state.cursor.jump_to(usize::MAX),
            KeyCode::PageUp => state.cursor.page(-1),
            KeyCode::PageDown => state.cursor.page(1),
            KeyCode::Char(' ') if self.multi => self.toggle(state),
            KeyCode::Backspace => {
                state.query.backspace();
                self.refilter(state);
            }
            KeyCode::Char(c) => {
                state.query.insert_char(c);
                self.refilter(state);
            }
            _ => {}
        }
        Flow::Continue
    }

    /// Submits the current selection if possible.
    fn submit(&self, state: &State) -> Flow<Vec<usize>> {
        if self.multi {
            let indices = (0..self.options.len())
                .filter(|&i| state.checked[i])
                .collect();
            return Flow::Submit(indices);
        }
        match state.filtered.get(state.cursor.index()) {
            Some(&index) => Flow::Submit(vec![index]),
            None => Flow::Continue,
        }
    }

    /// Toggles the checkbox of the row under the cursor.
    fn toggle(&self, state: &mut State) {
        if let Some(&index) = state.filtered.get(state.cursor.index()) {
            state.checked[index] = !state.checked[index];
        }
    }

    /// Recomputes the filtered list and resets the cursor to the top.
    fn refilter(&self, state: &mut State) {
        state.filtered = self.filter(&state.query.value());
        state.cursor.set_len(state.filtered.len());
        state.cursor.reset();
    }

    /// Filters and ranks options for `query` (original order when empty).
    fn filter(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return (0..self.options.len()).collect();
        }
        let pattern =
            Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut matcher = Matcher::new(Config::DEFAULT);
        let mut buf = Vec::new();
        let mut scored: Vec<(usize, u32)> = Vec::new();
        for (index, option) in self.options.iter().enumerate() {
            buf.clear();
            let haystack = Utf32Str::new(option, &mut buf);
            if let Some(score) = pattern.score(haystack, &mut matcher) {
                scored.push((index, score));
            }
        }
        scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        scored.into_iter().map(|(index, _)| index).collect()
    }
}

/// Builds the query input line (the cursor is hidden on the final frame).
fn query_line(
    prompt: &str,
    state: &State,
    theme: &Theme,
    final_frame: bool,
) -> Line {
    let mut spans = vec![Span::styled(format!("{prompt} "), theme.title)];
    spans.push(Span::raw(state.query.value()));
    if !final_frame {
        spans.push(Span::styled(" ".to_string(), theme.cursor));
    }
    Line::new(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::event::ScriptedSource;

    fn fuzzy() -> FuzzySelect {
        FuzzySelect::new("find").options(["apple", "banana", "cherry", "grape"])
    }

    #[test]
    fn end_jumps_to_the_last_result_and_home_back_to_the_first() {
        let outcome = fuzzy()
            .run_with(&mut ScriptedSource::keys([KeyCode::End, KeyCode::Enter]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![3]));

        let outcome = fuzzy()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::End,
                KeyCode::Home,
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![0]));
    }

    #[test]
    fn a_page_jump_clamps_at_both_ends_of_the_result_list() {
        // Four options fit inside one page, so PageDown lands on the last row
        // and PageUp on the first, rather than wrapping around.
        let outcome = fuzzy()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::PageDown,
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![3]));

        let outcome = fuzzy()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::PageUp,
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![0]));
    }

    #[test]
    fn typing_filters_and_enter_selects() {
        let outcome = fuzzy()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Char('c'),
                KeyCode::Char('h'),
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![2]));
    }

    #[test]
    fn empty_query_shows_all_in_order() {
        let outcome = fuzzy()
            .run_with(&mut ScriptedSource::keys([KeyCode::Enter]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![0]));
    }

    #[test]
    fn multi_select_collects_checked() {
        let outcome = fuzzy()
            .multi()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Char(' '),
                KeyCode::Down,
                KeyCode::Char(' '),
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![0, 1]));
    }

    #[test]
    fn esc_cancels() {
        let outcome = fuzzy()
            .run_with(&mut ScriptedSource::keys([KeyCode::Esc]))
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
    }
}
