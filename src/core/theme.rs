//! The unified theme shared by output widgets and input prompts.
//!
//! A single [`Theme`] is the single source of truth for the whole look. Set it
//! once via [`set_theme`] (ideally before spawning threads); read the current
//! theme via [`theme`]. Per-call widget options always override the theme.

use std::sync::{OnceLock, RwLock};

use crate::core::border::BorderType;
use crate::core::style::{Color, Style};

/// A muted accent color (soft blue) used by the default theme.
const DEFAULT_ACCENT: Color = Color::Rgb(137, 180, 250);

/// Visual defaults applied across input and output.
#[derive(Debug, Clone)]
pub struct Theme {
    /// The single accent color for highlights, active items and titles.
    pub accent: Color,
    /// Style for titles and headings.
    pub title: Style,
    /// Style for table headers and section headers.
    pub heading: Style,
    /// Style for secondary/auxiliary text (dim).
    pub secondary: Style,
    /// Style for success messages.
    pub success: Style,
    /// Style for error messages.
    pub error: Style,
    /// Style for warning messages.
    pub warning: Style,
    /// Style for informational messages.
    pub info: Style,
    /// Style for debug messages.
    pub debug: Style,
    /// Style for hints and footer key labels.
    pub hint: Style,
    /// Style for the active/selected row in prompts.
    pub selection: Style,
    /// Style for the text cursor block.
    pub cursor: Style,
    /// Default border type for boxes and panels.
    pub border: BorderType,
    /// Whether to use Unicode glyphs (`false` selects ASCII fallbacks).
    pub unicode: bool,
}

impl Default for Theme {
    fn default() -> Self {
        let accent = DEFAULT_ACCENT;
        Self {
            accent,
            title: Style::new().fg(accent).bold(),
            heading: Style::new().fg(accent).bold(),
            secondary: Style::new().dim(),
            success: Style::new().fg(Color::Green),
            error: Style::new().fg(Color::Red),
            warning: Style::new().fg(Color::Yellow),
            info: Style::new().fg(accent),
            debug: Style::new().fg(Color::Magenta),
            hint: Style::new().dim(),
            selection: Style::new().fg(accent).bold(),
            cursor: Style::new().fg(Color::Black).bg(accent),
            border: BorderType::Rounded,
            unicode: true,
        }
    }
}

impl Theme {
    /// The bullet glyph for lists (`•` or `*`).
    pub fn bullet(&self) -> &'static str {
        if self.unicode { "•" } else { "*" }
    }

    /// The cursor row marker for selection prompts (`‣ ` or `> `).
    pub fn cursor_marker(&self) -> &'static str {
        if self.unicode { "‣ " } else { "> " }
    }

    /// The non-cursor row marker (two spaces).
    pub fn marker(&self) -> &'static str {
        "  "
    }

    /// The checked checkbox glyph (`[x]` style independent of unicode).
    pub fn checkbox_on(&self) -> &'static str {
        if self.unicode { "◉ " } else { "[x] " }
    }

    /// The unchecked checkbox glyph.
    pub fn checkbox_off(&self) -> &'static str {
        if self.unicode { "◯ " } else { "[ ] " }
    }
}

/// Returns the process-wide theme storage, initialized on first use.
fn storage() -> &'static RwLock<Theme> {
    static THEME: OnceLock<RwLock<Theme>> = OnceLock::new();
    THEME.get_or_init(|| RwLock::new(Theme::default()))
}

/// Returns a clone of the current theme.
pub fn theme() -> Theme {
    // Recover from a poisoned lock instead of panicking (Style Guide §7.3).
    match storage().read() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

/// Replaces the process-wide theme.
pub fn set_theme(new_theme: Theme) {
    let mut guard = match storage().write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = new_theme;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_uses_rounded_border() {
        assert_eq!(Theme::default().border, BorderType::Rounded);
    }

    #[test]
    fn ascii_mode_changes_glyphs() {
        let theme = Theme {
            unicode: false,
            ..Theme::default()
        };
        assert_eq!(theme.bullet(), "*");
        assert_eq!(theme.cursor_marker(), "> ");
    }
}
