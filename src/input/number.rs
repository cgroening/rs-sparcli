//! Numeric input with bounds, step adjustment and a calculator mode.

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::terminal::is_input_tty;
use crate::core::theme::theme;
use crate::error::{Result, SparcliError};
use crate::input::Outcome;
use crate::input::event::{CrosstermSource, EventSource, InputEvent, KeyPress};
use crate::input::field::{error_line, field_line};
use crate::input::guard::TerminalGuard;
use crate::input::line_edit::LineEditor;
use crate::input::prompt::{Flow, run_prompt};

/// Mutable state of a running number prompt.
struct State {
    editor: LineEditor,
    error: Option<String>,
}

/// A numeric input prompt with optional calculator expressions.
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
    /// Returns [`SparcliError::NoTerminal`] without an interactive terminal,
    /// or [`SparcliError::Io`] on a terminal failure.
    pub fn run(self) -> Result<Outcome<f64>> {
        if !is_input_tty() {
            return Err(SparcliError::NoTerminal);
        }
        let _guard = TerminalGuard::new()?;
        let mut source = CrosstermSource;
        self.run_with(&mut source)
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
            |state| self.render(state),
            |state, event| self.handle(state, event),
        )
    }

    /// Builds the prompt frame.
    fn render(&self, state: &State) -> Rendered {
        let theme = theme();
        let mut lines = vec![field_line(
            &self.prompt,
            &state.editor.value(),
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
            parse_number(&text).ok_or_else(|| "not a number".to_string())
        };
        match parsed {
            Ok(value) => Flow::Submit(self.clamp(value)),
            Err(message) => {
                state.error = Some(message);
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

/// Parses a number, accepting `,` or `.` as the decimal separator.
fn parse_number(text: &str) -> Option<f64> {
    let normalized = text.trim().replace(',', ".");
    if normalized.is_empty() {
        return None;
    }
    normalized.parse().ok()
}

/// Evaluates an arithmetic expression with `+ - * / ( )`.
///
/// Accepts `,` or `.` as the decimal separator. Returns an error message on
/// malformed input or division by zero.
///
/// # Examples
/// ```
/// # use sparcli::input::number::eval;
/// assert_eq!(eval("2 + 3 * 4").unwrap(), 14.0);
/// ```
pub fn eval(expr: &str) -> std::result::Result<f64, String> {
    let normalized = expr.replace(',', ".");
    let mut parser = Calc {
        chars: normalized.chars().collect(),
        pos: 0,
    };
    let value = parser.expression()?;
    parser.skip_spaces();
    if parser.pos != parser.chars.len() {
        return Err("unexpected trailing input".to_string());
    }
    Ok(value)
}

/// A minimal recursive-descent arithmetic parser.
struct Calc {
    chars: Vec<char>,
    pos: usize,
}

impl Calc {
    /// Parses `term (('+'|'-') term)*`.
    fn expression(&mut self) -> std::result::Result<f64, String> {
        let mut value = self.term()?;
        loop {
            self.skip_spaces();
            match self.peek() {
                Some('+') => {
                    self.pos += 1;
                    value += self.term()?;
                }
                Some('-') => {
                    self.pos += 1;
                    value -= self.term()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    /// Parses `factor (('*'|'/') factor)*`.
    fn term(&mut self) -> std::result::Result<f64, String> {
        let mut value = self.factor()?;
        loop {
            self.skip_spaces();
            match self.peek() {
                Some('*') => {
                    self.pos += 1;
                    value *= self.factor()?;
                }
                Some('/') => {
                    self.pos += 1;
                    let divisor = self.factor()?;
                    if divisor == 0.0 {
                        return Err("division by zero".to_string());
                    }
                    value /= divisor;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    /// Parses a number, parenthesized expression or unary minus.
    fn factor(&mut self) -> std::result::Result<f64, String> {
        self.skip_spaces();
        match self.peek() {
            Some('(') => {
                self.pos += 1;
                let value = self.expression()?;
                self.skip_spaces();
                if self.peek() != Some(')') {
                    return Err("missing closing parenthesis".to_string());
                }
                self.pos += 1;
                Ok(value)
            }
            Some('-') => {
                self.pos += 1;
                Ok(-self.factor()?)
            }
            _ => self.number(),
        }
    }

    /// Parses a decimal number literal.
    fn number(&mut self) -> std::result::Result<f64, String> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.') {
            self.pos += 1;
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse().map_err(|_| "invalid number".to_string())
    }

    /// Returns the current character without consuming it.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Skips spaces.
    fn skip_spaces(&mut self) {
        while self.peek() == Some(' ') {
            self.pos += 1;
        }
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
    fn esc_cancels() {
        let outcome = NumberInput::new("n")
            .run_with(&mut ScriptedSource::keys([KeyCode::Esc]))
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
    }

    #[test]
    fn eval_respects_precedence() {
        assert_eq!(eval("2 + 3 * 4").unwrap(), 14.0);
        assert_eq!(eval("(2 + 3) * 4").unwrap(), 20.0);
    }

    #[test]
    fn eval_reports_division_by_zero() {
        assert!(eval("1 / 0").is_err());
    }

    #[test]
    fn eval_accepts_comma_decimal() {
        assert_eq!(eval("1,5 + 1,5").unwrap(), 3.0);
    }
}
