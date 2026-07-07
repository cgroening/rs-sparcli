//! Event and key handling for the text prompt.

use crate::input::editor;
use crate::input::event::{InputEvent, KeyPress};
use crate::input::prompt::Flow;
use crate::input::text::{MAX_DROPDOWN, State, TextInput};

impl TextInput {
    /// Handles one event.
    pub(super) fn handle(
        &self,
        state: &mut State,
        event: InputEvent,
    ) -> Flow<String> {
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
    ///
    /// Raw mode is only toggled when it was already enabled (i.e. a real
    /// interactive run), so headless callers never alter the terminal.
    fn launch_editor(&self, state: &mut State) -> Flow<String> {
        let was_raw =
            crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
        if was_raw {
            let _ = crossterm::terminal::disable_raw_mode();
        }
        let command = self.editor_command.as_deref();
        let result = editor::edit_text(command, &state.editor.value(), ".txt");
        if was_raw {
            let _ = crossterm::terminal::enable_raw_mode();
        }
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
