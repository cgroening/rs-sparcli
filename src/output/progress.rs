//! Progress bars with multiple styles and threshold-based coloring.

use crate::core::render::Rendered;
use crate::core::style::{Color, Style};
use crate::core::text::{Line, Span};
use crate::core::theme::theme;
use crate::error::Result;
use crate::output::live::InPlace;

/// Default bar width in columns.
const DEFAULT_WIDTH: u16 = 30;

/// The visual style of the filled/empty bar cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressStyle {
    /// Solid blocks (`█`/`░`).
    #[default]
    Block,
    /// ASCII (`#`/`-`).
    Ascii,
    /// Heavy line (`━`/`╌`).
    Line,
    /// Shaded blocks (`▓`/`░`).
    Shaded,
}

impl ProgressStyle {
    /// Returns the `(filled, empty)` glyphs for this style.
    fn glyphs(self) -> (char, char) {
        match self {
            Self::Block => ('█', '░'),
            Self::Ascii => ('#', '-'),
            Self::Line => ('━', '╌'),
            Self::Shaded => ('▓', '░'),
        }
    }
}

/// Threshold-based fill colors keyed on the completion ratio.
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    /// Ratio at/above which the mid color applies.
    pub mid: f64,
    /// Ratio at/above which the high color applies.
    pub high: f64,
    /// Color below `mid`.
    pub low_color: Color,
    /// Color in `[mid, high)`.
    pub mid_color: Color,
    /// Color at/above `high`.
    pub high_color: Color,
}

/// A configurable progress bar.
pub struct ProgressBar {
    style: ProgressStyle,
    left_cap: String,
    right_cap: String,
    fill_color: Color,
    empty_color: Color,
    thresholds: Option<Thresholds>,
    show_percent: bool,
    show_value: bool,
    width: u16,
    label: String,
    label_style: Style,
    inplace: Option<InPlace>,
}

impl Default for ProgressBar {
    fn default() -> Self {
        let theme = theme();
        Self {
            style: ProgressStyle::Block,
            left_cap: String::new(),
            right_cap: String::new(),
            fill_color: theme.accent,
            empty_color: Color::DarkGray,
            thresholds: None,
            show_percent: true,
            show_value: false,
            width: DEFAULT_WIDTH,
            label: String::new(),
            label_style: theme.secondary,
            inplace: None,
        }
    }
}

impl ProgressBar {
    /// Creates a progress bar with default styling.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the bar style.
    #[must_use]
    pub fn style(mut self, style: ProgressStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the left and right cap strings.
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

    /// Sets the fill color.
    #[must_use]
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = color;
        self
    }

    /// Sets threshold-based fill colors.
    #[must_use]
    pub fn thresholds(mut self, thresholds: Thresholds) -> Self {
        self.thresholds = Some(thresholds);
        self
    }

    /// Toggles the percentage suffix.
    #[must_use]
    pub fn show_percent(mut self, show: bool) -> Self {
        self.show_percent = show;
        self
    }

    /// Toggles the `(value/max)` suffix.
    #[must_use]
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Sets the bar width in columns.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.width = width.max(1);
        self
    }

    /// Sets a leading label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Builds the bar as a single rendered line for the given progress.
    pub fn bar(&self, value: f64, max: f64) -> Rendered {
        let ratio = ratio_of(value, max);
        let filled = (ratio * self.width as f64).round() as usize;
        let filled = filled.min(self.width as usize);
        let empty = self.width as usize - filled;
        let (fill_glyph, empty_glyph) = self.style.glyphs();
        let mut spans = Vec::new();
        self.push_label(&mut spans);
        self.push_cap(&mut spans, &self.left_cap);
        spans.push(Span::styled(
            fill_glyph.to_string().repeat(filled),
            Style::new().fg(self.resolve_fill_color(ratio)),
        ));
        spans.push(Span::styled(
            empty_glyph.to_string().repeat(empty),
            Style::new().fg(self.empty_color),
        ));
        self.push_cap(&mut spans, &self.right_cap);
        self.push_suffix(&mut spans, ratio, value, max);
        Rendered::new(vec![Line::new(spans)])
    }

    /// Draws the bar in place (animated when on a terminal).
    ///
    /// # Errors
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn draw(&mut self, value: f64, max: f64) -> Result<()> {
        let frame = self.bar(value, max);
        self.inplace
            .get_or_insert_with(|| InPlace::new(false))
            .draw(&frame)
    }

    /// Draws the final bar and ends the in-place session with a newline.
    ///
    /// # Errors
    /// Returns [`crate::SparcliError::Io`] if writing fails.
    pub fn finish(mut self, value: f64, max: f64) -> Result<()> {
        let frame = self.bar(value, max);
        let mut inplace =
            self.inplace.take().unwrap_or_else(|| InPlace::new(false));
        inplace.draw(&frame)?;
        inplace.finish()
    }

    /// Resolves the fill color for `ratio`, honoring thresholds.
    fn resolve_fill_color(&self, ratio: f64) -> Color {
        match self.thresholds {
            None => self.fill_color,
            Some(t) if ratio >= t.high => t.high_color,
            Some(t) if ratio >= t.mid => t.mid_color,
            Some(t) => t.low_color,
        }
    }

    /// Pushes the leading label span, if any.
    fn push_label(&self, spans: &mut Vec<Span>) {
        if !self.label.is_empty() {
            spans.push(Span::styled(
                format!("{} ", self.label),
                self.label_style,
            ));
        }
    }

    /// Pushes a cap span, if non-empty.
    fn push_cap(&self, spans: &mut Vec<Span>, cap: &str) {
        if !cap.is_empty() {
            spans.push(Span::raw(cap.to_string()));
        }
    }

    /// Pushes the percentage and/or value suffix.
    fn push_suffix(
        &self,
        spans: &mut Vec<Span>,
        ratio: f64,
        value: f64,
        max: f64,
    ) {
        if self.show_percent {
            let percent = (ratio * 100.0).round() as i64;
            spans.push(Span::styled(
                format!(" {percent:>3}%"),
                self.label_style,
            ));
        }
        if self.show_value {
            spans.push(Span::styled(
                format!(" ({value:.0}/{max:.0})"),
                self.label_style,
            ));
        }
    }
}

/// Clamps `value / max` into `[0, 1]`, treating non-positive `max` as zero.
fn ratio_of(value: f64, max: f64) -> f64 {
    if max <= 0.0 {
        return 0.0;
    }
    (value / max).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(rendered: &Rendered) -> String {
        rendered.lines[0].plain()
    }

    #[test]
    fn empty_bar_is_all_empty_glyphs() {
        let bar = ProgressBar::new().width(4).show_percent(false);
        assert_eq!(plain(&bar.bar(0.0, 10.0)), "░░░░");
    }

    #[test]
    fn full_bar_is_all_filled_glyphs() {
        let bar = ProgressBar::new().width(4).show_percent(false);
        assert_eq!(plain(&bar.bar(10.0, 10.0)), "████");
    }

    #[test]
    fn half_bar_is_half_filled() {
        let bar = ProgressBar::new().width(4).show_percent(false);
        assert_eq!(plain(&bar.bar(5.0, 10.0)), "██░░");
    }

    #[test]
    fn percent_suffix_is_shown() {
        let bar = ProgressBar::new().width(2);
        assert!(plain(&bar.bar(1.0, 4.0)).contains("25%"));
    }

    #[test]
    fn zero_max_is_safe() {
        let bar = ProgressBar::new().width(3).show_percent(false);
        assert_eq!(plain(&bar.bar(1.0, 0.0)), "░░░");
    }
}
