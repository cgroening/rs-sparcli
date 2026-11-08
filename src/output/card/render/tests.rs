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
