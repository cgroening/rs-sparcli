//! Conversion between RGB triples and the HSL color space.
//!
//! HSL separates a color's identity (hue) from its intensity (saturation) and
//! brightness (lightness), which is what lets a widget re-shade one accent
//! color into a whole palette without changing its character.

/// Degrees covered by one of the six hue sectors.
const DEGREES_PER_SECTOR: f32 = 60.0;
/// Degrees of a full hue circle.
const FULL_TURN: f32 = 360.0;
/// Largest value of an 8-bit color channel.
const CHANNEL_MAX: f32 = 255.0;

/// A color in HSL space: `hue` in degrees, the rest in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Hsl {
    /// Hue in degrees (`0.0..360.0`).
    pub hue: f32,
    /// Saturation, from gray (`0.0`) to fully saturated (`1.0`).
    pub saturation: f32,
    /// Lightness, from black (`0.0`) over the pure hue (`0.5`) to white.
    pub lightness: f32,
}

impl Hsl {
    /// Converts an RGB triple to HSL.
    ///
    /// An achromatic input (all channels equal) yields a hue of zero; callers
    /// that re-saturate a color must treat that hue as meaningless.
    pub(crate) fn from_rgb(rgb: (u8, u8, u8)) -> Self {
        let (red, green, blue) = normalize(rgb);
        let max = red.max(green).max(blue);
        let min = red.min(green).min(blue);
        let lightness = (max + min) / 2.0;
        let delta = max - min;
        if delta == 0.0 {
            return Self {
                hue: 0.0,
                saturation: 0.0,
                lightness,
            };
        }
        let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
        Self {
            hue: hue_of(rgb_max_channel(red, green, blue, max), delta),
            saturation,
            lightness,
        }
    }

    /// Converts back to RGB, clamping components that lie out of range.
    pub(crate) fn to_rgb(self) -> (u8, u8, u8) {
        let saturation = self.saturation.clamp(0.0, 1.0);
        let lightness = self.lightness.clamp(0.0, 1.0);
        let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
        let sector = self.hue.rem_euclid(FULL_TURN) / DEGREES_PER_SECTOR;
        let second = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
        let base = lightness - chroma / 2.0;
        let (red, green, blue) = sector_channels(sector, chroma, second);
        (
            to_channel(red + base),
            to_channel(green + base),
            to_channel(blue + base),
        )
    }
}

/// The three color channels scaled to `0.0..=1.0`.
fn normalize(rgb: (u8, u8, u8)) -> (f32, f32, f32) {
    let (red, green, blue) = rgb;
    (
        f32::from(red) / CHANNEL_MAX,
        f32::from(green) / CHANNEL_MAX,
        f32::from(blue) / CHANNEL_MAX,
    )
}

/// The channel differences relative to the brightest channel, which together
/// with the delta determine the hue.
struct MaxChannel {
    /// Offset of the brightest channel's sector, in degrees.
    offset: f32,
    /// Difference between the two remaining channels.
    span: f32,
}

/// Identifies the brightest channel and the span between the other two.
fn rgb_max_channel(red: f32, green: f32, blue: f32, max: f32) -> MaxChannel {
    if max == red {
        return MaxChannel {
            offset: 0.0,
            span: green - blue,
        };
    }
    if max == green {
        return MaxChannel {
            offset: 2.0,
            span: blue - red,
        };
    }
    MaxChannel {
        offset: 4.0,
        span: red - green,
    }
}

/// Computes the hue in degrees from the brightest channel and the delta.
fn hue_of(channel: MaxChannel, delta: f32) -> f32 {
    let sector = channel.span / delta + channel.offset;
    (sector * DEGREES_PER_SECTOR).rem_euclid(FULL_TURN)
}

/// Distributes chroma over the channels according to the hue sector.
fn sector_channels(sector: f32, chroma: f32, second: f32) -> (f32, f32, f32) {
    match sector as u8 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    }
}

/// Scales a normalized component back to an 8-bit channel value.
fn to_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * CHANNEL_MAX).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_hsl_round_trip_is_stable() {
        let samples = [
            (137, 180, 250),
            (255, 0, 0),
            (12, 200, 87),
            (0, 0, 0),
            (255, 255, 255),
        ];
        for sample in samples {
            let (red, green, blue) = Hsl::from_rgb(sample).to_rgb();
            assert!(red.abs_diff(sample.0) <= 1, "red of {sample:?}");
            assert!(green.abs_diff(sample.1) <= 1, "green of {sample:?}");
            assert!(blue.abs_diff(sample.2) <= 1, "blue of {sample:?}");
        }
    }

    #[test]
    fn gray_has_zero_saturation_and_a_finite_hue() {
        let gray = Hsl::from_rgb((128, 128, 128));
        assert_eq!(gray.saturation, 0.0);
        assert!(gray.hue.is_finite());
    }

    #[test]
    fn converts_the_primary_hues() {
        assert_eq!(Hsl::from_rgb((255, 0, 0)).hue, 0.0);
        assert_eq!(Hsl::from_rgb((0, 255, 0)).hue, 120.0);
        assert_eq!(Hsl::from_rgb((0, 0, 255)).hue, 240.0);
    }

    #[test]
    fn out_of_range_components_are_clamped() {
        let overshoot = Hsl {
            hue: 200.0,
            saturation: 1.5,
            lightness: 1.5,
        };
        assert_eq!(overshoot.to_rgb(), (255, 255, 255));
        let undershoot = Hsl {
            hue: 200.0,
            saturation: -1.0,
            lightness: -1.0,
        };
        assert_eq!(undershoot.to_rgb(), (0, 0, 0));
    }
}
