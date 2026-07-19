//! Derivation of a card's five styles from a single accent color.
//!
//! The accent keeps its hue throughout; only saturation and lightness change.
//! That is what makes one color enough: the title stays saturated, the body
//! text and both surfaces are desaturated shades of the same tone.

use crate::core::style::hsl::Hsl;
use crate::core::style::{Color, Style};
use crate::core::terminal::ColorSupport;

/// Saturation and lightness of the title text.
const TITLE_FG_SATURATION: f32 = 0.85;
const TITLE_FG_LIGHTNESS: f32 = 0.78;
/// Saturation and lightness of the title bar surface.
const TITLE_BG_SATURATION: f32 = 0.35;
const TITLE_BG_LIGHTNESS: f32 = 0.22;
/// Saturation and lightness of the content surface.
const CONTENT_BG_SATURATION: f32 = 0.18;
const CONTENT_BG_LIGHTNESS: f32 = 0.13;
/// Saturation and lightness of the body text.
const CONTENT_FG_SATURATION: f32 = 0.15;
const CONTENT_FG_LIGHTNESS: f32 = 0.75;
/// Below this saturation a color carries no meaningful hue. Re-saturating it
/// would pick up the fallback hue of zero and turn a gray accent into a red
/// one, so such accents stay neutral.
const ACHROMATIC_SATURATION: f32 = 0.05;

/// The five styles a card derives from one accent color.
pub(crate) struct CardStyles {
    /// Style of the border glyphs.
    pub border: Style,
    /// Style of the title row's text, including its own background.
    pub title: Style,
    /// Background of the content surface, used for padding and blank rows.
    pub fill: Style,
    /// Style of the body text.
    pub content: Style,
    /// Style of the footer row's text, including its own background.
    pub footer: Style,
}

/// Derives the card palette from `accent` for the given color support.
///
/// Below [`ColorSupport::TrueColor`] the surfaces are dropped: `nearest_ansi16`
/// quantizes each channel at `0x60`, so both derived backgrounds collapse onto
/// the same named color and the title bar would become indistinguishable from
/// the content area - the card's only separator. An accent without an RGB value
/// ([`Color::Reset`]) takes the same path, since nothing can be derived from it.
pub(crate) fn derive(accent: Color, support: ColorSupport) -> CardStyles {
    let Some(rgb) = accent.to_rgb() else {
        return flat_styles(accent);
    };
    if support != ColorSupport::TrueColor {
        return flat_styles(accent);
    }
    let base = Hsl::from_rgb(rgb);
    let title_bg = shade(base, TITLE_BG_SATURATION, TITLE_BG_LIGHTNESS);
    let content_bg = shade(base, CONTENT_BG_SATURATION, CONTENT_BG_LIGHTNESS);
    let title_fg = shade(base, TITLE_FG_SATURATION, TITLE_FG_LIGHTNESS);
    let content_fg = shade(base, CONTENT_FG_SATURATION, CONTENT_FG_LIGHTNESS);
    CardStyles {
        border: Style::new().fg(accent).bg(content_bg),
        title: Style::new().fg(title_fg).bg(title_bg),
        fill: Style::new().bg(content_bg),
        content: Style::new().fg(content_fg).bg(content_bg),
        footer: Style::new().fg(title_fg).bg(title_bg),
    }
}

/// Re-shades `base` to the given saturation and lightness, keeping its hue.
///
/// An achromatic base stays achromatic: its hue is a fallback value, not a
/// measurement, so re-saturating it would invent a color.
fn shade(base: Hsl, saturation: f32, lightness: f32) -> Color {
    let saturation = if base.saturation < ACHROMATIC_SATURATION {
        0.0
    } else {
        saturation
    };
    let shaded = Hsl {
        hue: base.hue,
        saturation,
        lightness,
    };
    let (red, green, blue) = shaded.to_rgb();
    Color::Rgb(red, green, blue)
}

/// The background-free palette used when surfaces cannot be rendered.
fn flat_styles(accent: Color) -> CardStyles {
    let accented = Style::new().fg(accent);
    CardStyles {
        border: accented,
        title: accented.bold(),
        fill: Style::new(),
        content: Style::new(),
        footer: accented.bold(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::style::Attribute;

    /// The default theme accent, a soft blue.
    const ACCENT: Color = Color::Rgb(137, 180, 250);

    /// Returns the RGB triple of a style's background.
    fn background(style: Style) -> (u8, u8, u8) {
        style
            .bg
            .and_then(Color::to_rgb)
            .expect("a truecolor card style always carries an RGB background")
    }

    /// Returns the sum of a color's channels as a coarse brightness measure.
    fn brightness(rgb: (u8, u8, u8)) -> u32 {
        u32::from(rgb.0) + u32::from(rgb.1) + u32::from(rgb.2)
    }

    #[test]
    fn derives_distinct_styles_from_one_accent() {
        let styles = derive(ACCENT, ColorSupport::TrueColor);
        assert_eq!(styles.border.fg, Some(ACCENT));
        assert_ne!(styles.title.bg, styles.fill.bg);
        assert_ne!(styles.title.fg, styles.content.fg);
    }

    #[test]
    fn content_surface_is_darker_than_the_title_surface() {
        // The background step is the card's only separator between title and
        // content, so their ordering is pinned independently of the exact
        // lightness constants.
        let styles = derive(ACCENT, ColorSupport::TrueColor);
        let title = brightness(background(styles.title));
        let content = brightness(background(styles.fill));
        assert!(content < title, "content {content} vs title {title}");
    }

    #[test]
    fn body_text_is_lighter_than_both_surfaces() {
        let styles = derive(ACCENT, ColorSupport::TrueColor);
        let text = styles
            .content
            .fg
            .and_then(Color::to_rgb)
            .expect("the derived body text is an RGB color");
        assert!(brightness(text) > brightness(background(styles.title)));
    }

    #[test]
    fn derived_shades_keep_the_accent_hue() {
        // The dark surfaces span only a dozen values per channel, so rounding
        // to 8 bits moves the measured hue by a degree or two. The tolerance
        // covers that quantization while still catching a real hue shift.
        const MAX_DRIFT: f32 = 5.0;
        let styles = derive(ACCENT, ColorSupport::TrueColor);
        let accent_hue = Hsl::from_rgb((137, 180, 250)).hue;
        for style in [styles.title, styles.content, styles.fill] {
            let hue = Hsl::from_rgb(background(style)).hue;
            assert!((hue - accent_hue).abs() < MAX_DRIFT, "hue {hue}");
        }
    }

    #[test]
    fn achromatic_accent_stays_neutral() {
        // A gray accent has no hue; re-saturating the fallback hue of zero
        // would turn the whole card red.
        let styles = derive(Color::Rgb(160, 160, 160), ColorSupport::TrueColor);
        for style in [styles.title, styles.fill, styles.content] {
            let (red, green, blue) = background(style);
            assert_eq!((red, green), (green, blue), "{red},{green},{blue}");
        }
    }

    #[test]
    fn fill_and_content_share_a_background() {
        // Blank padding cells and text cells must not drift apart.
        let styles = derive(ACCENT, ColorSupport::TrueColor);
        assert_eq!(styles.fill.bg, styles.content.bg);
    }

    #[test]
    fn ansi16_support_drops_all_backgrounds() {
        let styles = derive(ACCENT, ColorSupport::Ansi16);
        for style in [styles.border, styles.title, styles.fill, styles.content]
        {
            assert_eq!(style.bg, None);
        }
        assert_eq!(styles.title.fg, Some(ACCENT));
        assert!(styles.title.attrs.contains(Attribute::BOLD));
    }

    #[test]
    fn no_color_support_drops_all_backgrounds() {
        let styles = derive(ACCENT, ColorSupport::None);
        assert_eq!(styles.fill.bg, None);
        assert_eq!(styles.content.bg, None);
    }

    #[test]
    fn reset_accent_falls_back_to_the_flat_palette() {
        let styles = derive(Color::Reset, ColorSupport::TrueColor);
        assert_eq!(styles.fill.bg, None);
        assert_eq!(styles.title.bg, None);
    }
}
