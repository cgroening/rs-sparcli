//! Multi-line text input (Enter inserts a newline, Ctrl-D submits).

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::terminal::is_input_tty;
use crate::core::text::Line;
use crate::core::theme::theme;
use crate::error::{Result, SparcliError};
use crate::input::Outcome;
use crate::input::event::{CrosstermSource, EventSource, InputEvent, KeyPress};
use crate::input::field::field_line;
use crate::input::guard::TerminalGuard;
use crate::input::line_edit::LineEditor;
use crate::input::prompt::{Flow, run_prompt};

/// A multi-line text input prompt.
pub struct Textarea {
    prompt: String,
    initial: String,
}

impl Textarea {
    /// Creates a multi-line prompt with the given label.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            initial: String::new(),
        }
    }

    /// Sets the initial multi-line value.
    #[must_use]
    pub fn initial(mut self, value: impl Into<String>) -> Self {
        self.initial = value.into();
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
        let mut editor = LineEditor::new(&self.initial, true);
        run_prompt(source, &mut editor, |editor| self.render(editor), handle)
    }

    /// Builds the prompt frame, drawing the cursor on its line.
    fn render(&self, editor: &LineEditor) -> Rendered {
        let theme = theme();
        let (cursor_line, cursor_col) = editor.cursor_line_col();
        let mut lines = vec![Line::styled(self.prompt.clone(), theme.title)];
        for (index, text) in editor.lines().into_iter().enumerate() {
            if index == cursor_line {
                lines.push(field_line(
                    "",
                    &text,
                    cursor_col,
                    Style::new(),
                    &theme,
                ));
            } else {
                lines.push(Line::raw(text));
            }
        }
        Rendered::new(lines)
    }
}

/// Handles one event for the textarea.
fn handle(editor: &mut LineEditor, event: InputEvent) -> Flow<String> {
    match event {
        InputEvent::Paste(text) => {
            editor.insert_str(&text);
            Flow::Continue
        }
        InputEvent::Key(key) => handle_key(editor, key),
        InputEvent::Resize => Flow::Continue,
    }
}

/// Handles a single key press.
fn handle_key(editor: &mut LineEditor, key: KeyPress) -> Flow<String> {
    use crate::input::event::KeyCode::{
        Backspace, Char, Delete, Down, End, Enter, Esc, Home, Left, Right, Up,
    };
    if key.ctrl {
        return handle_ctrl(editor, key);
    }
    match key.code {
        Esc => return Flow::Cancel,
        Enter => editor.insert_newline(),
        Left => editor.move_left(key.shift),
        Right => editor.move_right(key.shift),
        Up => editor.move_up(key.shift),
        Down => editor.move_down(key.shift),
        Home => editor.move_home(key.shift),
        End => editor.move_end(key.shift),
        Backspace => editor.backspace(),
        Delete => editor.delete(),
        Char(c) => editor.insert_char(c),
        _ => {}
    }
    Flow::Continue
}

/// Handles Ctrl-modified keys (Ctrl-D submits).
fn handle_ctrl(editor: &mut LineEditor, key: KeyPress) -> Flow<String> {
    use crate::input::event::KeyCode::Char;
    if let Char(c) = key.code {
        match c {
            'd' => return Flow::Submit(editor.value()),
            'a' => editor.select_all(),
            'w' => editor.delete_word_back(),
            'u' => editor.kill_to_line_start(),
            'k' => editor.kill_to_line_end(),
            'c' => editor.copy(),
            'x' => editor.cut(),
            'v' => editor.paste(),
            _ => {}
        }
    }
    Flow::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::event::{KeyCode, ScriptedSource};

    #[test]
    fn enter_inserts_newline_and_ctrl_d_submits() {
        let outcome = Textarea::new("notes")
            .run_with(&mut ScriptedSource::events([
                InputEvent::Key(KeyPress::new(KeyCode::Char('a'))),
                InputEvent::Key(KeyPress::new(KeyCode::Enter)),
                InputEvent::Key(KeyPress::new(KeyCode::Char('b'))),
                InputEvent::Key(KeyPress::ctrl('d')),
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted("a\nb".to_string()));
    }

    #[test]
    fn esc_cancels() {
        let outcome = Textarea::new("notes")
            .run_with(&mut ScriptedSource::keys([KeyCode::Esc]))
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
    }
}
