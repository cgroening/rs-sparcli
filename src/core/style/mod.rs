//! Colors, text attributes and styles.
//!
//! The types mirror ratatui's vocabulary so the API feels familiar, but they
//! convert to [`crossterm`] for rendering and carry the terminal-aware color
//! downgrade logic (truecolor -> ANSI-16 -> none). The [`Color`] model and its
//! terminal conversions live in a dedicated submodule.

mod color;

pub use self::color::Color;

use std::ops::{BitOr, BitOrAssign};

/// Text attribute flags (bold, dim, italic, …), combinable with `|`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Attribute(u8);

impl Attribute {
    /// No attributes.
    pub const NONE: Self = Self(0);
    /// Bold / increased intensity.
    pub const BOLD: Self = Self(1 << 0);
    /// Dim / decreased intensity.
    pub const DIM: Self = Self(1 << 1);
    /// Italic.
    pub const ITALIC: Self = Self(1 << 2);
    /// Underlined.
    pub const UNDERLINED: Self = Self(1 << 3);
    /// Crossed out.
    pub const STRIKETHROUGH: Self = Self(1 << 4);

    /// Returns `true` if every flag in `other` is set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns `true` if no attribute is set.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for Attribute {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Attribute {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Familiar alias for [`Attribute`] (ratatui calls this `Modifier`).
pub type Modifier = Attribute;

/// A foreground/background color pair plus text attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    /// Foreground color, if set.
    pub fg: Option<Color>,
    /// Background color, if set.
    pub bg: Option<Color>,
    /// Combined text attributes.
    pub attrs: Attribute,
}

impl Style {
    /// Creates an empty style (no colors, no attributes).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the foreground color.
    #[must_use]
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// Sets the background color.
    #[must_use]
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Adds one or more attributes (ratatui-style name).
    #[must_use]
    pub fn add_modifier(mut self, attr: Attribute) -> Self {
        self.attrs |= attr;
        self
    }

    /// Clears one or more attributes from the style.
    #[must_use]
    pub fn remove_modifier(mut self, attr: Attribute) -> Self {
        self.attrs = Attribute(self.attrs.0 & !attr.0);
        self
    }

    /// Adds the bold attribute.
    #[must_use]
    pub fn bold(self) -> Self {
        self.add_modifier(Attribute::BOLD)
    }

    /// Adds the dim attribute.
    #[must_use]
    pub fn dim(self) -> Self {
        self.add_modifier(Attribute::DIM)
    }

    /// Adds the italic attribute.
    #[must_use]
    pub fn italic(self) -> Self {
        self.add_modifier(Attribute::ITALIC)
    }

    /// Adds the underlined attribute.
    #[must_use]
    pub fn underlined(self) -> Self {
        self.add_modifier(Attribute::UNDERLINED)
    }

    /// Adds the strikethrough attribute.
    #[must_use]
    pub fn strikethrough(self) -> Self {
        self.add_modifier(Attribute::STRIKETHROUGH)
    }

    /// Merges `other` on top of `self`: set colors and attributes from
    /// `other` win, unset fields keep `self`'s values.
    #[must_use]
    pub fn patch(mut self, other: Style) -> Self {
        if other.fg.is_some() {
            self.fg = other.fg;
        }
        if other.bg.is_some() {
            self.bg = other.bg;
        }
        self.attrs |= other.attrs;
        self
    }
}

impl From<Color> for Style {
    fn from(color: Color) -> Self {
        Style::new().fg(color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_combine_and_test_membership() {
        let attrs = Attribute::BOLD | Attribute::ITALIC;
        assert!(attrs.contains(Attribute::BOLD));
        assert!(attrs.contains(Attribute::ITALIC));
        assert!(!attrs.contains(Attribute::DIM));
    }

    #[test]
    fn patch_overrides_set_fields_only() {
        let base = Style::new().fg(Color::Red).bold();
        let patched = base.patch(Style::new().bg(Color::Blue).italic());
        assert_eq!(patched.fg, Some(Color::Red));
        assert_eq!(patched.bg, Some(Color::Blue));
        assert!(patched.attrs.contains(Attribute::BOLD));
        assert!(patched.attrs.contains(Attribute::ITALIC));
    }

    #[test]
    fn remove_modifier_clears_only_the_named_attribute() {
        let style = Style::new()
            .bold()
            .italic()
            .remove_modifier(Attribute::BOLD);
        assert!(!style.attrs.contains(Attribute::BOLD));
        assert!(style.attrs.contains(Attribute::ITALIC));
    }
}
