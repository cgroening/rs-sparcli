//! The render pipeline of a [`Card`]: capabilities, regions and assembly.
//!
//! The pipeline resolves the terminal capabilities and the palette, cuts the
//! card into its three regions, and stacks their rows. The rows themselves are
//! built in [`super::rows`], the style math lives in [`super::style`].

use crate::core::border::BorderType;
use crate::core::geometry::Position;
use crate::core::render::{Renderable, Rendered};
use crate::core::terminal::{ColorSupport, UNCONSTRAINED_WIDTH, color_support};
use crate::core::theme::theme;
use crate::output::card::Card;
use crate::output::card::palette::{self, CardStyles};
use crate::output::card::rows::{Region, border_columns, edge_row, push_block};
use crate::output::card::style::{border_over, surface_of};

/// The terminal capabilities a card renders against.
///
/// Bundling them keeps the render seam at one parameter and makes both
/// degradations testable without touching the global theme or environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RenderCaps {
    /// How much color the terminal supports.
    pub support: ColorSupport,
    /// Whether Unicode glyphs may be used.
    pub unicode: bool,
}

impl RenderCaps {
    /// Returns the capabilities of a fully capable terminal.
    #[cfg(test)]
    pub(crate) fn truecolor() -> Self {
        Self {
            support: ColorSupport::TrueColor,
            unicode: true,
        }
    }
}

impl Renderable for Card {
    fn render(&self, max_width: u16) -> Rendered {
        let caps = RenderCaps {
            support: color_support(),
            unicode: theme().unicode,
        };
        self.render_with(max_width, caps)
    }
}

impl Card {
    /// Renders the card for explicit terminal capabilities.
    ///
    /// [`Renderable::render`] detects them from the environment and the theme;
    /// this is the seam that keeps both degradations testable, since under
    /// `cargo test` standard output is not a terminal and detection would
    /// always report [`ColorSupport::None`].
    pub(crate) fn render_with(
        &self,
        max_width: u16,
        caps: RenderCaps,
    ) -> Rendered {
        let border = self.effective_border(caps);
        let border_columns = border_columns(border);
        let outer = self.outer_width(max_width, border_columns) as usize;
        if outer <= border_columns {
            return Rendered::empty();
        }
        let styles = self.resolved_styles(caps.support);
        let surface = outer - border_columns;
        self.assemble(&self.regions(&styles, surface, border), border)
    }

    /// Resolves the card's outer width in columns.
    ///
    /// A card fills the width it is given, which is what makes it read as a
    /// panel rather than a label. Without a terminal there is no width to
    /// fill, so an unconstrained card falls back to its natural content width
    /// instead of stretching to [`UNCONSTRAINED_WIDTH`]. An explicit
    /// [`Card::width`] always wins, capped at what is available.
    fn outer_width(&self, max_width: u16, border_columns: usize) -> u16 {
        if let Some(width) = self.opts.width {
            return width.min(max_width);
        }
        if max_width != UNCONSTRAINED_WIDTH {
            return max_width;
        }
        let natural = self.natural_width() + border_columns;
        u16::try_from(natural).unwrap_or(UNCONSTRAINED_WIDTH)
    }

    /// Returns the widest slot's content width plus that slot's padding.
    ///
    /// The three slots pad independently, so the widest row is not simply the
    /// widest text: a narrow title with wide padding can still be the widest.
    fn natural_width(&self) -> usize {
        let optional = [
            (self.title.as_ref(), self.opts.title_padding),
            (self.footer.as_ref(), self.opts.footer_padding),
        ];
        let content =
            self.content.width() + self.opts.padding.horizontal() as usize;
        optional
            .into_iter()
            .filter_map(|(block, padding)| {
                block.map(|b| b.width() + padding.horizontal() as usize)
            })
            .chain(std::iter::once(content))
            .max()
            .unwrap_or(0)
    }

    /// Resolves the border type actually drawn.
    ///
    /// A tall border is built from half blocks whose bar only reads against a
    /// contrasting surface, so it needs both truecolor and Unicode glyphs.
    /// Without either it degrades to the heavy frame its glyph set already
    /// maps to. Other border types are returned unchanged, so a card does not
    /// diverge from [`Panel`](crate::output::panel::Panel) here.
    fn effective_border(&self, caps: RenderCaps) -> BorderType {
        if !self.opts.border.is_tall() {
            return self.opts.border;
        }
        if !caps.unicode {
            return BorderType::Ascii;
        }
        if caps.support != ColorSupport::TrueColor {
            return BorderType::Thick;
        }
        BorderType::Tall
    }

    /// Builds all rows of the card, top to bottom.
    fn assemble(&self, regions: &CardRegions, border: BorderType) -> Rendered {
        let mut lines = Vec::new();
        let has_border = !border.is_none();
        if has_border {
            lines.push(edge_row(regions.top(), Position::Top));
        }
        if let Some(title) = &self.title {
            push_block(&mut lines, &title.lines, &regions.title);
        }
        push_block(&mut lines, &self.content.lines, &regions.content);
        if let Some(footer) = &self.footer {
            push_block(&mut lines, &footer.lines, &regions.footer);
        }
        if has_border {
            lines.push(edge_row(regions.bottom(), Position::Bottom));
        }
        Rendered::new(lines)
    }

    /// Derives the palette and applies the per-slot overrides on top.
    ///
    /// The surface background is the single source for both the blank padding
    /// cells and the text cells, so a custom [`Card::fill`] reaches the body
    /// text as well; an explicit [`Card::content_style`] still wins over it.
    fn resolved_styles(&self, support: ColorSupport) -> CardStyles {
        let mut styles = palette::derive(self.opts.accent, support);
        styles.fill = styles.fill.patch(self.opts.fill);
        styles.content.bg = styles.fill.bg;
        if self.opts.flat_title {
            styles.title.bg = styles.fill.bg;
        }
        if self.opts.flat_footer {
            styles.footer.bg = styles.fill.bg;
        }
        styles.border = styles.border.patch(self.opts.border_style);
        styles.title = styles.title.patch(self.opts.title_style);
        styles.content = styles.content.patch(self.opts.content_style);
        styles.footer = styles.footer.patch(self.opts.footer_style);
        styles
    }

    /// Builds the three regions a card is made of.
    fn regions(
        &self,
        styles: &CardStyles,
        surface: usize,
        border: BorderType,
    ) -> CardRegions {
        let title_surface = surface_of(styles.title);
        let footer_surface = surface_of(styles.footer);
        CardRegions {
            title: Region {
                width: surface,
                padding: self.opts.title_padding,
                align: self.opts.title_align,
                text: styles.title,
                surface: title_surface,
                border: border_over(styles.border, title_surface),
                border_type: border,
                wrap: self.opts.wrap,
            },
            content: Region {
                width: surface,
                padding: self.opts.padding,
                align: self.opts.content_align,
                text: styles.content,
                surface: styles.fill,
                border: border_over(styles.border, styles.fill),
                border_type: border,
                wrap: self.opts.wrap,
            },
            footer: Region {
                width: surface,
                padding: self.opts.footer_padding,
                align: self.opts.footer_align,
                text: styles.footer,
                surface: footer_surface,
                border: border_over(styles.border, footer_surface),
                border_type: border,
                wrap: self.opts.wrap,
            },
            has_title: self.title.is_some(),
            has_footer: self.footer.is_some(),
        }
    }
}

/// The three regions of a card plus which optional ones are present.
struct CardRegions {
    /// The title row region.
    title: Region,
    /// The body content region.
    content: Region,
    /// The footer row region.
    footer: Region,
    /// Whether a title row is rendered.
    has_title: bool,
    /// Whether a footer row is rendered.
    has_footer: bool,
}

impl CardRegions {
    /// Returns the region the top border sits against.
    fn top(&self) -> &Region {
        if self.has_title {
            &self.title
        } else {
            &self.content
        }
    }

    /// Returns the region the bottom border sits against.
    fn bottom(&self) -> &Region {
        if self.has_footer {
            &self.footer
        } else {
            &self.content
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::geometry::{Align, Edges};
    use crate::core::style::{Color, Style};
    use crate::core::text::{Line, Span, Text};

    /// Renders a card at truecolor, bypassing the environment detection that
    /// would report [`ColorSupport::None`] under `cargo test`.
    fn render(card: Card, width: u16) -> Rendered {
        card.render_with(width, RenderCaps::truecolor())
    }

    /// Returns the plain text of every row.
    /// Returns the background of a row, taken from its first span.
    fn row_background(rendered: &Rendered, row: usize) -> Option<Color> {
        rendered.lines[row]
            .spans
            .first()
            .and_then(|span| span.style.bg)
    }

    /// Builds an explicit capability set.
    fn caps(support: ColorSupport, unicode: bool) -> RenderCaps {
        RenderCaps { support, unicode }
    }

    /// Returns the left and right border glyph spans of a row.
    fn edges(rendered: &Rendered, row: usize) -> (&Span, &Span) {
        let spans = &rendered.lines[row].spans;
        let left = spans.first().expect("a bordered row has a left glyph");
        let right = spans.last().expect("a bordered row has a right glyph");
        (left, right)
    }

    #[test]
    fn card_fills_the_full_max_width_by_default() {
        let out = render(Card::new("hi"), 40);
        for line in &out.lines {
            assert_eq!(line.width(), 40, "{:?}", line.plain());
        }
    }

    #[test]
    fn every_cell_of_the_surface_carries_a_background() {
        // A background paints only its own span, so a single unstyled span
        // punches a transparent hole through the card. This catches the
        // padding columns, the alignment slack, the border seam and content
        // spans that were not rebased onto the surface - all at once.
        let card = Card::new("body")
            .title("Heading")
            .footer("footer")
            .border(BorderType::Rounded);
        let out = render(card, 40);
        for line in &out.lines {
            for span in &line.spans {
                assert!(
                    span.style.bg.is_some(),
                    "uncolored span {:?} in {:?}",
                    span.content,
                    line.plain()
                );
            }
        }
    }

    #[test]
    fn content_spans_keep_their_own_foreground() {
        let styled =
            Line::new(vec![Span::styled("alert", Style::new().fg(Color::Red))]);
        let out = render(Card::from_rendered(Rendered::new(vec![styled])), 30);
        let span = out
            .lines
            .iter()
            .flat_map(|line| &line.spans)
            .find(|span| span.content == "alert")
            .expect("the content span survives rendering");
        assert_eq!(span.style.fg, Some(Color::Red));
        assert!(span.style.bg.is_some(), "it still gains the surface");
    }

    #[test]
    fn title_row_has_its_own_background() {
        // Rows: title, blank, content, blank.
        let out = render(Card::new("body").title("Heading"), 30);
        assert_ne!(row_background(&out, 0), row_background(&out, 2));
    }

    #[test]
    fn flat_title_shares_the_content_background() {
        let out = render(Card::new("body").title("Heading").flat_title(), 30);
        assert_eq!(row_background(&out, 0), row_background(&out, 2));
    }

    #[test]
    fn footer_row_is_last_and_has_its_own_background() {
        let out = render(Card::new("body").footer("footer"), 30);
        let last = out.lines.len() - 1;
        assert!(out.plain_lines()[last].contains("footer"));
        assert_ne!(row_background(&out, last), row_background(&out, 1));
    }

    #[test]
    fn border_glyphs_carry_the_adjacent_surface_background() {
        let card = Card::new("body")
            .title("Heading")
            .border(BorderType::Single);
        let out = render(card, 30);
        // The top edge sits against the title row, not the content surface.
        assert_eq!(row_background(&out, 0), row_background(&out, 1));
        let content_row = &out.lines[3];
        let glyph = content_row.spans.first().expect("a bordered row");
        assert_eq!(glyph.content, "│");
        assert_eq!(glyph.style.bg, row_background(&out, 3));
    }

    #[test]
    fn no_divider_sits_between_title_and_content() {
        // With no vertical content padding the title and body are adjacent:
        // top border, title, content, bottom border.
        let card = Card::new("body")
            .title("Heading")
            .padding(Edges::symmetric(0, 1))
            .border(BorderType::Single);
        let lines = render(card, 30).plain_lines();
        assert_eq!(lines.len(), 4);
        assert!(lines[1].contains("Heading"));
        assert!(lines[2].contains("body"));
    }

    #[test]
    fn wrap_is_enabled_by_default() {
        // Surface 20 minus one padding column per side leaves 18 columns, so
        // the text wraps into "alpha beta gamma" and "delta epsilon".
        let card = Card::new("alpha beta gamma delta epsilon").width(20);
        let lines = render(card, 80).plain_lines();
        assert_eq!(lines.len(), 4, "two wrapped rows plus two blanks");
        assert!(lines[1].contains("alpha beta gamma"));
        assert!(lines[2].contains("delta epsilon"));
        assert!(!lines.iter().any(|line| line.contains('…')));
    }

    #[test]
    fn wrap_off_truncates_with_an_ellipsis() {
        let card = Card::new("alpha beta gamma delta epsilon")
            .width(20)
            .wrap(false);
        let lines = render(card, 80).plain_lines();
        assert_eq!(lines.len(), 3, "one truncated row plus two blanks");
        assert!(lines[1].contains('…'));
    }

    #[test]
    fn fixed_width_is_clamped_to_max_width() {
        let out = render(Card::new("hi").width(200), 80);
        for line in &out.lines {
            assert_eq!(line.width(), 80);
        }
    }

    #[test]
    fn narrow_width_is_honored() {
        let out = render(Card::new("hi").width(20), 80);
        for line in &out.lines {
            assert_eq!(line.width(), 20);
        }
    }

    #[test]
    fn an_unconstrained_card_shrinks_to_its_content() {
        // Piped output has no terminal width to fill, so the card must not
        // stretch to UNCONSTRAINED_WIDTH; it lays out at its natural width.
        let out = render(Card::new("body"), UNCONSTRAINED_WIDTH);
        // "body" plus the default single padding column on each side.
        assert_eq!(out.lines[0].width(), 6);
        assert!(out.plain().contains("body"));
    }

    #[test]
    fn an_unconstrained_card_still_honors_an_explicit_width() {
        let out = render(Card::new("body").width(20), UNCONSTRAINED_WIDTH);
        for line in &out.lines {
            assert_eq!(line.width(), 20);
        }
    }

    #[test]
    fn an_unconstrained_card_measures_its_widest_slot() {
        let card = Card::new("body").title("a much longer heading");
        let out = render(card, UNCONSTRAINED_WIDTH);
        assert_eq!(out.lines[0].width(), "a much longer heading".len() + 2);
    }

    #[test]
    fn degenerate_width_renders_nothing() {
        let card = Card::new("hi").width(2).border(BorderType::Single);
        assert_eq!(render(card, 80), Rendered::empty());
    }

    #[test]
    fn the_two_padding_levels_are_independent() {
        let card = Card::new("body")
            .title("Heading")
            .title_padding(Edges::all(0))
            .padding(Edges::all(2));
        let lines = render(card, 30).plain_lines();
        assert_eq!(lines.len(), 6, "title plus two blanks around one body row");
        assert!(lines[0].starts_with("Heading"));
        assert!(lines[3].starts_with("  body"));
    }

    #[test]
    fn title_and_content_align_independently() {
        let card = Card::new("body")
            .title("Heading")
            .title_align(Align::Right)
            .content_align(Align::Center)
            .width(20);
        // Text area 18: the right-aligned title ends one padding column short
        // of the edge, the centered body sits behind seven slack columns plus
        // the one padding column.
        let lines = render(card, 30).plain_lines();
        assert!(lines[0].ends_with("Heading "), "{:?}", lines[0]);
        assert_eq!(lines[2], "        body        ");
    }

    #[test]
    fn ansi16_renders_without_backgrounds_but_keeps_the_geometry() {
        let card = Card::new("body").title("Heading");
        let flat = card.render_with(30, caps(ColorSupport::Ansi16, true));
        for line in &flat.lines {
            assert_eq!(line.width(), 30);
            for span in &line.spans {
                assert_eq!(span.style.bg, None);
            }
        }
    }

    #[test]
    fn content_wraps_to_the_content_area_not_the_surface() {
        // Surface 20, horizontal padding 8, so the text area is 12 columns.
        let card = Card::new("aaaa bbbb cccc")
            .width(20)
            .padding(Edges::symmetric(0, 4));
        let lines = render(card, 30).plain_lines();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("aaaa bbbb"));
        assert!(lines[1].contains("cccc"));
    }

    #[test]
    fn a_custom_fill_reaches_the_body_text() {
        // The surface is one source: blank padding cells and text cells must
        // not drift apart when the fill is overridden.
        let card = Card::new("body").fill(Style::new().bg(Color::Indexed(236)));
        let out = render(card, 30);
        assert_eq!(row_background(&out, 0), Some(Color::Indexed(236)));
        assert_eq!(row_background(&out, 1), Some(Color::Indexed(236)));
    }

    #[test]
    fn a_style_override_patches_rather_than_replaces() {
        use crate::core::style::Attribute;

        let card = Card::new("body")
            .title("Heading")
            .title_style(Style::new().bold());
        let out = render(card, 30);
        // The first span is a padding cell, which carries only the surface;
        // the override applies to the text span.
        let title = out.lines[0]
            .spans
            .iter()
            .find(|span| span.content == "Heading")
            .expect("the title text is rendered as its own span");
        assert!(title.style.attrs.contains(Attribute::BOLD));
        assert!(title.style.bg.is_some(), "the derived surface survives");
        assert!(title.style.fg.is_some(), "the derived text color survives");
    }

    #[test]
    fn a_card_without_a_title_starts_with_the_content_surface() {
        let out = render(Card::new("body").border(BorderType::Single), 30);
        assert_eq!(row_background(&out, 0), row_background(&out, 1));
    }

    #[test]
    fn tall_side_bars_are_quarter_blocks_on_opposite_edges() {
        // `▊` inks the left three quarters, so only swapping its colors turns
        // the remaining quarter into the bar - Unicode defines no
        // right-aligned quarter block.
        let card = Card::new("body").border(BorderType::Tall);
        let out = render(card, 30);
        let (left, right) = edges(&out, 1);
        assert_eq!(left.content, "▎");
        assert_eq!(right.content, "▊");
        assert_eq!(left.style.fg, right.style.bg, "the bar color");
        assert_eq!(left.style.bg, right.style.fg, "the surface color");
        assert_ne!(left.style.fg, left.style.bg);
    }

    #[test]
    fn tall_corners_are_closed_by_a_full_width_line() {
        // The horizontal line runs across the corner cells too, and sits on
        // the inner side of its row, so it touches the side bar of the
        // adjoining row instead of starting a cell away from it.
        let card = Card::new("body").border(BorderType::Tall);
        let lines = render(card, 20).plain_lines();
        let last = lines.len() - 1;
        assert_eq!(lines[0], "▁".repeat(20));
        assert_eq!(lines[last], "▔".repeat(20));
    }

    #[test]
    fn tall_horizontal_rows_carry_no_surface() {
        // Otherwise a band of card color would sit above the top line, making
        // the frame look like it starts one row too early.
        let card = Card::new("body").border(BorderType::Tall);
        let out = render(card, 30);
        for span in &out.lines[0].spans {
            assert_eq!(span.style.bg, None);
            assert!(span.style.fg.is_some(), "the line stays visible");
        }
    }

    #[test]
    fn tall_keeps_a_background_under_every_body_cell() {
        // The horizontal rows are deliberately transparent; between them the
        // surface must still be gapless.
        let card = Card::new("body")
            .title("Heading")
            .footer("footer")
            .border(BorderType::Tall);
        let out = render(card, 30);
        let body = &out.lines[1..out.lines.len() - 1];
        for line in body {
            for span in &line.spans {
                assert!(
                    span.style.bg.is_some(),
                    "uncolored span {:?} in {:?}",
                    span.content,
                    line.plain()
                );
            }
        }
    }

    #[test]
    fn tall_side_bars_take_the_surface_of_their_own_region() {
        // The title bar and the body sit on different surfaces, and the side
        // glyph of each row has to follow its own row.
        let card = Card::new("body").title("Heading").border(BorderType::Tall);
        let out = render(card, 30);
        assert_ne!(row_background(&out, 1), row_background(&out, 3));
    }

    #[test]
    fn tall_degrades_to_thick_without_truecolor() {
        let card = Card::new("body").border(BorderType::Tall);
        let out = card.render_with(30, caps(ColorSupport::Ansi16, true));
        let lines: Vec<String> = out.lines.iter().map(Line::plain).collect();
        assert!(lines[0].starts_with('┏') && lines[0].ends_with('┓'));
        assert!(lines[1].starts_with('┃'));
        assert!(!lines.iter().any(|line| line.contains('▎')));
        for line in &out.lines {
            assert_eq!(line.width(), 30, "the geometry is unchanged");
        }
    }

    #[test]
    fn tall_degrades_to_ascii_without_unicode() {
        let card = Card::new("body").border(BorderType::Tall);
        let out = card.render_with(30, caps(ColorSupport::TrueColor, false));
        let lines: Vec<String> = out.lines.iter().map(Line::plain).collect();
        assert!(lines[0].starts_with('+') && lines[0].ends_with('+'));
        assert!(lines[1].starts_with('|'));
    }

    #[test]
    fn a_non_tall_border_uses_one_glyph_on_both_sides() {
        // Only a tall border distinguishes its two side glyphs; every other
        // border type keeps the single `vertical` glyph on both sides.
        let card = Card::new("body").border(BorderType::Rounded);
        let out = render(card, 30);
        let (left, right) = edges(&out, 1);
        assert_eq!(left.content, "│");
        assert_eq!(right.content, "│");
        assert_eq!(left.style, right.style);
    }

    #[test]
    fn a_multiline_title_produces_one_row_per_line() {
        let card = Card::new("body").title(Text::raw("first\nsecond"));
        let lines = render(card, 30).plain_lines();
        assert!(lines[0].contains("first"));
        assert!(lines[1].contains("second"));
    }
}
