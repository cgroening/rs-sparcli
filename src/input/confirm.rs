//! Yes/no confirmation prompt.

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::terminal::is_input_tty;
use crate::core::text::{Line, Span};
use crate::core::theme::{Theme, theme};
use crate::error::{Result, SparcliError};
use crate::input::Outcome;
use crate::input::event::{CrosstermSource, EventSource, InputEvent, KeyCode};
use crate::input::guard::TerminalGuard;
use crate::input::prompt::{Flow, run_prompt};

/// A yes/no confirmation prompt.
pub struct Confirm {
    question: String,
    default_yes: bool,
    yes_label: String,
    no_label: String,
}

impl Confirm {
    /// Creates a confirmation prompt (defaults to "No").
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            default_yes: false,
            yes_label: "Yes".to_string(),
            no_label: "No".to_string(),
        }
    }

    /// Sets the initial selection to "Yes".
    #[must_use]
    pub fn default_yes(mut self) -> Self {
        self.default_yes = true;
        self
    }

    /// Sets custom labels for the two options.
    #[must_use]
    pub fn labels(
        mut self,
        yes: impl Into<String>,
        no: impl Into<String>,
    ) -> Self {
        self.yes_label = yes.into();
        self.no_label = no.into();
        self
    }

    /// Runs the prompt on the real terminal.
    ///
    /// # Errors
    /// Returns [`SparcliError::NoTerminal`] without an interactive terminal,
    /// or [`SparcliError::Io`] on a terminal failure.
    pub fn run(self) -> Result<Outcome<bool>> {
        if !is_input_tty() {
            return Err(SparcliError::NoTerminal);
        }
        let _guard = TerminalGuard::new()?;
        let mut source = CrosstermSource;
        self.run_with(&mut source)
    }

    /// Runs the prompt against any event source (used for tests).
    fn run_with(&self, source: &mut impl EventSource) -> Result<Outcome<bool>> {
        let mut state = self.default_yes;
        run_prompt(source, &mut state, |yes| self.render(*yes), handle)
    }

    /// Builds the prompt frame for the given selection.
    fn render(&self, yes_selected: bool) -> Rendered {
        let theme = theme();
        let mut spans = vec![Span::raw(format!("{} ", self.question))];
        spans.push(option_span(&self.yes_label, yes_selected, &theme));
        spans.push(Span::raw(" "));
        spans.push(option_span(&self.no_label, !yes_selected, &theme));
        Rendered::new(vec![Line::new(spans)])
    }
}

/// Renders one option, highlighted when selected.
fn option_span(label: &str, selected: bool, theme: &Theme) -> Span {
    if selected {
        Span::styled(format!("[{label}]"), theme.selection)
    } else {
        Span::styled(format!(" {label} "), Style::new().dim())
    }
}

/// Handles one event for the confirm prompt.
fn handle(yes: &mut bool, event: InputEvent) -> Flow<bool> {
    let InputEvent::Key(key) = event else {
        return Flow::Continue;
    };
    if key.is_ctrl('c') {
        return Flow::Cancel;
    }
    match key.code {
        KeyCode::Esc => Flow::Cancel,
        KeyCode::Enter => Flow::Submit(*yes),
        KeyCode::Char('y') | KeyCode::Char('Y') => Flow::Submit(true),
        KeyCode::Char('n') | KeyCode::Char('N') => Flow::Submit(false),
        KeyCode::Left
        | KeyCode::Right
        | KeyCode::Tab
        | KeyCode::Char('h')
        | KeyCode::Char('l') => {
            *yes = !*yes;
            Flow::Continue
        }
        _ => Flow::Continue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::event::ScriptedSource;

    #[test]
    fn y_key_submits_true() {
        let outcome = Confirm::new("ok?")
            .run_with(&mut ScriptedSource::keys([KeyCode::Char('y')]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(true));
    }

    #[test]
    fn enter_uses_default() {
        let outcome = Confirm::new("ok?")
            .default_yes()
            .run_with(&mut ScriptedSource::keys([KeyCode::Enter]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(true));
    }

    #[test]
    fn arrow_toggles_selection() {
        let outcome = Confirm::new("ok?")
            .default_yes()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Left,
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(false));
    }

    #[test]
    fn esc_cancels() {
        let outcome = Confirm::new("ok?")
            .run_with(&mut ScriptedSource::keys([KeyCode::Esc]))
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
    }
}
