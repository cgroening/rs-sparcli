//! Border styles and their box-drawing glyph sets.

/// The visual style of a border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderType {
    /// No border.
    None,
    /// ASCII-only border (`+`, `-`, `|`).
    Ascii,
    /// Single-line box drawing.
    Single,
    /// Double-line box drawing.
    Double,
    /// Single-line box drawing with rounded corners.
    #[default]
    Rounded,
    /// Heavy/thick box drawing.
    Thick,
}

/// The glyphs used to draw a border of a given [`BorderType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderChars {
    /// Top-left corner.
    pub top_left: char,
    /// Top-right corner.
    pub top_right: char,
    /// Bottom-left corner.
    pub bottom_left: char,
    /// Bottom-right corner.
    pub bottom_right: char,
    /// Horizontal edge.
    pub horizontal: char,
    /// Vertical edge.
    pub vertical: char,
    /// Four-way junction (used by tables).
    pub cross: char,
    /// T-junction pointing down (top edge column separator).
    pub tee_down: char,
    /// T-junction pointing up (bottom edge column separator).
    pub tee_up: char,
    /// T-junction pointing right (left edge row separator).
    pub tee_right: char,
    /// T-junction pointing left (right edge row separator).
    pub tee_left: char,
}

impl BorderType {
    /// Returns the glyph set for this border type.
    ///
    /// [`BorderType::None`] returns spaces; callers normally skip drawing.
    pub fn chars(self) -> BorderChars {
        match self {
            Self::None => SPACE,
            Self::Ascii => ASCII,
            Self::Single => SINGLE,
            Self::Double => DOUBLE,
            Self::Rounded => ROUNDED,
            Self::Thick => THICK,
        }
    }

    /// Returns `true` if this border type draws nothing.
    pub fn is_none(self) -> bool {
        matches!(self, Self::None)
    }
}

const SPACE: BorderChars = BorderChars {
    top_left: ' ',
    top_right: ' ',
    bottom_left: ' ',
    bottom_right: ' ',
    horizontal: ' ',
    vertical: ' ',
    cross: ' ',
    tee_down: ' ',
    tee_up: ' ',
    tee_right: ' ',
    tee_left: ' ',
};

const ASCII: BorderChars = BorderChars {
    top_left: '+',
    top_right: '+',
    bottom_left: '+',
    bottom_right: '+',
    horizontal: '-',
    vertical: '|',
    cross: '+',
    tee_down: '+',
    tee_up: '+',
    tee_right: '+',
    tee_left: '+',
};

const SINGLE: BorderChars = BorderChars {
    top_left: '┌',
    top_right: '┐',
    bottom_left: '└',
    bottom_right: '┘',
    horizontal: '─',
    vertical: '│',
    cross: '┼',
    tee_down: '┬',
    tee_up: '┴',
    tee_right: '├',
    tee_left: '┤',
};

const DOUBLE: BorderChars = BorderChars {
    top_left: '╔',
    top_right: '╗',
    bottom_left: '╚',
    bottom_right: '╝',
    horizontal: '═',
    vertical: '║',
    cross: '╬',
    tee_down: '╦',
    tee_up: '╩',
    tee_right: '╠',
    tee_left: '╣',
};

const ROUNDED: BorderChars = BorderChars {
    top_left: '╭',
    top_right: '╮',
    bottom_left: '╰',
    bottom_right: '╯',
    horizontal: '─',
    vertical: '│',
    cross: '┼',
    tee_down: '┬',
    tee_up: '┴',
    tee_right: '├',
    tee_left: '┤',
};

const THICK: BorderChars = BorderChars {
    top_left: '┏',
    top_right: '┓',
    bottom_left: '┗',
    bottom_right: '┛',
    horizontal: '━',
    vertical: '┃',
    cross: '╋',
    tee_down: '┳',
    tee_up: '┻',
    tee_right: '┣',
    tee_left: '┫',
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_is_the_default() {
        assert_eq!(BorderType::default(), BorderType::Rounded);
    }

    #[test]
    fn rounded_uses_curved_corners() {
        let chars = BorderType::Rounded.chars();
        assert_eq!(chars.top_left, '╭');
        assert_eq!(chars.bottom_right, '╯');
    }

    #[test]
    fn none_reports_is_none() {
        assert!(BorderType::None.is_none());
        assert!(!BorderType::Single.is_none());
    }
}
