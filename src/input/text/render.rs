//! Frame rendering for the text prompt: field line, ghost text and dropdown.

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::theme::{Theme, theme};
use crate::input::field::{
    error_line, field_line, placeholder_line, value_line,
};
use crate::input::text::{MAX_DROPDOWN, State, TextInput};

impl TextInput {
    /// Builds the prompt frame.
    pub(super) fn render(&self, state: &State, final_frame: bool) -> Rendered {
        let theme = theme();
        let value = state.editor.value();
        if final_frame {
            let line = value_line(&self.prompt, &value, Style::new(), &theme);
            return Rendered::new(vec![line]);
        }
        let mut lines = Vec::new();
        if value.is_empty() && !self.placeholder.is_empty() {
            lines.push(placeholder_line(
                &self.prompt,
                &self.placeholder,
                &theme,
            ));
        } else {
            let mut line = field_line(
                &self.prompt,
                &value,
                state.editor.cursor(),
                Style::new(),
                &theme,
            );
            if !self.dropdown
                && let Some(ghost) = self.ghost(&value)
            {
                line.spans.push(Span::styled(ghost, theme.secondary));
            }
            lines.push(line);
        }
        if self.max_chars > 0
            && !self.hide_char_count
            && let Some(line) = lines.last_mut()
        {
            let count = format!(" ({}/{})", state.editor.len(), self.max_chars);
            line.spans.push(Span::styled(count, theme.secondary));
        }
        if self.dropdown {
            self.push_dropdown(&mut lines, state, &value, &theme);
        }
        if let Some(error) = &state.error {
            lines.push(error_line(error, &theme));
        }
        Rendered::new(lines)
    }

    /// Appends the dropdown rows for the current matches.
    fn push_dropdown(
        &self,
        lines: &mut Vec<Line>,
        state: &State,
        value: &str,
        theme: &Theme,
    ) {
        let matches = self.matches(value);
        for (row, &index) in matches.iter().take(MAX_DROPDOWN).enumerate() {
            let active = state.dropdown_index == Some(row);
            let marker = if active {
                theme.cursor_marker()
            } else {
                theme.marker()
            };
            let style = if active {
                theme.selection
            } else {
                theme.secondary
            };
            lines.push(Line::new(vec![
                Span::styled(marker.to_string(), theme.selection),
                Span::styled(self.suggestions[index].clone(), style),
            ]));
        }
    }

    /// Returns the ghost completion suffix for `value`, if any.
    pub(super) fn ghost(&self, value: &str) -> Option<String> {
        if value.is_empty() {
            return None;
        }
        self.suggestions
            .iter()
            .find(|s| s.starts_with(value) && s.len() > value.len())
            .map(|s| s[value.len()..].to_string())
    }
}
