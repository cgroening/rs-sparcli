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
    /// A thin block frame around a filled surface, following the geometry of
    /// Textual's `wide` border.
    ///
    /// The side bars ink a quarter of their cell's width and the top and
    /// bottom lines an eighth of their cell's height, which comes out the same
    /// number of pixels because a terminal cell is about twice as tall as it
    /// is wide. The horizontal lines run across the corner cells as well, so
    /// the corners close.
    ///
    /// Only [`Card`](crate::output::card::Card) draws this natively: the bars
    /// need a filled surface to read against, all four edges use a different
    /// glyph, and the right-hand one is painted with foreground and background
    /// swapped - none of which [`BorderChars`], with its single `horizontal`
    /// and `vertical` and one uniform style, can express. Every other widget
    /// receives the [`BorderType::Thick`] glyphs from [`BorderType::chars`]
    /// instead.
    Tall,
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
    /// [`BorderType::Tall`] returns the heavy line glyphs, since its block
    /// glyphs cannot be expressed as a [`BorderChars`] set - widgets that do
    /// not draw it natively therefore fall back to a heavy frame.
    pub fn chars(self) -> BorderChars {
        match self {
            Self::None => SPACE,
            Self::Ascii => ASCII,
            Self::Single => SINGLE,
            Self::Double => DOUBLE,
            Self::Rounded => ROUNDED,
            Self::Thick | Self::Tall => THICK,
        }
    }

    /// Returns `true` if this border type draws nothing.
    pub fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns `true` if this border type is drawn from block glyphs.
    pub fn is_tall(self) -> bool {
        matches!(self, Self::Tall)
    }
}

/// The block glyphs of a [`BorderType::Tall`] border.
///
/// The sides ink a quarter of their cell's width, the top and bottom an
/// eighth of their cell's height. Since a terminal cell is roughly twice as
/// tall as it is wide, both come out the same number of pixels - a fraction
/// equal on both axes would not.
///
/// The horizontal glyphs sit on the *inner* side of their row (`▁` at the top
/// of the frame, `▔` at the bottom) and run across the corner cells too. That
/// is what closes the corner: the line touches the side bar of the adjoining
/// row instead of starting a cell away from it.
///
/// [`BorderChars`] cannot express any of this: it offers one `horizontal` and
/// one `vertical` for all four edges, and no way to say that a glyph is
/// painted with foreground and background swapped.
pub(crate) struct TallChars {
    /// Left edge, inking the left quarter of its cell.
    pub left: char,
    /// Right edge, painted with swapped colors so its right quarter inks.
    pub right: char,
    /// Top edge, inking the bottom eighth of its cell.
    pub top: char,
    /// Bottom edge, inking the top eighth of its cell.
    pub bottom: char,
}

pub(crate) const TALL: TallChars = TallChars {
    left: '\u{258e}',  // LEFT ONE QUARTER BLOCK
    right: '\u{258a}', // LEFT THREE QUARTERS BLOCK, painted with swapped
    // colors so its remaining quarter becomes the bar - Unicode has no
    // right-aligned quarter block.
    top: '\u{2581}',    // LOWER ONE EIGHTH BLOCK
    bottom: '\u{2594}', // UPPER ONE EIGHTH BLOCK
};

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

    #[test]
    fn tall_degrades_to_thick_glyphs() {
        // Widgets that cannot draw half blocks read `chars()` directly, so
        // this is what keeps them from rendering a blank frame.
        assert_eq!(BorderType::Tall.chars(), BorderType::Thick.chars());
    }

    #[test]
    fn tall_is_a_border_of_its_own() {
        assert!(BorderType::Tall.is_tall());
        assert!(!BorderType::Tall.is_none());
        assert!(!BorderType::Thick.is_tall());
    }

    #[test]
    fn tall_strokes_are_equally_thick_on_both_axes() {
        // A quarter of the cell width and an eighth of its height come out the
        // same number of pixels, because a cell is about twice as tall as it
        // is wide. Equal fractions on both axes would not.
        assert_eq!(TALL.left, '▎');
        assert_eq!(TALL.right, '▊');
        assert_eq!(TALL.top, '▁');
        assert_eq!(TALL.bottom, '▔');
    }
}
