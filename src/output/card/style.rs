//! Style math for the card surface.
//!
//! A [`Rendered`](crate::core::render::Rendered) block is a list of styled
//! spans, not a cell grid, so a background paints only the characters of its
//! own span. These helpers keep the surface gapless: they rebase content onto
//! it, derive it from a text style, and carry it into the border glyphs.

use crate::core::style::Style;
use crate::core::text::Line;

/// Returns `style` with foreground and background swapped.
///
/// A tall border's right-hand glyph inks the left three quarters of its cell;
/// swapping turns the remaining quarter into the bar, which is the only way to
/// get a right-aligned quarter block - Unicode does not define one. Returns
/// the style unchanged when either color is unset, since the swap needs both.
pub(super) fn swap_colors(style: Style) -> Style {
    let (Some(fg), Some(bg)) = (style.fg, style.bg) else {
        return style;
    };
    style.fg(bg).bg(fg)
}

/// Rebases every span of `line` onto `base`.
///
/// Without this the content spans keep their own (usually unset) background and
/// punch transparent holes through the surface - invisible in the plain text,
/// visible in the terminal.
pub(super) fn rebase(line: Line, base: Style) -> Line {
    let spans = line
        .spans
        .into_iter()
        .map(|mut span| {
            span.style = base.patch(span.style);
            span
        })
        .collect();
    Line::new(spans)
}

/// Reduces a text style to the background it sits on.
pub(super) fn surface_of(style: Style) -> Style {
    match style.bg {
        Some(bg) => Style::new().bg(bg),
        None => Style::new(),
    }
}

/// Returns the border style carrying the given surface's background, so the
/// glyphs do not leave a transparent seam along the card's edges.
pub(super) fn border_over(border: Style, surface: Style) -> Style {
    match surface.bg {
        Some(bg) => border.bg(bg),
        None => border,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::style::Color;
    use crate::core::text::Span;

    #[test]
    fn swap_colors_exchanges_foreground_and_background() {
        let style = Style::new().fg(Color::Red).bg(Color::Blue);
        let swapped = swap_colors(style);
        assert_eq!(swapped.fg, Some(Color::Blue));
        assert_eq!(swapped.bg, Some(Color::Red));
    }

    #[test]
    fn swap_colors_leaves_a_half_set_style_alone() {
        let only_fg = Style::new().fg(Color::Red);
        assert_eq!(swap_colors(only_fg), only_fg);
        let neither = Style::new();
        assert_eq!(swap_colors(neither), neither);
    }

    #[test]
    fn rebase_gives_every_span_the_base_background() {
        let line = Line::new(vec![
            Span::raw("a"),
            Span::styled("b", Style::new().fg(Color::Red)),
        ]);
        let based = rebase(line, Style::new().bg(Color::Blue));
        for span in &based.spans {
            assert_eq!(span.style.bg, Some(Color::Blue));
        }
    }

    #[test]
    fn rebase_keeps_a_span_s_own_foreground() {
        let line =
            Line::new(vec![Span::styled("b", Style::new().fg(Color::Red))]);
        let based = rebase(line, Style::new().bg(Color::Blue).fg(Color::Green));
        assert_eq!(based.spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn surface_of_keeps_only_the_background() {
        let style = Style::new().fg(Color::Red).bg(Color::Blue).bold();
        let surface = surface_of(style);
        assert_eq!(surface.bg, Some(Color::Blue));
        assert_eq!(surface.fg, None);
    }

    #[test]
    fn surface_of_a_background_less_style_is_empty() {
        assert_eq!(surface_of(Style::new().fg(Color::Red)), Style::new());
    }

    #[test]
    fn border_over_adopts_the_surface_background() {
        let border = Style::new().fg(Color::Red);
        let over = border_over(border, Style::new().bg(Color::Blue));
        assert_eq!(over.fg, Some(Color::Red));
        assert_eq!(over.bg, Some(Color::Blue));
    }

    #[test]
    fn border_over_a_transparent_surface_is_unchanged() {
        let border = Style::new().fg(Color::Red);
        assert_eq!(border_over(border, Style::new()), border);
    }
}
