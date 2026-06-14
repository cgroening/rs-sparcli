//! Shared text-editing core for single- and multi-line input widgets.
//!
//! This is the single source of truth for caret movement, selection and edit
//! operations. Widgets own control keys (Enter, Esc, …); the editor only
//! handles editing. The clipboard is in-process (no system dependency); the
//! terminal's bracketed paste still delivers external text via [`insert_str`].
//!
//! [`insert_str`]: LineEditor::insert_str

/// A caret-and-selection text editor over a character buffer.
#[derive(Debug, Clone, Default)]
pub struct LineEditor {
    chars: Vec<char>,
    cursor: usize,
    anchor: Option<usize>,
    clipboard: String,
    multiline: bool,
}

impl LineEditor {
    /// Creates an editor seeded with `initial` text.
    pub fn new(initial: &str, multiline: bool) -> Self {
        let chars: Vec<char> = initial.chars().collect();
        let cursor = chars.len();
        Self {
            chars,
            cursor,
            anchor: None,
            clipboard: String::new(),
            multiline,
        }
    }

    /// Returns the current text.
    pub fn value(&self) -> String {
        self.chars.iter().collect()
    }

    /// Replaces the entire buffer and moves the caret to the end.
    pub fn set_value(&mut self, value: &str) {
        self.chars = value.chars().collect();
        self.cursor = self.chars.len();
        self.anchor = None;
    }

    /// Returns the number of characters.
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Returns `true` if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Returns the caret position as a character index.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the display lines (split on newlines).
    pub fn lines(&self) -> Vec<String> {
        self.value().split('\n').map(str::to_string).collect()
    }

    /// Returns the caret's `(line, column)` in characters.
    pub fn cursor_line_col(&self) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        for &ch in &self.chars[..self.cursor] {
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// Returns `true` if a non-empty selection exists.
    pub fn has_selection(&self) -> bool {
        matches!(self.anchor, Some(anchor) if anchor != self.cursor)
    }

    /// Returns the selection range as `(start, end)` if any.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?;
        if anchor == self.cursor {
            return None;
        }
        Some((anchor.min(self.cursor), anchor.max(self.cursor)))
    }

    /// Inserts a character, replacing any selection.
    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.chars.insert(self.cursor, ch);
        self.cursor += 1;
    }

    /// Inserts a string (e.g. a paste), replacing any selection.
    ///
    /// In single-line mode newlines are converted to spaces.
    pub fn insert_str(&mut self, text: &str) {
        self.delete_selection();
        for ch in text.chars() {
            let ch = if !self.multiline && ch == '\n' {
                ' '
            } else {
                ch
            };
            self.chars.insert(self.cursor, ch);
            self.cursor += 1;
        }
    }

    /// Inserts a newline (multi-line only; ignored otherwise).
    pub fn insert_newline(&mut self) {
        if self.multiline {
            self.insert_char('\n');
        }
    }

    /// Deletes the character before the caret, or the selection.
    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

    /// Deletes the character at the caret, or the selection.
    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor < self.chars.len() {
            self.chars.remove(self.cursor);
        }
    }

    /// Moves the caret left by one character.
    pub fn move_left(&mut self, select: bool) {
        self.update_anchor(select);
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Moves the caret right by one character.
    pub fn move_right(&mut self, select: bool) {
        self.update_anchor(select);
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    /// Moves the caret to the start of the current line.
    pub fn move_home(&mut self, select: bool) {
        self.update_anchor(select);
        self.cursor = self.line_start(self.cursor);
    }

    /// Moves the caret to the end of the current line.
    pub fn move_end(&mut self, select: bool) {
        self.update_anchor(select);
        self.cursor = self.line_end(self.cursor);
    }

    /// Moves the caret up one line, preserving the column (multi-line).
    pub fn move_up(&mut self, select: bool) {
        self.update_anchor(select);
        let start = self.line_start(self.cursor);
        if start == 0 {
            return;
        }
        let col = self.cursor - start;
        let prev_start = self.line_start(start - 1);
        let prev_end = start - 1;
        self.cursor = (prev_start + col).min(prev_end);
    }

    /// Moves the caret down one line, preserving the column (multi-line).
    pub fn move_down(&mut self, select: bool) {
        self.update_anchor(select);
        let end = self.line_end(self.cursor);
        if end >= self.chars.len() {
            return;
        }
        let col = self.cursor - self.line_start(self.cursor);
        let next_start = end + 1;
        let next_end = self.line_end(next_start);
        self.cursor = (next_start + col).min(next_end);
    }

    /// Selects the entire buffer.
    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.cursor = self.chars.len();
    }

    /// Deletes the previous whitespace-delimited word.
    pub fn delete_word_back(&mut self) {
        if self.delete_selection() {
            return;
        }
        let mut index = self.cursor;
        while index > 0 && self.chars[index - 1].is_whitespace() {
            index -= 1;
        }
        while index > 0 && !self.chars[index - 1].is_whitespace() {
            index -= 1;
        }
        self.chars.drain(index..self.cursor);
        self.cursor = index;
    }

    /// Deletes from the caret to the start of the current line.
    pub fn kill_to_line_start(&mut self) {
        let start = self.line_start(self.cursor);
        self.chars.drain(start..self.cursor);
        self.cursor = start;
    }

    /// Deletes from the caret to the end of the current line.
    pub fn kill_to_line_end(&mut self) {
        let end = self.line_end(self.cursor);
        self.chars.drain(self.cursor..end);
    }

    /// Copies the selection to the in-process clipboard.
    pub fn copy(&mut self) {
        if let Some((start, end)) = self.selection_range() {
            self.clipboard = self.chars[start..end].iter().collect();
        }
    }

    /// Cuts the selection to the in-process clipboard.
    pub fn cut(&mut self) {
        self.copy();
        self.delete_selection();
    }

    /// Pastes the in-process clipboard at the caret.
    pub fn paste(&mut self) {
        let text = std::mem::take(&mut self.clipboard);
        self.insert_str(&text);
        self.clipboard = text;
    }

    /// Sets or clears the selection anchor before a movement.
    fn update_anchor(&mut self, select: bool) {
        if select {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
    }

    /// Deletes the current selection; returns whether anything was removed.
    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection_range() else {
            self.anchor = None;
            return false;
        };
        self.chars.drain(start..end);
        self.cursor = start;
        self.anchor = None;
        true
    }

    /// Returns the index of the start of the line containing `index`.
    fn line_start(&self, index: usize) -> usize {
        self.chars[..index]
            .iter()
            .rposition(|&c| c == '\n')
            .map_or(0, |pos| pos + 1)
    }

    /// Returns the index of the end of the line containing `index`.
    fn line_end(&self, index: usize) -> usize {
        self.chars[index..]
            .iter()
            .position(|&c| c == '\n')
            .map_or(self.chars.len(), |offset| index + offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_and_reports_value() {
        let mut editor = LineEditor::new("", false);
        editor.insert_char('h');
        editor.insert_char('i');
        assert_eq!(editor.value(), "hi");
        assert_eq!(editor.cursor(), 2);
    }

    #[test]
    fn backspace_removes_previous_char() {
        let mut editor = LineEditor::new("ab", false);
        editor.backspace();
        assert_eq!(editor.value(), "a");
    }

    #[test]
    fn selection_is_replaced_on_insert() {
        let mut editor = LineEditor::new("hello", false);
        editor.move_home(false);
        editor.move_right(true);
        editor.move_right(true);
        editor.insert_char('X');
        assert_eq!(editor.value(), "Xllo");
    }

    #[test]
    fn delete_word_back_removes_word() {
        let mut editor = LineEditor::new("foo bar", false);
        editor.delete_word_back();
        assert_eq!(editor.value(), "foo ");
    }

    #[test]
    fn kill_to_line_start_and_end() {
        let mut editor = LineEditor::new("hello", false);
        editor.move_home(false);
        editor.move_right(false);
        editor.move_right(false);
        editor.kill_to_line_end();
        assert_eq!(editor.value(), "he");
        editor.kill_to_line_start();
        assert_eq!(editor.value(), "");
    }

    #[test]
    fn single_line_converts_pasted_newlines() {
        let mut editor = LineEditor::new("", false);
        editor.insert_str("a\nb");
        assert_eq!(editor.value(), "a b");
    }

    #[test]
    fn multiline_navigates_between_rows() {
        let mut editor = LineEditor::new("ab\ncd", true);
        editor.move_home(false);
        editor.move_up(false);
        assert_eq!(editor.cursor_line_col(), (0, 0));
    }

    #[test]
    fn cut_and_paste_round_trip() {
        let mut editor = LineEditor::new("hello", false);
        editor.select_all();
        editor.cut();
        assert_eq!(editor.value(), "");
        editor.paste();
        assert_eq!(editor.value(), "hello");
    }
}
