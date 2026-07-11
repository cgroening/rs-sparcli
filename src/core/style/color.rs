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
    fn ansi16_downgrades_rgb_to_named() {
        let resolved = Color::Rgb(200, 0, 0).resolve(ColorSupport::Ansi16);
        assert_eq!(resolved, Some(CtColor::Red));
    }
}
