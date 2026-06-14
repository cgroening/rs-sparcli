//! Animated spinners for in-progress operations.

use crate::core::render::Rendered;
use crate::core::style::{Color, Style};
use crate::core::text::{Line, Span};
use crate::core::theme::theme;
use crate::error::Result;
use crate::output::live::InPlace;

/// The animation style of a spinner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerStyle {
    /// Braille dots.
    #[default]
    Braille,
    /// ASCII pipe (`|/-\`).
    Pipe,
    /// Heavy braille dots.
    Dots,
    /// Rotating arrows.
    Arrow,
}

impl SpinnerStyle {
    /// Returns the animation frames for this style.
    fn frames(self) -> &'static [char] {
        match self {
            Self::Braille => {
                &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
            }
            Self::Pipe => &['|', '/', '-', '\\'],
            Self::Dots => &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'],
            Self::Arrow => &['←', '↖', '↑', '↗', '→', '↘', '↓', '↙'],
        }
    }
}

/// An animated, single-line spinner with a label.
pub struct Spinner {
    style: SpinnerStyle,
    color: Color,
    label: String,
    label_style: Style,
    frame_index: usize,
    inplace: Option<InPlace>,
}

impl Spinner {
    /// Creates a spinner with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        let theme = theme();
        Self {
            style: SpinnerStyle::Braille,
            color: theme.accent,
            label: label.into(),
            label_style: Style::new(),
            frame_index: 0,
            inplace: None,
        }
    }

    /// Sets the spinner style.
    #[must_use]
    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the spinner color.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Updates the label.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    /// Builds the current frame as a rendered line.
    pub fn frame(&self) -> Rendered {
        let frames = self.style.frames();
        let glyph = frames[self.frame_index % frames.len()];
        self.compose(glyph.to_string(), Style::new().fg(self.color))
    }

    /// Advances to the next frame and redraws it in place.
    ///
    /// # Errors
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn tick(&mut self) -> Result<()> {
        let frame = self.frame();
        self.inplace
            .get_or_insert_with(|| InPlace::new(false))
            .draw(&frame)?;
        self.frame_index = self.frame_index.wrapping_add(1);
        Ok(())
    }

    /// Stops the spinner with a success or failure marker.
    ///
    /// # Errors
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn finish(
        mut self,
        success: bool,
        label: impl Into<String>,
    ) -> Result<()> {
        let theme = theme();
        let (glyph, style) = if success {
            (success_glyph(theme.unicode), theme.success)
        } else {
            (failure_glyph(theme.unicode), theme.error)
        };
        self.label = label.into();
        let frame = self.compose(glyph.to_string(), style);
        let mut inplace =
            self.inplace.take().unwrap_or_else(|| InPlace::new(false));
        inplace.draw(&frame)?;
        inplace.finish()
    }

    /// Composes a marker glyph and the label into one line.
    fn compose(&self, glyph: String, glyph_style: Style) -> Rendered {
        let mut spans = vec![Span::styled(glyph, glyph_style)];
        if !self.label.is_empty() {
            spans.push(Span::styled(
                format!(" {}", self.label),
                self.label_style,
            ));
        }
        Rendered::new(vec![Line::new(spans)])
    }
}

/// The success marker glyph for the given glyph mode.
fn success_glyph(unicode: bool) -> char {
    if unicode { '✔' } else { '+' }
}

/// The failure marker glyph for the given glyph mode.
fn failure_glyph(unicode: bool) -> char {
    if unicode { '✖' } else { 'x' }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_contains_glyph_and_label() {
        let spinner = Spinner::new("loading");
        let line = spinner.frame().lines[0].plain();
        assert!(line.contains('⠋'));
        assert!(line.contains("loading"));
    }

    #[test]
    fn pipe_style_uses_ascii_frames() {
        let spinner = Spinner::new("").style(SpinnerStyle::Pipe);
        assert_eq!(spinner.frame().lines[0].plain(), "|");
    }
}
