//! Bordered panels framing content with an optional title and subtitle.
//!
//! The frame geometry itself lives in [`crate::output::box_draw`], which
//! [`Alert`](crate::output::alert::Alert) shares.

use crate::core::border::BorderType;
use crate::core::geometry::{Align, Edges, Position, Title};
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::Text;
use crate::output::box_draw::{BoxOpts, draw_box};

/// A bordered panel around rich content.
///
/// # Examples
///
/// ```
/// use sparcli::{Panel, Renderable, Title};
///
/// let out = Panel::new("Ready.").title(Title::new("Status")).render(40);
/// assert!(out.plain().contains("Ready."));
/// ```
pub struct Panel {
    content: Rendered,
    opts: BoxOpts,
}

impl Panel {
    /// Creates a panel around text content.
    pub fn new(content: impl Into<Text>) -> Self {
        let text = content.into();
        Self {
            content: Rendered::new(text.lines),
            opts: BoxOpts::default(),
        }
    }

    /// Creates a panel around an already rendered block.
    pub fn from_rendered(content: Rendered) -> Self {
        Self {
            content,
            opts: BoxOpts::default(),
        }
    }

    /// Sets the border type.
    #[must_use]
    pub fn border(mut self, border: BorderType) -> Self {
        self.opts.border = border;
        self
    }

    /// Sets the border glyph style.
    #[must_use]
    pub fn border_style(mut self, style: Style) -> Self {
        self.opts.border_style = style;
        self
    }

    /// Sets the interior fill style (e.g. a background color).
    #[must_use]
    pub fn fill(mut self, style: Style) -> Self {
        self.opts.fill = style;
        self
    }

    /// Sets the inner padding.
    #[must_use]
    pub fn padding(mut self, padding: Edges) -> Self {
        self.opts.padding = padding;
        self
    }

    /// Sets the title.
    #[must_use]
    pub fn title(mut self, title: impl Into<Title>) -> Self {
        self.opts.title = Some(title.into());
        self
    }

    /// Sets the subtitle (placed on the bottom edge unless overridden).
    #[must_use]
    pub fn subtitle(mut self, mut subtitle: Title) -> Self {
        subtitle.position = Position::Bottom;
        self.opts.subtitle = Some(subtitle);
        self
    }

    /// Sets a fixed outer width in columns.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.opts.width = Some(width);
        self
    }

    /// Sets the horizontal content alignment.
    #[must_use]
    pub fn content_align(mut self, align: Align) -> Self {
        self.opts.content_align = align;
        self
    }
}

impl Renderable for Panel {
    fn render(&self, max_width: u16) -> Rendered {
        draw_box(&self.content, &self.opts, max_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_frames_content_with_border() {
        let panel = Panel::new("hi").border(BorderType::Single);
        let lines = panel.render(40).plain_lines();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with('┌'));
        assert!(lines[1].contains("hi"));
        assert!(lines[2].starts_with('└'));
    }

    #[test]
    fn panel_embeds_title_in_top_border() {
        let panel = Panel::new("body")
            .border(BorderType::Single)
            .title(Title::new("Info"));
        let lines = panel.render(40).plain_lines();
        assert!(lines[0].contains("Info"));
    }

    #[test]
    fn left_title_reads_as_part_of_the_border() {
        // With slack around the title, a left-aligned title keeps one
        // connecting glyph between the corner and the title, so the top border
        // reads `┌─ Info ─` rather than a flush `┌ Info ─`.
        let panel = Panel::new("a wide body of content")
            .border(BorderType::Single)
            .title(Title::new("Info"));
        let lines = panel.render(40).plain_lines();
        assert!(lines[0].starts_with("\u{250c}\u{2500} Info "));
    }

    #[test]
    fn borderless_panel_only_pads() {
        let panel = Panel::new("x")
            .border(BorderType::None)
            .padding(Edges::all(1));
        let lines = panel.render(40).plain_lines();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], " x ");
    }

    #[test]
    fn fixed_width_is_clamped_to_max_width() {
        // A fixed width wider than the terminal is capped so the border fits.
        let panel = Panel::new("hi").border(BorderType::Single).width(200);
        let lines = panel.render(80).plain_lines();
        assert_eq!(lines[0].chars().count(), 80);
        assert_eq!(lines[2].chars().count(), 80);
    }

    #[test]
    fn overflowing_content_shrinks_the_frame() {
        // Natural content wider than max_width shrinks the frame instead of
        // overflowing the border edges.
        let panel =
            Panel::new("abcdefghijklmnopqrstuvwxyz").border(BorderType::Single);
        let lines = panel.render(12).plain_lines();
        assert_eq!(lines[0].chars().count(), 12);
    }

    #[test]
    fn overlong_title_is_truncated_not_widened() {
        // A title too wide for the interior is truncated into the border run
        // rather than widening the frame.
        let panel = Panel::new("x")
            .border(BorderType::Single)
            .title(Title::new("A very long title here"));
        let lines = panel.render(20).plain_lines();
        assert!(lines[0].chars().count() <= 20);
        assert!(lines[0].contains('…'));
    }
}
