//! Terminal colors and their crossterm/ANSI-downgrade conversions.

use crossterm::style::Color as CtColor;

use crate::core::terminal::ColorSupport;

/// A terminal color.
///
/// Variant names follow ratatui: the plain names map to the eight standard
/// ANSI colors, the `Light*` names to their bright variants. Colors are
/// downgraded automatically (truecolor -> ANSI-16 -> none) to match the
/// terminal at render time.
///
/// # Examples
///
/// ```
/// use sparcli::{Color, Style};
///
/// let style = Style::new().fg(Color::Rgb(137, 180, 250));
/// assert_eq!(Color::from_name("red"), Some(Color::Red));
/// assert_eq!(Color::from_name("nope"), None);
/// let _ = style;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Reset to the terminal's default color.
    Reset,
    /// Standard black.
    Black,
    /// Standard red.
    Red,
    /// Standard green.
    Green,
    /// Standard yellow.
    Yellow,
    /// Standard blue.
    Blue,
    /// Standard magenta.
    Magenta,
    /// Standard cyan.
    Cyan,
    /// Light gray (standard white).
    Gray,
    /// Dark gray (bright black).
    DarkGray,
    /// Bright red.
    LightRed,
    /// Bright green.
    LightGreen,
    /// Bright yellow.
    LightYellow,
    /// Bright blue.
    LightBlue,
    /// Bright magenta.
    LightMagenta,
    /// Bright cyan.
    LightCyan,
    /// Bright white.
    White,
    /// A 24-bit truecolor value.
    Rgb(u8, u8, u8),
    /// An index into the 256-color palette.
    Indexed(u8),
}

impl Color {
    /// Parses a color name (case-insensitive), e.g. `"red"` or `"lightblue"`.
    ///
    /// Returns `None` for unknown names; `#rrggbb` is handled by the caller.
    pub fn from_name(name: &str) -> Option<Self> {
        let color = match name.trim().to_ascii_lowercase().as_str() {
            "reset" | "default" => Self::Reset,
            "black" => Self::Black,
            "red" => Self::Red,
            "green" => Self::Green,
            "yellow" => Self::Yellow,
            "blue" => Self::Blue,
            "magenta" | "purple" => Self::Magenta,
            "cyan" => Self::Cyan,
            "gray" | "grey" | "white" => Self::Gray,
            "darkgray" | "darkgrey" => Self::DarkGray,
            "lightred" => Self::LightRed,
            "lightgreen" => Self::LightGreen,
            "lightyellow" => Self::LightYellow,
            "lightblue" => Self::LightBlue,
            "lightmagenta" => Self::LightMagenta,
            "lightcyan" => Self::LightCyan,
            "brightwhite" => Self::White,
            _ => return None,
        };
        Some(color)
    }

    /// Parses a `#rrggbb` hex color. Returns `None` if malformed.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let digits = hex.strip_prefix('#').unwrap_or(hex);
        if digits.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&digits[0..2], 16).ok()?;
        let g = u8::from_str_radix(&digits[2..4], 16).ok()?;
        let b = u8::from_str_radix(&digits[4..6], 16).ok()?;
        Some(Self::Rgb(r, g, b))
    }

    /// Returns the 24-bit RGB value of this color, if it has one.
    ///
    /// Named colors and palette indices resolve through the standard xterm
    /// palette: slots 0-15 from a fixed table, 16-231 from the 6x6x6 color
    /// cube, 232-255 from the grayscale ramp. [`Color::Reset`] adopts the
    /// terminal's default color and therefore has no fixed value.
    ///
    /// # Examples
    ///
    /// ```
    /// use sparcli::Color;
    ///
    /// assert_eq!(Color::Rgb(1, 2, 3).to_rgb(), Some((1, 2, 3)));
    /// assert_eq!(Color::LightRed.to_rgb(), Some((255, 0, 0)));
    /// assert_eq!(Color::Indexed(196).to_rgb(), Some((255, 0, 0)));
    /// assert_eq!(Color::Reset.to_rgb(), None);
    /// ```
    pub fn to_rgb(self) -> Option<(u8, u8, u8)> {
        match self {
            Self::Reset => None,
            Self::Rgb(r, g, b) => Some((r, g, b)),
            Self::Indexed(index) => Some(indexed_to_rgb(index)),
            named => ansi_index(named).map(|i| ANSI16_RGB[i as usize]),
        }
    }

    /// Resolves the color for the given support level.
    ///
    /// Returns `None` when nothing should be emitted (no-color terminals or
    /// [`Color::Reset`] is handled by the caller via [`crossterm`] reset).
    pub(crate) fn resolve(self, support: ColorSupport) -> Option<CtColor> {
        match support {
            ColorSupport::None => None,
            ColorSupport::TrueColor => Some(self.to_crossterm()),
            ColorSupport::Ansi16 => Some(self.downgrade_to_ansi16()),
        }
    }

    /// Maps the color to its [`crossterm`] equivalent without downgrading.
    fn to_crossterm(self) -> CtColor {
        match self {
            Self::Reset => CtColor::Reset,
            Self::Black => CtColor::Black,
            Self::Red => CtColor::DarkRed,
            Self::Green => CtColor::DarkGreen,
            Self::Yellow => CtColor::DarkYellow,
            Self::Blue => CtColor::DarkBlue,
            Self::Magenta => CtColor::DarkMagenta,
            Self::Cyan => CtColor::DarkCyan,
            Self::Gray => CtColor::Grey,
            Self::DarkGray => CtColor::DarkGrey,
            Self::LightRed => CtColor::Red,
            Self::LightGreen => CtColor::Green,
            Self::LightYellow => CtColor::Yellow,
            Self::LightBlue => CtColor::Blue,
            Self::LightMagenta => CtColor::Magenta,
            Self::LightCyan => CtColor::Cyan,
            Self::White => CtColor::White,
            Self::Rgb(r, g, b) => CtColor::Rgb { r, g, b },
            Self::Indexed(i) => CtColor::AnsiValue(i),
        }
    }

    /// Downgrades RGB/indexed colors to the nearest named ANSI-16 color.
    fn downgrade_to_ansi16(self) -> CtColor {
        match self {
            Self::Rgb(r, g, b) => nearest_ansi16(r, g, b).to_crossterm(),
            Self::Indexed(i) => indexed_to_named(i).to_crossterm(),
            other => other.to_crossterm(),
        }
    }
}

/// Standard xterm RGB values of the sixteen named colors, by palette index.
const ANSI16_RGB: [(u8, u8, u8); 16] = [
    (0x00, 0x00, 0x00), // 0  Black
    (0x80, 0x00, 0x00), // 1  Red
    (0x00, 0x80, 0x00), // 2  Green
    (0x80, 0x80, 0x00), // 3  Yellow
    (0x00, 0x00, 0x80), // 4  Blue
    (0x80, 0x00, 0x80), // 5  Magenta
    (0x00, 0x80, 0x80), // 6  Cyan
    (0xc0, 0xc0, 0xc0), // 7  Gray
    (0x80, 0x80, 0x80), // 8  DarkGray
    (0xff, 0x00, 0x00), // 9  LightRed
    (0x00, 0xff, 0x00), // 10 LightGreen
    (0xff, 0xff, 0x00), // 11 LightYellow
    (0x00, 0x00, 0xff), // 12 LightBlue
    (0xff, 0x00, 0xff), // 13 LightMagenta
    (0x00, 0xff, 0xff), // 14 LightCyan
    (0xff, 0xff, 0xff), // 15 White
];

/// The six per-channel levels of the 6x6x6 color cube (indices 16-231).
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
/// First palette index of the color cube.
const CUBE_START: u8 = 16;
/// First palette index of the grayscale ramp.
const GRAY_START: u8 = 232;
/// Value of the darkest grayscale ramp entry.
const GRAY_BASE: u8 = 8;
/// Value step between two grayscale ramp entries.
const GRAY_STEP: u8 = 10;

/// Returns the palette index of a named color, or `None` for the other
/// variants. Inverse of [`indexed_to_named`] for the range 0-15.
fn ansi_index(color: Color) -> Option<u8> {
    let index = match color {
        Color::Black => 0,
        Color::Red => 1,
        Color::Green => 2,
        Color::Yellow => 3,
        Color::Blue => 4,
        Color::Magenta => 5,
        Color::Cyan => 6,
        Color::Gray => 7,
        Color::DarkGray => 8,
        Color::LightRed => 9,
        Color::LightGreen => 10,
        Color::LightYellow => 11,
        Color::LightBlue => 12,
        Color::LightMagenta => 13,
        Color::LightCyan => 14,
        Color::White => 15,
        _ => return None,
    };
    Some(index)
}

/// Maps a 256-color index to its RGB value.
fn indexed_to_rgb(index: u8) -> (u8, u8, u8) {
    if index < CUBE_START {
        return ANSI16_RGB[index as usize];
    }
    if index >= GRAY_START {
        let level = GRAY_BASE + (index - GRAY_START) * GRAY_STEP;
        return (level, level, level);
    }
    cube_to_rgb(index - CUBE_START)
}

/// Maps an offset into the 6x6x6 color cube to its RGB value.
fn cube_to_rgb(offset: u8) -> (u8, u8, u8) {
    const PLANE: u8 = 36;
    const ROW: u8 = 6;
    let level = |step: u8| CUBE_LEVELS[step as usize];
    (
        level(offset / PLANE),
        level((offset / ROW) % ROW),
        level(offset % ROW),
    )
}

/// Maps an RGB triple to the closest of the sixteen named colors.
fn nearest_ansi16(r: u8, g: u8, b: u8) -> Color {
    const THRESHOLD: u8 = 0x60;
    const BRIGHT: u8 = 0xc0;
    let bright = r >= BRIGHT || g >= BRIGHT || b >= BRIGHT;
    let bit = |channel: u8| channel >= THRESHOLD;
    match (bit(r), bit(g), bit(b), bright) {
        (false, false, false, false) => Color::Black,
        (false, false, false, true) => Color::DarkGray,
        (true, false, false, false) => Color::Red,
        (true, false, false, true) => Color::LightRed,
        (false, true, false, false) => Color::Green,
        (false, true, false, true) => Color::LightGreen,
        (true, true, false, false) => Color::Yellow,
        (true, true, false, true) => Color::LightYellow,
        (false, false, true, false) => Color::Blue,
        (false, false, true, true) => Color::LightBlue,
        (true, false, true, false) => Color::Magenta,
        (true, false, true, true) => Color::LightMagenta,
        (false, true, true, false) => Color::Cyan,
        (false, true, true, true) => Color::LightCyan,
        (true, true, true, false) => Color::Gray,
        (true, true, true, true) => Color::White,
    }
}

/// Maps a 256-color index to an approximate named color.
fn indexed_to_named(index: u8) -> Color {
    match index {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        15 => Color::White,
        // Approximate the cube/grayscale ramp by overall brightness.
        i if i >= 244 => Color::White,
        i if i >= 232 => Color::DarkGray,
        _ => Color::Gray,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_hex_parses_six_digit_colors() {
        assert_eq!(Color::from_hex("#ff8800"), Some(Color::Rgb(255, 136, 0)));
        assert_eq!(Color::from_hex("8899aa"), Some(Color::Rgb(136, 153, 170)));
    }

    #[test]
    fn from_hex_rejects_malformed_input() {
        assert_eq!(Color::from_hex("#fff"), None);
        assert_eq!(Color::from_hex("#gggggg"), None);
    }

    #[test]
    fn from_name_is_case_insensitive() {
        assert_eq!(Color::from_name("RED"), Some(Color::Red));
        assert_eq!(Color::from_name("LightBlue"), Some(Color::LightBlue));
        assert_eq!(Color::from_name("nope"), None);
    }

    #[test]
    fn none_support_emits_no_color() {
        assert_eq!(Color::Red.resolve(ColorSupport::None), None);
    }

    #[test]
    fn to_rgb_maps_named_colors_to_the_xterm_palette() {
        assert_eq!(Color::Black.to_rgb(), Some((0, 0, 0)));
        assert_eq!(Color::Gray.to_rgb(), Some((192, 192, 192)));
        assert_eq!(Color::LightRed.to_rgb(), Some((255, 0, 0)));
    }

    #[test]
    fn to_rgb_resolves_the_color_cube() {
        assert_eq!(Color::Indexed(16).to_rgb(), Some((0, 0, 0)));
        assert_eq!(Color::Indexed(196).to_rgb(), Some((255, 0, 0)));
        assert_eq!(Color::Indexed(231).to_rgb(), Some((255, 255, 255)));
    }

    #[test]
    fn to_rgb_resolves_the_grayscale_ramp() {
        assert_eq!(Color::Indexed(232).to_rgb(), Some((8, 8, 8)));
        assert_eq!(Color::Indexed(255).to_rgb(), Some((238, 238, 238)));
    }

    #[test]
    fn to_rgb_passes_truecolor_through() {
        assert_eq!(Color::Rgb(1, 2, 3).to_rgb(), Some((1, 2, 3)));
    }

    #[test]
    fn reset_has_no_rgb_value() {
        assert_eq!(Color::Reset.to_rgb(), None);
    }

    #[test]
    fn low_indices_agree_with_the_named_downgrade() {
        // `ANSI16_RGB`/`ansi_index` and `indexed_to_named` describe the same
        // sixteen slots; this pins them together instead of relying on a
        // "keep in sync" comment.
        for index in 0..16u8 {
            assert_eq!(ansi_index(indexed_to_named(index)), Some(index));
        }
    }

    #[test]
    fn ansi16_downgrades_rgb_to_named() {
        let resolved = Color::Rgb(200, 0, 0).resolve(ColorSupport::Ansi16);
        assert_eq!(resolved, Some(CtColor::Red));
    }
}
