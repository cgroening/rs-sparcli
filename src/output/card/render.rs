//! The render pipeline of a [`Card`]: regions, rows and the gap-free surface.
//!
//! A [`Rendered`] block is a list of styled spans, not a cell grid, so a
//! background paints only the characters of its own span. Every row is
//! therefore assembled through [`region_row`], which materializes the padding
//! columns, the alignment slack and the border glyphs with the region's
//! background - and rebases the content spans onto it.

use crate::core::border::BorderType;
use crate::core::geometry::{Align, Edges, Position};
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::terminal::{ColorSupport, color_support};
use crate::core::text::{Line, Span};
use crate::core::width::{truncate_line, wrap_line};
use crate::output::card::Card;
use crate::output::card::palette::{self, CardStyles};
use crate::output::layout::{blank_line, pad_line};

/// Columns consumed by the two vertical border glyphs.
const BORDER_COLUMNS: usize = 2;
/// Marker appended to content that had to be truncated.
const ELLIPSIS: &str = "…";

impl Renderable for Card {
    fn render(&self, max_width: u16) -> Rendered {
        self.render_with(max_width, color_support())
    }
}

/// The geometry and styling one row of a card needs.
struct Region {
    /// Width of the surface between the borders, in columns.
    width: usize,
    /// Padding between the surface edge and the text.
    padding: Edges,
    /// Horizontal alignment of the text.
    align: Align,
    /// Base style the row's text is rebased onto.
    text: Style,
    /// Background of every cell in the row.
    surface: Style,
    /// Style of the border glyphs, already carrying the surface background.
    border: Style,
    /// Border type; [`BorderType::None`] omits the vertical glyphs.
    border_type: BorderType,
    /// Whether overlong lines wrap instead of being truncated.
    wrap: bool,
}

impl Region {
    /// Returns the width available to text, after the horizontal padding.
    fn area(&self) -> usize {
        self.width
            .saturating_sub(self.padding.horizontal() as usize)
    }
}

impl Card {
    /// Renders the card for an explicit color-support level.
    ///
    /// [`Renderable::render`] detects the level from the environment; this is
    /// the seam that keeps the decision testable, since under `cargo test`
    /// standard output is not a terminal and detection would always report
    /// [`ColorSupport::None`].
    pub(crate) fn render_with(
        &self,
        max_width: u16,
        support: ColorSupport,
    ) -> Rendered {
        let outer = self.opts.width.map_or(max_width, |w| w.min(max_width));
        let border_columns = self.border_columns();
        let outer = outer as usize;
        if outer <= border_columns {
            return Rendered::empty();
        }
        let styles = self.resolved_styles(support);
        let surface = outer - border_columns;
        self.assemble(&self.regions(&styles, surface))
    }

    /// Builds all rows of the card, top to bottom.
    fn assemble(&self, regions: &CardRegions) -> Rendered {
        let mut lines = Vec::new();
        let has_border = !self.opts.border.is_none();
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

    /// Returns the columns the vertical border glyphs consume.
    fn border_columns(&self) -> usize {
        if self.opts.border.is_none() {
            0
        } else {
            BORDER_COLUMNS
        }
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
    fn regions(&self, styles: &CardStyles, surface: usize) -> CardRegions {
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
                border_type: self.opts.border,
                wrap: self.opts.wrap,
            },
            content: Region {
                width: surface,
                padding: self.opts.padding,
                align: self.opts.content_align,
                text: styles.content,
                surface: styles.fill,
                border: border_over(styles.border, styles.fill),
                border_type: self.opts.border,
                wrap: self.opts.wrap,
            },
            footer: Region {
                width: surface,
                padding: self.opts.footer_padding,
                align: self.opts.footer_align,
                text: styles.footer,
                surface: footer_surface,
                border: border_over(styles.border, footer_surface),
                border_type: self.opts.border,
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

/// Appends one region's padding rows, text rows and padding rows.
fn push_block(lines: &mut Vec<Line>, source: &[Line], region: &Region) {
    for _ in 0..region.padding.top {
        lines.push(blank_row(region));
    }
    let area = region.area();
    if area > 0 {
        for line in source {
            for fitted in fit(line, area, region.wrap) {
                lines.push(region_row(fitted, region));
            }
        }
    }
    for _ in 0..region.padding.bottom {
        lines.push(blank_row(region));
    }
}

/// Fits one line into `area` columns, either by wrapping or by truncating.
fn fit(line: &Line, area: usize, wrap: bool) -> Vec<Line> {
    if wrap {
        return wrap_line(line, area);
    }
    vec![truncate_line(line, area, ELLIPSIS)]
}

/// Builds one full-width row: border glyph, padding, aligned text, padding,
/// border glyph - every cell carrying the region's background.
fn region_row(line: Line, region: &Region) -> Line {
    let mut spans = Vec::new();
    push_vertical(&mut spans, region);
    push_padding(&mut spans, region.padding.left, region.surface);
    let rebased = rebase(line, region.text);
    let padded = pad_line(rebased, region.area(), region.align, region.surface);
    spans.extend(padded.spans);
    push_padding(&mut spans, region.padding.right, region.surface);
    push_vertical(&mut spans, region);
    Line::new(spans)
}

/// Builds an empty row of the region's surface.
fn blank_row(region: &Region) -> Line {
    let mut spans = Vec::new();
    push_vertical(&mut spans, region);
    spans.extend(blank_line(region.width, region.surface).spans);
    push_vertical(&mut spans, region);
    Line::new(spans)
}

/// Builds the top or bottom border row of a region.
fn edge_row(region: &Region, position: Position) -> Line {
    let chars = region.border_type.chars();
    let (left, right) = match position {
        Position::Top => (chars.top_left, chars.top_right),
        Position::Bottom => (chars.bottom_left, chars.bottom_right),
    };
    let mut content = String::with_capacity(region.width + BORDER_COLUMNS);
    content.push(left);
    for _ in 0..region.width {
        content.push(chars.horizontal);
    }
    content.push(right);
    Line::new(vec![Span::styled(content, region.border)])
}

/// Appends a vertical border glyph, unless the card is unframed.
fn push_vertical(spans: &mut Vec<Span>, region: &Region) {
    if region.border_type.is_none() {
        return;
    }
    let glyph = region.border_type.chars().vertical;
    spans.push(Span::styled(glyph.to_string(), region.border));
}

/// Appends `columns` padding cells carrying the surface background.
fn push_padding(spans: &mut Vec<Span>, columns: u16, surface: Style) {
    if columns == 0 {
        return;
    }
    spans.push(Span::styled(" ".repeat(columns as usize), surface));
}

/// Rebases every span of `line` onto `base`.
///
/// Without this the content spans keep their own (usually unset) background and
/// punch transparent holes through the surface - invisible in the plain text,
/// visible in the terminal.
fn rebase(line: Line, base: Style) -> Line {
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
fn surface_of(style: Style) -> Style {
    match style.bg {
        Some(bg) => Style::new().bg(bg),
        None => Style::new(),
    }
}

/// Returns the border style carrying the given surface's background, so the
/// glyphs do not leave a transparent seam along the card's edges.
fn border_over(border: Style, surface: Style) -> Style {
    match surface.bg {
        Some(bg) => border.bg(bg),
        None => border,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::style::Color;
    use crate::core::text::Text;

    /// Renders a card at truecolor, bypassing the environment detection that
    /// would report [`ColorSupport::None`] under `cargo test`.
    fn render(card: Card, width: u16) -> Rendered {
        card.render_with(width, ColorSupport::TrueColor)
    }

    /// Returns the plain text of every row.
    fn plain(rendered: &Rendered) -> Vec<String> {
        rendered.lines.iter().map(Line::plain).collect()
    }

    /// Returns the background of a row, taken from its first span.
    fn row_background(rendered: &Rendered, row: usize) -> Option<Color> {
        rendered.lines[row]
            .spans
            .first()
            .and_then(|span| span.style.bg)
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
        assert!(plain(&out)[last].contains("footer"));
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
        let lines = plain(&render(card, 30));
        assert_eq!(lines.len(), 4);
        assert!(lines[1].contains("Heading"));
        assert!(lines[2].contains("body"));
    }

    #[test]
    fn wrap_is_enabled_by_default() {
        // Surface 20 minus one padding column per side leaves 18 columns, so
        // the text wraps into "alpha beta gamma" and "delta epsilon".
        let card = Card::new("alpha beta gamma delta epsilon").width(20);
        let lines = plain(&render(card, 80));
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
        let lines = plain(&render(card, 80));
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
        let lines = plain(&render(card, 30));
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
        let lines = plain(&render(card, 30));
        assert!(lines[0].ends_with("Heading "), "{:?}", lines[0]);
        assert_eq!(lines[2], "        body        ");
    }

    #[test]
    fn ansi16_renders_without_backgrounds_but_keeps_the_geometry() {
        let card = Card::new("body").title("Heading");
        let flat = card.render_with(30, ColorSupport::Ansi16);
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
        let lines = plain(&render(card, 30));
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
    fn a_multiline_title_produces_one_row_per_line() {
        let card = Card::new("body").title(Text::raw("first\nsecond"));
        let lines = plain(&render(card, 30));
        assert!(lines[0].contains("first"));
        assert!(lines[1].contains("second"));
    }
}
