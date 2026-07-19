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
    /// A single-line prompt cannot hold newlines, so a multi-line edit is
    /// flattened back into one line.
    fn launch_editor(&self, state: &mut State) -> Flow<String> {
        let command = self.editor_command.as_deref();
        let result =
            editor::edit_text_suspended(command, &state.editor.value(), ".txt");
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
            if let Err(error) = store.save() {
                log::warn!("could not save input history: {error}");
            }
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

#[cfg(test)]
mod tests {
    use crate::input::Outcome;
    use crate::input::event::{KeyCode, ScriptedSource};
    use crate::input::text::TextInput;

    /// Drives `input` through `keys` and returns the prompt's outcome.
    fn run(input: &TextInput, keys: Vec<KeyCode>) -> Outcome<String> {
        input
            .run_with(&mut ScriptedSource::keys(keys))
            .expect("a scripted source never fails")
    }

    /// A prompt with three suggestions and a dropdown.
    fn with_dropdown() -> TextInput {
        TextInput::new("pick")
            .suggestions(["alpha", "alpine", "beta"])
            .dropdown()
    }

    #[test]
    fn arrows_walk_backwards_and_forwards_through_history() {
        let input = TextInput::new("cmd").history(["first", "second"]);
        // Up starts at the newest entry, a second Up reaches the older one.
        let outcome = run(&input, vec![KeyCode::Up, KeyCode::Enter]);
        assert_eq!(outcome, Outcome::Submitted("second".to_string()));
        let outcome =
            run(&input, vec![KeyCode::Up, KeyCode::Up, KeyCode::Enter]);
        assert_eq!(outcome, Outcome::Submitted("first".to_string()));
    }

    #[test]
    fn history_stops_at_the_oldest_entry() {
        let input = TextInput::new("cmd").history(["only"]);
        let outcome = run(
            &input,
            vec![KeyCode::Up, KeyCode::Up, KeyCode::Up, KeyCode::Enter],
        );
        assert_eq!(outcome, Outcome::Submitted("only".to_string()));
    }

    #[test]
    fn walking_past_the_newest_history_entry_clears_the_field() {
        // Down past the newest entry returns to the empty line the user was
        // typing on, rather than sticking on the last recalled command.
        let input = TextInput::new("cmd").history(["first", "second"]);
        let outcome =
            run(&input, vec![KeyCode::Up, KeyCode::Down, KeyCode::Enter]);
        assert_eq!(outcome, Outcome::Submitted(String::new()));
    }

    #[test]
    fn down_without_a_recalled_entry_does_nothing() {
        let input = TextInput::new("cmd").history(["first"]);
        let outcome = run(&input, vec![KeyCode::Down, KeyCode::Enter]);
        assert_eq!(outcome, Outcome::Submitted(String::new()));
    }

    #[test]
    fn tab_fills_the_field_from_the_highlighted_suggestion() {
        let outcome = run(
            &with_dropdown(),
            vec![KeyCode::Char('a'), KeyCode::Tab, KeyCode::Enter],
        );
        assert_eq!(outcome, Outcome::Submitted("alpha".to_string()));
    }

    #[test]
    fn the_dropdown_highlight_cycles_over_the_matches() {
        // Two options match "al"; stepping down twice wraps back to the first.
        // The first Enter accepts the highlight, the second submits it.
        let keys = |steps: usize| {
            let mut keys = vec![KeyCode::Char('a'), KeyCode::Char('l')];
            keys.extend(std::iter::repeat_n(KeyCode::Down, steps));
            keys.push(KeyCode::Enter);
            keys.push(KeyCode::Enter);
            keys
        };
        assert_eq!(
            run(&with_dropdown(), keys(1)),
            Outcome::Submitted("alpha".to_string()),
            "the first step highlights the first match"
        );
        assert_eq!(
            run(&with_dropdown(), keys(2)),
            Outcome::Submitted("alpine".to_string())
        );
        assert_eq!(
            run(&with_dropdown(), keys(3)),
            Outcome::Submitted("alpha".to_string()),
            "stepping past the last match wraps to the first"
        );
    }

    #[test]
    fn stepping_up_from_no_highlight_starts_at_the_last_match() {
        let outcome = run(
            &with_dropdown(),
            vec![
                KeyCode::Char('a'),
                KeyCode::Char('l'),
                KeyCode::Up,
                KeyCode::Enter,
                KeyCode::Enter,
            ],
        );
        assert_eq!(outcome, Outcome::Submitted("alpine".to_string()));
    }

    #[test]
    fn enter_accepts_the_highlight_before_it_submits() {
        // The first Enter takes the highlighted row, the second submits it.
        let outcome = run(
            &with_dropdown(),
            vec![
                KeyCode::Char('a'),
                KeyCode::Down,
                KeyCode::Enter,
                KeyCode::Enter,
            ],
        );
        assert_eq!(outcome, Outcome::Submitted("alpha".to_string()));
    }

    #[test]
    fn typing_clears_a_stale_dropdown_highlight() {
        // With a highlight active Enter would accept it. Typing must drop it,
        // so Enter submits what was actually typed instead of "alpha".
        let outcome = run(
            &with_dropdown(),
            vec![
                KeyCode::Char('a'),
                KeyCode::Down,
                KeyCode::Char('l'),
                KeyCode::Enter,
            ],
        );
        assert_eq!(outcome, Outcome::Submitted("al".to_string()));
    }

    #[test]
    fn ctrl_u_and_ctrl_k_cut_to_the_line_bounds() {
        use crate::input::event::InputEvent;
        use crate::input::event::KeyPress;

        let events = |chord: KeyPress| {
            vec![
                InputEvent::Key(KeyPress::new(KeyCode::Char('a'))),
                InputEvent::Key(KeyPress::new(KeyCode::Char('b'))),
                InputEvent::Key(KeyPress::new(KeyCode::Left)),
                InputEvent::Key(chord),
                InputEvent::Key(KeyPress::new(KeyCode::Enter)),
            ]
        };
        let input = TextInput::new("x");
        let outcome = input
            .run_with(&mut ScriptedSource::events(events(KeyPress::ctrl('u'))))
            .unwrap();
        assert_eq!(
            outcome,
            Outcome::Submitted("b".to_string()),
            "cut to start"
        );
        let outcome = input
            .run_with(&mut ScriptedSource::events(events(KeyPress::ctrl('k'))))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted("a".to_string()), "cut to end");
    }

    #[test]
    fn esc_cancels_the_prompt() {
        assert_eq!(
            run(&TextInput::new("x"), vec![KeyCode::Esc]),
            Outcome::Cancelled
        );
    }
}
