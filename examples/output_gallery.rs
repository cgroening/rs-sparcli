//! A gallery of every static output widget. Run with
//! `cargo run --example output_gallery --features markup,fuzzy,pager`.
//!
//! Everything here renders to a fixed frame, so the output is deterministic
//! and pipe-friendly (`| cat`, `> file`, or `NO_COLOR=1` yield plain text).
//! Time-based widgets (spinner animation, progress, multi-progress, live,
//! pager) live in the `output_dynamic` example.

use sparcli::prelude::*;
use sparcli::{
    Cell, Column, Columns, Diff, ProgressBar, ProgressStyle, Spinner, Table,
    Thresholds,
};

#[cfg(feature = "markup")]
use sparcli::markup::markup_println;

fn main() -> Result<()> {
    styled_text()?;
    sections()?;
    alerts()?;
    panels()?;
    cards()?;
    tables()?;
    lists_and_trees()?;
    key_values_and_badges()?;
    progress_and_spinner()?;
    diff_and_columns()?;
    composition()?;
    Ok(())
}

/// Prints a section header rule.
fn section(title: &str) -> Result<()> {
    println!();
    Rule::with_title(title).align(Align::Left).print()?;
    println!();
    Ok(())
}

/// Styled spans, attributes, an OSC-8 hyperlink and optional markup.
fn styled_text() -> Result<()> {
    Rule::with_title("sparcli output gallery").print()?;
    println!();
    Rendered::new(vec![Line::new(vec![
        Span::styled("bold ", Style::new().bold()),
        Span::styled("dim ", Style::new().dim()),
        Span::styled("italic ", Style::new().italic()),
        Span::styled("red ", Style::new().fg(Color::Red)),
        Span::styled("on-blue ", Style::new().bg(Color::Blue)),
        Span::raw("link: "),
        Span::raw("sparcli").link("https://example.com/sparcli"),
    ])])
    .print()?;

    #[cfg(feature = "markup")]
    markup_println("[bold green]markup[/]: [#ff8800]orange[/] and `code`")?;
    Ok(())
}

/// Rules in all three alignments.
fn sections() -> Result<()> {
    section("Rules")?;
    Rule::with_title("left")
        .align(Align::Left)
        .border(BorderType::Single)
        .print()?;
    Rule::with_title("center").align(Align::Center).print()?;
    Rule::with_title("right")
        .align(Align::Right)
        .border(BorderType::Thick)
        .print()?;
    Ok(())
}

/// All five alert kinds.
fn alerts() -> Result<()> {
    section("Alerts")?;
    Alert::info("Informational message.").print()?;
    Alert::debug("Diagnostic detail.").print()?;
    Alert::success("Everything compiled.").print()?;
    Alert::warning("Low on disk space.").print()?;
    Alert::error("Connection refused.").print()?;
    Ok(())
}

/// Panels: borders, title/subtitle, fill and centered content.
fn panels() -> Result<()> {
    section("Panels")?;
    Panel::new("Rounded border (default).").print()?;
    Panel::new("Double border.")
        .border(BorderType::Double)
        .print()?;
    Panel::new("Centered, filled, fixed width.")
        .border(BorderType::Thick)
        .title(Title::new("Title"))
        .subtitle(Title::new("subtitle"))
        .content_align(Align::Center)
        .fill(Style::new().bg(Color::Indexed(236)))
        .width(40)
        .print()?;
    Ok(())
}

/// Cards: filled surfaces whose colors all come from one accent.
fn cards() -> Result<()> {
    section("Cards")?;
    Card::new("Every tone is derived from the theme accent.")
        .title("Default")
        .width(52)
        .print()?;
    println!();
    Card::new("A single accent sets title, surface, text and border.")
        .title("Derived from one color")
        .footer("border and footer, same accent")
        .accent(Color::from_hex("#a6e3a1").unwrap_or(Color::Green))
        .border(BorderType::Rounded)
        .width(52)
        .print()?;
    println!();
    Card::new("Centered, flat title, wider padding.")
        .title("Flat")
        .accent(Color::from_hex("#f9e2af").unwrap_or(Color::Yellow))
        .flat_title()
        .title_align(Align::Center)
        .content_align(Align::Center)
        .padding(Edges::symmetric(1, 3))
        .width(52)
        .print()?;
    Ok(())
}

/// Tables: colspan + striping, and footer + a wrapping column.
fn tables() -> Result<()> {
    section("Tables")?;
    Table::new()
        .title("Servers")
        .columns(["Name", "Region", "Status"])
        .row(["web-1", "eu-west", "online"])
        .row(["db-1", "eu-west", "online"])
        .row([Cell::new("maintenance window")
            .colspan(3)
            .align(Align::Center)])
        .striped(true)
        .print()?;
    println!();
    Table::new()
        .columns([
            Column::new("Item").align(Align::Left),
            Column::new("Note").wrap().max_width(24),
        ])
        .row([
            "alpha",
            "a long note that wraps across several lines nicely",
        ])
        .row(["beta", "short note"])
        .footer_row([Cell::new("2 items").colspan(2).align(Align::Right)])
        .border(BorderType::Ascii)
        .print()?;
    Ok(())
}

/// Lists in several marker styles, plus a tree.
fn lists_and_trees() -> Result<()> {
    section("Lists & trees")?;
    List::ordered(Marker::Number)
        .item("First step")
        .item_with("Second step", List::new().item("detail a").item("detail b"))
        .item("Third step")
        .print()?;
    println!();
    List::ordered(Marker::AlphaLower)
        .item("alpha")
        .item("beta")
        .print()?;
    List::ordered(Marker::RomanUpper)
        .item("one")
        .item("two")
        .print()?;
    println!();
    Tree::new()
        .node(
            TreeNode::new("project")
                .child(TreeNode::new("src").child(TreeNode::new("main.rs")))
                .child(TreeNode::new("Cargo.toml")),
        )
        .print()?;
    Ok(())
}

/// Key-value list and inline badges.
fn key_values_and_badges() -> Result<()> {
    section("Key-value & badges")?;
    KeyValue::new()
        .add("Version", "0.1.0")
        .add("License", "MIT")
        .print()?;
    println!();
    Rendered::new(vec![Line::new(vec![
        Badge::new("PASS")
            .style(Style::new().fg(Color::Green).bold())
            .span(),
        Span::raw(" "),
        Badge::new("WARN")
            .style(Style::new().fg(Color::Yellow).bold())
            .span(),
        Span::raw(" "),
        Badge::new("v0.1").caps("(", ")").span(),
    ])])
    .print()?;
    Ok(())
}

/// Progress bars in every style, a threshold bar, and a static spinner frame.
fn progress_and_spinner() -> Result<()> {
    section("Progress & spinner")?;
    let styles = [
        ("block", ProgressStyle::Block),
        ("ascii", ProgressStyle::Ascii),
        ("line", ProgressStyle::Line),
        ("shaded", ProgressStyle::Shaded),
    ];
    for (label, style) in styles {
        ProgressBar::new()
            .style(style)
            .label(label)
            .width(20)
            .bar(6.0, 10.0)
            .print()?;
    }
    let thresholds = Thresholds {
        mid: 0.5,
        high: 0.8,
        low_color: Color::Green,
        mid_color: Color::Yellow,
        high_color: Color::Red,
    };
    ProgressBar::new()
        .thresholds(thresholds)
        .label("load")
        .width(20)
        .bar(9.0, 10.0)
        .print()?;
    Spinner::new("spinner (static frame)").frame().print()?;
    Ok(())
}

/// A colored diff and a two-column layout.
fn diff_and_columns() -> Result<()> {
    section("Diff & columns")?;
    Diff::new(
        "line one\nline two\nline three",
        "line one\nline 2\nline three",
    )
    .print()?;
    println!();
    let left = Panel::new("left").render(20);
    let right = Panel::new("right").render(20);
    Columns::new()
        .add_rendered(left)
        .add_rendered(right)
        .gap(3)
        .separator(BorderType::Single)
        .print()?;
    Ok(())
}

/// Composition helpers: align, pad and vstack.
fn composition() -> Result<()> {
    section("Composition (align / pad / vstack)")?;
    let block = Rendered::new(vec![Line::raw("aligned right")]);
    align(&block, 30, Align::Right).print()?;
    println!();
    pad(&Rendered::new(vec![Line::raw("padded")]), Edges::all(1)).print()?;
    println!();
    let a = Rendered::new(vec![Line::raw("first block")]);
    let b = Rendered::new(vec![Line::raw("second block")]);
    vstack(&[a, b], 1).print()?;
    Ok(())
}
