//! Numeric input with bounds, step adjustment and a calculator mode.

mod calc;

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::theme::theme;
use crate::error::Result;
use crate::input::Outcome;
use crate::input::event::{EventSource, InputEvent, KeyPress};
use crate::input::field::{error_line, field_line, value_line};
use crate::input::line_edit::LineEditor;
use crate::input::number::calc::{CalcError, eval, parse_number};
use crate::input::prompt::{Flow, run_interactive, run_prompt};

/// Mutable state of a running number prompt.
struct State {
    editor: LineEditor,
    error: Option<String>,
}

/// A numeric input prompt with optional calculator expressions.
///
/// # Examples
///
/// ```no_run
/// use sparcli::{NumberInput, Outcome};
///
/// # fn main() -> sparcli::Result<()> {
/// if let Outcome::Submitted(count) =
///     NumberInput::new("How many?").range(0.0, 100.0).run()?
/// {
///     println!("count = {count}");
/// }
/// # Ok(())
/// # }
/// ```
pub struct NumberInput {
    prompt: String,
    initial: f64,
    min: f64,
    max: f64,
    step: f64,
    decimals: usize,
    calculator: bool,
}

impl NumberInput {
    /// Creates a number prompt with the given label.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            initial: 0.0,
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            step: 1.0,
            decimals: 0,
            calculator: false,
        }
    }

    /// Sets the initial value.
    #[must_use]
    pub fn initial(mut self, value: f64) -> Self {
        self.initial = value;
        self
    }

    /// Sets the inclusive `[min, max]` bounds.
    #[must_use]
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Sets the step used by Up/Down.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Sets the number of decimal places shown by step adjustments.
    #[must_use]
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Enables calculator expressions (`+ - * / ( )`).
    #[must_use]
    pub fn calculator(mut self) -> Self {
        self.calculator = true;
        self
    }

    /// Runs the prompt on the real terminal.
    ///
    /// # Errors
    ///
    /// Returns [`SparcliError::NoTerminal`] without an interactive terminal,
    /// or [`SparcliError::Io`] on a terminal failure.
    pub fn run(self) -> Result<Outcome<f64>> {
        run_interactive(|source| self.run_with(source))
    }

    /// Runs the prompt against any event source (used for tests).
    fn run_with(&self, source: &mut impl EventSource) -> Result<Outcome<f64>> {
        let mut state = State {
            editor: LineEditor::new(&self.format(self.initial), false),
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
            editor: LineEditor::new(&self.format(self.initial), false),
            error: None,
        };
        self.render(&state, false)
    }

    /// Builds the prompt frame.
    fn render(&self, state: &State, final_frame: bool) -> Rendered {
        let theme = theme();
        let value = state.editor.value();
        if final_frame {
            let line = value_line(&self.prompt, &value, Style::new(), &theme);
            return Rendered::new(vec![line]);
        }
        let mut lines = vec![field_line(
            &self.prompt,
            &value,
            state.editor.cursor(),
            Style::new(),
            &theme,
        )];
        if let Some(error) = &state.error {
            lines.push(error_line(error, &theme));
        }
        Rendered::new(lines)
    }

    /// Handles one event.
    fn handle(&self, state: &mut State, event: InputEvent) -> Flow<f64> {
        let InputEvent::Key(key) = event else {
            return Flow::Continue;
        };
        self.handle_key(state, key)
    }

    /// Handles a single key press.
    fn handle_key(&self, state: &mut State, key: KeyPress) -> Flow<f64> {
        use crate::input::event::KeyCode::{
            Backspace, Char, Delete, Down, Enter, Esc, Left, Right, Up,
        };
        if key.ctrl && key.code == Char('u') {
            state.editor.kill_to_line_start();
            return Flow::Continue;
        }
        match key.code {
            Esc => return Flow::Cancel,
            Enter => return self.submit(state),
            Up => self.adjust(state, self.step),
            Down => self.adjust(state, -self.step),
            Left => state.editor.move_left(false),
            Right => state.editor.move_right(false),
            Backspace => state.editor.backspace(),
            Delete => state.editor.delete(),
            Char(c) if self.accepts(c) => {
                state.editor.insert_char(c);
                state.error = None;
            }
            _ => {}
        }
        Flow::Continue
    }

    /// Returns whether `ch` is a valid input character.
    fn accepts(&self, ch: char) -> bool {
        let numeric = ch.is_ascii_digit() || matches!(ch, '.' | ',' | '-');
        let calc = matches!(ch, '+' | '*' | '/' | '(' | ')' | ' ');
        numeric || (self.calculator && calc)
    }

    /// Adjusts the current value by `delta` and reformats the field.
    fn adjust(&self, state: &mut State, delta: f64) {
        let current = parse_number(&state.editor.value()).unwrap_or(0.0);
        let value = self.clamp(current + delta);
        state.editor.set_value(&self.format(value));
        state.error = None;
    }

    /// Evaluates the field and submits the clamped value.
    fn submit(&self, state: &mut State) -> Flow<f64> {
        let text = state.editor.value();
        let parsed = if self.calculator {
            eval(&text)
        } else {
            parse_number(&text).ok_or(CalcError::NotANumber)
        };
        match parsed {
            Ok(value) => Flow::Submit(self.clamp(value)),
            Err(error) => {
                state.error = Some(error.to_string());
                Flow::Continue
            }
        }
    }

    /// Clamps `value` to the configured range.
    fn clamp(&self, value: f64) -> f64 {
        value.clamp(self.min, self.max)
    }

    /// Formats `value` with the configured number of decimals.
    fn format(&self, value: f64) -> String {
        format!("{value:.*}", self.decimals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::event::{KeyCode, ScriptedSource};

    #[test]
    fn types_and_submits_number() {
        let outcome = NumberInput::new("n")
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Char('4'),
                KeyCode::Char('2'),
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(42.0));
    }

    #[test]
    fn up_arrow_adds_step() {
        let outcome = NumberInput::new("n")
            .initial(5.0)
            .step(2.0)
            .run_with(&mut ScriptedSource::keys([KeyCode::Up, KeyCode::Enter]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(7.0));
    }

    #[test]
    fn value_is_clamped_to_range() {
        let outcome = NumberInput::new("n")
            .range(0.0, 10.0)
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Char('9'),
                KeyCode::Char('9'),
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(10.0));
    }

    #[test]
    fn calculator_evaluates_expression_on_submit() {
        let outcome = NumberInput::new("n")
            .calculator()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Char('2'),
                KeyCode::Char('+'),
                KeyCode::Char('3'),
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(5.0));
    }

    #[test]
    fn esc_cancels() {
        let outcome = NumberInput::new("n")
            .run_with(&mut ScriptedSource::keys([KeyCode::Esc]))
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
    }
}
