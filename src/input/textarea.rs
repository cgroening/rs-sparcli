//! Multi-line text input (Enter inserts a newline, Ctrl-D submits).

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::text::Line;
use crate::core::theme::theme;
use crate::error::Result;
use crate::input::Outcome;
use crate::input::editor::edit_text_suspended;
use crate::input::event::{EventSource, InputEvent, KeyPress};
use crate::input::field::field_line;
use crate::input::line_edit::LineEditor;
use crate::input::prompt::{Flow, run_interactive, run_prompt};

/// A multi-line text input prompt.
///
/// # Examples
///
/// ```no_run
/// use sparcli::{Outcome, Textarea};
///
/// # fn main() -> sparcli::Result<()> {
/// if let Outcome::Submitted(notes) = Textarea::new("Notes:").run()? {
///     println!("{notes}");
/// }
/// # Ok(())
/// # }
/// ```
pub struct Textarea {
    prompt: String,
    initial: String,
    editor_enabled: bool,
    editor_command: Option<String>,
}

impl Textarea {
    /// Creates a multi-line prompt with the given label.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            initial: String::new(),
            editor_enabled: false,
            editor_command: None,
        }
    }

    /// Sets the initial multi-line value.
    #[must_use]
    pub fn initial(mut self, value: impl Into<String>) -> Self {
        self.initial = value.into();
        self
    }

    /// Enables opening the buffer in `$EDITOR` with Ctrl-G.
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
        run_interactive(|source| self.run_with(source))
    }

    /// Runs the prompt against any event source (used for tests).
    fn run_with(
        &self,
        source: &mut impl EventSource,
    ) -> Result<Outcome<String>> {
        let mut editor = LineEditor::new(&self.initial, true);
        run_prompt(
            source,
            &mut editor,
            |editor, final_frame| self.render(editor, final_frame),
            |editor, event| self.handle(editor, event),
        )
    }

    /// Renders the prompt's static frame without running it (for previews
    /// and README screenshots).
    pub fn frame(&self) -> Rendered {
        let editor = LineEditor::new(&self.initial, true);
        self.render(&editor, false)
    }

    /// Builds the prompt frame, drawing the cursor on its line.
    ///
    /// The final frame omits the cursor.
    fn render(&self, editor: &LineEditor, final_frame: bool) -> Rendered {
        let theme = theme();
        let (cursor_line, cursor_col) = editor.cursor_line_col();
        let mut lines = vec![Line::styled(self.prompt.clone(), theme.title)];
        for (index, text) in editor.lines().into_iter().enumerate() {
            if index == cursor_line && !final_frame {
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

impl Textarea {
    /// Handles one event for the textarea.
    fn handle(
        &self,
        editor: &mut LineEditor,
        event: InputEvent,
    ) -> Flow<String> {
        match event {
            InputEvent::Paste(text) => {
                editor.insert_str(&text);
                Flow::Continue
            }
            InputEvent::Key(key) => self.handle_key(editor, key),
            InputEvent::Resize => Flow::Continue,
        }
    }

    /// Handles a single key press.
    fn handle_key(
        &self,
        editor: &mut LineEditor,
        key: KeyPress,
    ) -> Flow<String> {
        use crate::input::event::KeyCode::{
            Backspace, Char, Delete, Down, End, Enter, Esc, Home, Left, Right,
            Up,
        };
        if key.ctrl {
            return self.handle_ctrl(editor, key);
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

    /// Handles Ctrl-modified keys (Ctrl-D submits, Ctrl-G opens the editor).
    fn handle_ctrl(
        &self,
        editor: &mut LineEditor,
        key: KeyPress,
    ) -> Flow<String> {
        use crate::input::event::KeyCode::Char;
        if let Char(c) = key.code {
            match c {
                'd' => return Flow::Submit(editor.value()),
                'g' if self.editor_enabled => {
                    return self.launch_editor(editor);
                }
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

    /// Opens the buffer in an external editor, then refreshes the prompt.
    fn launch_editor(&self, editor: &mut LineEditor) -> Flow<String> {
        let command = self.editor_command.as_deref();
        let result = edit_text_suspended(command, &editor.value(), ".md");
        if let Ok(text) = result {
            editor.set_value(text.trim_end_matches('\n'));
        }
        Flow::Refresh
    }
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
