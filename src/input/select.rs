//! Single- and multi-selection list prompt.

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::terminal::is_input_tty;
use crate::core::text::{Line, Span};
use crate::core::theme::{Theme, theme};
use crate::error::{Result, SparcliError};
use crate::input::Outcome;
use crate::input::event::{
    CrosstermSource, EventSource, InputEvent, KeyCode, KeyPress,
};
use crate::input::guard::TerminalGuard;
use crate::input::prompt::{Flow, run_prompt};
use crate::input::shortcut::{self, Shortcut};

/// Default number of visible rows.
const DEFAULT_VISIBLE: usize = 10;

/// Mutable state of a running select prompt.
struct State {
    cursor: usize,
    checked: Vec<bool>,
    offset: usize,
    help: bool,
}

/// A scrollable selection list (single or multi).
pub struct Select {
    prompt: String,
    options: Vec<String>,
    multi: bool,
    max_visible: usize,
    cycle: bool,
    shortcuts: Vec<Shortcut>,
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
        }
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
    /// Returns [`SparcliError::NoTerminal`] without an interactive terminal,
    /// or [`SparcliError::Io`] on a terminal failure.
    pub fn run_multi(self) -> Result<Outcome<Vec<usize>>> {
        self.run_collect()
    }

    /// Sets up the terminal and runs the loop.
    fn run_collect(self) -> Result<Outcome<Vec<usize>>> {
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
    ) -> Result<Outcome<Vec<usize>>> {
        if self.options.is_empty() {
            return Ok(Outcome::Submitted(Vec::new()));
        }
        let mut state = State {
            cursor: 0,
            checked: vec![false; self.options.len()],
            offset: 0,
            help: false,
        };
        run_prompt(
            source,
            &mut state,
            |state, _| self.render(state),
            |state, event| self.handle(state, event),
        )
    }

    /// Builds the prompt frame with the visible window of options.
    fn render(&self, state: &State) -> Rendered {
        let theme = theme();
        if state.help {
            return Rendered::new(shortcut::help_overlay(&self.shortcuts));
        }
        let mut lines = vec![Line::styled(self.prompt.clone(), theme.title)];
        let end = (state.offset + self.max_visible).min(self.options.len());
        for index in state.offset..end {
            lines.push(self.option_line(state, index, &theme));
        }
        if !self.shortcuts.is_empty() {
            lines.push(shortcut::hint_line(&self.shortcuts));
        }
        Rendered::new(lines)
    }

    /// Renders one option row.
    fn option_line(&self, state: &State, index: usize, theme: &Theme) -> Line {
        let is_cursor = index == state.cursor;
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
        if state.help {
            state.help = false;
            return Flow::Continue;
        }
        if key.code == KeyCode::Char('?') && !self.shortcuts.is_empty() {
            state.help = true;
            return Flow::Continue;
        }
        if let Some(id) = shortcut::find(key, &self.shortcuts) {
            return Flow::Shortcut(id);
        }
        match key.code {
            KeyCode::Esc => return Flow::Cancel,
            KeyCode::Enter => return Flow::Submit(self.collect(state)),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(state, -1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(state, 1),
            KeyCode::Home => self.set_cursor(state, 0),
            KeyCode::End => self.set_cursor(state, self.options.len() - 1),
            KeyCode::PageUp => {
                self.move_cursor(state, -(self.max_visible as isize));
            }
            KeyCode::PageDown => {
                self.move_cursor(state, self.max_visible as isize);
            }
            KeyCode::Char(' ') if self.multi => {
                state.checked[state.cursor] = !state.checked[state.cursor];
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
            vec![state.cursor]
        }
    }

    /// Moves the cursor by `delta`, cycling or clamping per config.
    fn move_cursor(&self, state: &mut State, delta: isize) {
        let len = self.options.len() as isize;
        let next = if self.cycle {
            (state.cursor as isize + delta).rem_euclid(len)
        } else {
            (state.cursor as isize + delta).clamp(0, len - 1)
        };
        self.set_cursor(state, next as usize);
    }

    /// Sets the cursor and scrolls so it stays visible.
    fn set_cursor(&self, state: &mut State, index: usize) {
        state.cursor = index;
        if index < state.offset {
            state.offset = index;
        } else if index >= state.offset + self.max_visible {
            state.offset = index + 1 - self.max_visible;
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
