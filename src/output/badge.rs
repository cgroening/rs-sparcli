//! Inline status badges such as `[TAG]` or `(v1.0)`.

use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::{Line, Span};
use crate::core::theme::theme;

/// A small inline token with configurable caps and style.
pub struct Badge {
    text: String,
    left_cap: String,
    right_cap: String,
    style: Style,
    pad: u16,
}

impl Badge {
    /// Creates a badge with default square brackets and the accent style.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            left_cap: "[".to_string(),
            right_cap: "]".to_string(),
            style: Style::new().fg(theme().accent).bold(),
            pad: 0,
        }
    }

    /// Sets both caps at once.
    #[must_use]
    pub fn caps(
        mut self,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        self.left_cap = left.into();
        self.right_cap = right.into();
        self
    }

    /// Sets the badge text style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the number of spaces inside the caps.
    #[must_use]
    pub fn pad(mut self, pad: u16) -> Self {
        self.pad = pad;
        self
    }

    /// Returns the badge as a single styled [`Span`].
    pub fn span(&self) -> Span {
        let spaces = " ".repeat(self.pad as usize);
        let content = format!(
            "{}{spaces}{}{spaces}{}",
            self.left_cap, self.text, self.right_cap
        );
        Span::styled(content, self.style)
    }
}

impl Renderable for Badge {
    fn render(&self, _max_width: u16) -> Rendered {
        Rendered::new(vec![Line::new(vec![self.span()])])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_wraps_text_in_caps() {
        assert_eq!(Badge::new("OK").span().content, "[OK]");
    }

    #[test]
    fn badge_honors_custom_caps_and_pad() {
        let span = Badge::new("v1").caps("(", ")").pad(1).span();
        assert_eq!(span.content, "( v1 )");
    }
}
