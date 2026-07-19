//! Single- and multi-selection list prompt.

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::theme::{Theme, theme};
use crate::error::Result;
use crate::input::Outcome;
use crate::input::event::{EventSource, InputEvent, KeyCode, KeyPress};
use crate::input::prompt::{Flow, run_interactive, run_prompt};
use crate::input::selection::SelectionCursor;
use crate::input::shortcut::{self, Shortcut};

/// Default number of visible rows.
const DEFAULT_VISIBLE: usize = 10;

/// Mutable state of a running select prompt.
struct State {
    cursor: SelectionCursor,
    checked: Vec<bool>,
    help: bool,
}

/// A scrollable selection list (single or multi).
///
/// # Examples
///
/// ```no_run
/// use sparcli::{Outcome, Select};
///
/// # fn main() -> sparcli::Result<()> {
/// if let Outcome::Submitted(index) =
///     Select::new("Pick one").options(["red", "green", "blue"]).run()?
/// {
///     println!("chose option {index}");
/// }
/// # Ok(())
/// # }
/// ```
pub struct Select {
    prompt: String,
    options: Vec<String>,
    multi: bool,
    max_visible: usize,
    cycle: bool,
    shortcuts: Vec<Shortcut>,
    initial_cursor: usize,
    initial_checked: Vec<usize>,
}

impl Select {
    /// Creates a select prompt with the given label.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            options: Vec::new(),
            multi: false,
            max_visible: DEFAULT_VISIBLE,
            cycle: true,
            shortcuts: Vec::new(),
            initial_cursor: 0,
            initial_checked: Vec::new(),
        }
    }

    /// Sets the initially highlighted row.
    #[must_use]
    pub fn cursor(mut self, index: usize) -> Self {
        self.initial_cursor = index;
        self
    }

    /// Sets the initially checked rows (multi-select).
    #[must_use]
    pub fn checked<I>(mut self, indices: I) -> Self
    where
        I: IntoIterator<Item = usize>,
    {
        self.initial_checked = indices.into_iter().collect();
        self
    }

    /// Registers shortcuts shown in a footer hint and the `?` help overlay.
    ///
    /// Pressing a bound key ends the prompt with [`Outcome::Shortcut`].
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

    /// Sets the maximum number of visible rows.
    #[must_use]
    pub fn max_visible(mut self, rows: usize) -> Self {
        self.max_visible = rows.max(1);
        self
    }

    /// Disables wrap-around cursor navigation.
    #[must_use]
    pub fn no_cycle(mut self) -> Self {
        self.cycle = false;
        self
    }

    /// Runs a single-select prompt, returning the chosen index.
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

    /// Runs a multi-select prompt, returning all checked indices.
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
        if self.options.is_empty() {
            return Ok(Outcome::Submitted(Vec::new()));
        }
        let mut state = self.initial_state();
        run_prompt(
            source,
            &mut state,
            |state, _| self.render(state),
            |state, event| self.handle(state, event),
        )
    }

    /// Builds the starting state, honoring the initial cursor and checks.
    fn initial_state(&self) -> State {
        let len = self.options.len();
        let mut checked = vec![false; len];
        for &index in &self.initial_checked {
            if index < len {
                checked[index] = true;
            }
        }
        let mut cursor =
            SelectionCursor::new(len, self.max_visible, self.cycle);
        cursor.jump_to(self.initial_cursor);
        State {
            cursor,
            checked,
            help: false,
        }
    }

    /// Renders the prompt's static frame without running it (for previews
    /// and README screenshots).
    pub fn frame(&self) -> Rendered {
        self.render(&self.initial_state())
    }

    /// Builds the prompt frame with the visible window of options.
    fn render(&self, state: &State) -> Rendered {
        let theme = theme();
        if state.help {
            return Rendered::new(shortcut::help_overlay(&self.shortcuts));
        }
        let mut lines = vec![Line::styled(self.prompt.clone(), theme.title)];
        for index in state.cursor.window() {
            lines.push(self.option_line(state, index, &theme));
        }
        if !self.shortcuts.is_empty() {
            lines.push(shortcut::hint_line(&self.shortcuts));
        }
        Rendered::new(lines)
    }

    /// Renders one option row.
    fn option_line(&self, state: &State, index: usize, theme: &Theme) -> Line {
        let is_cursor = index == state.cursor.index();
        let marker = if is_cursor {
            theme.cursor_marker()
        } else {
            theme.marker()
        };
        let mut spans = vec![Span::styled(marker.to_string(), theme.selection)];
        if self.multi {
            let checkbox = if state.checked[index] {
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
        spans.push(Span::styled(self.options[index].clone(), style));
        Line::new(spans)
    }

    /// Handles one event.
    fn handle(&self, state: &mut State, event: InputEvent) -> Flow<Vec<usize>> {
        let InputEvent::Key(key) = event else {
            return Flow::Continue;
        };
        self.handle_key(state, key)
    }

    /// Handles a single key press.
    fn handle_key(&self, state: &mut State, key: KeyPress) -> Flow<Vec<usize>> {
        if let Some(flow) =
            shortcut::intercept(key, &self.shortcuts, &mut state.help)
        {
            return flow;
        }
        match key.code {
            KeyCode::Esc => return Flow::Cancel,
            KeyCode::Enter => return Flow::Submit(self.collect(state)),
            KeyCode::Up | KeyCode::Char('k') => state.cursor.step(-1),
            KeyCode::Down | KeyCode::Char('j') => state.cursor.step(1),
            KeyCode::Home => state.cursor.jump_to(0),
            KeyCode::End => state.cursor.jump_to(usize::MAX),
            KeyCode::PageUp => state.cursor.page(-1),
            KeyCode::PageDown => state.cursor.page(1),
            KeyCode::Char(' ') if self.multi => {
                let index = state.cursor.index();
                state.checked[index] = !state.checked[index];
            }
            _ => {}
        }
        Flow::Continue
    }

    /// Returns the result indices for the current state.
    fn collect(&self, state: &State) -> Vec<usize> {
        if self.multi {
            (0..self.options.len())
                .filter(|&i| state.checked[i])
                .collect()
        } else {
            vec![state.cursor.index()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::event::ScriptedSource;

    fn select() -> Select {
        Select::new("pick").options(["a", "b", "c"])
    }

    #[test]
    fn a_page_jump_clamps_even_though_stepping_cycles() {
        // Arrow keys wrap, but a page jump from the top must not land on the
        // bottom of the list - that reads as a lost position, not navigation.
        let outcome = select()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::PageUp,
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![0]));

        let outcome = select()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::PageDown,
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![2]));
    }

    #[test]
    fn end_and_home_jump_to_the_list_bounds() {
        let outcome = select()
            .run_with(&mut ScriptedSource::keys([KeyCode::End, KeyCode::Enter]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![2]));

        let outcome = select()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::End,
                KeyCode::Home,
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![0]));
    }

    #[test]
    fn enter_selects_cursor_in_single_mode() {
        let outcome = select()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Down,
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![1]));
    }

    #[test]
    fn cursor_cycles_past_the_end() {
        let outcome = select()
            .run_with(&mut ScriptedSource::keys([KeyCode::Up, KeyCode::Enter]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![2]));
    }

    #[test]
    fn no_cycle_clamps_at_top() {
        let outcome = select()
            .no_cycle()
            .run_with(&mut ScriptedSource::keys([KeyCode::Up, KeyCode::Enter]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![0]));
    }

    #[test]
    fn space_toggles_in_multi_mode() {
        let outcome = select()
            .multi()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Char(' '),
                KeyCode::Down,
                KeyCode::Down,
                KeyCode::Char(' '),
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![0, 2]));
    }

    #[test]
    fn shortcut_ends_with_its_id() {
        use crate::input::event::{InputEvent, KeyPress};
        let outcome = select()
            .shortcuts([Shortcut::new(KeyPress::ctrl('n'), 7, "new")])
            .run_with(&mut ScriptedSource::events([InputEvent::Key(
                KeyPress::ctrl('n'),
            )]))
            .unwrap();
        assert_eq!(outcome, Outcome::Shortcut(7));
    }

    #[test]
    fn help_overlay_opens_and_closes() {
        let outcome = select()
            .shortcuts([Shortcut::new(
                crate::input::event::KeyPress::ctrl('n'),
                1,
                "new",
            )])
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Char('?'),
                KeyCode::Char('x'),
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(vec![0]));
    }

    #[test]
    fn esc_cancels() {
        let outcome = select()
            .run_with(&mut ScriptedSource::keys([KeyCode::Esc]))
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
    }
}
