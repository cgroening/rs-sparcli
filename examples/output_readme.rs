//! The output hero collage for the README, mirroring the C sparcli demo:
//! a titled hero panel, a three-column dashboard (table | list+tree+rules |
//! key-value+badges+columns) and a progress bar, in the same colors.
//!
//! `cargo run --example output_readme --features markup`

use sparcli::prelude::*;
use sparcli::{
    Badge, Cell, Column, Columns, KeyValue, List, Marker, Panel, ProgressBar,
    Rule, Table, Tree, TreeNode,
};

fn main() -> Result<()> {
    println!();
    hero()?;
    println!();
    dashboard()?;
    println!();
    progress()?;
    Ok(())
}

/// A blank one-line block, used as a vertical spacer in `vstack`.
fn blank() -> Rendered {
    Rendered::new(vec![Line::default()])
}

/// The top hero panel with styled, centered text.
fn hero() -> Result<()> {
    let cyan = Style::new().fg(Color::Cyan).bold();
    let on_magenta = Style::new().fg(Color::Black).bg(Color::Magenta).bold();
    let yellow = Style::new().fg(Color::Yellow).bold();
    let body = Text::new(vec![
        Line::new(vec![
            Span::raw("A native Rust library for "),
            Span::styled("styled terminal output", cyan),
            Span::raw(" and "),
            Span::styled("input", on_magenta),
            Span::raw(" - panels, tables, trees, lists,"),
        ]),
        Line::new(vec![
            Span::raw("key-value pairs, badges, rules, "),
            Span::styled("columns", cyan),
            Span::raw(" and progress bars, with Rich-compatible "),
            Span::styled("[markup]", yellow),
            Span::raw("."),
        ]),
    ]);
    Panel::new(body)
        .border(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Cyan))
        .title(
            Title::new(" sparcli ")
                .align(Align::Center)
                .style(Style::new().fg(Color::Cyan).bold()),
        )
        .content_align(Align::Center)
        .width(96)
        .print()
}

/// The three-column dashboard row.
fn dashboard() -> Result<()> {
    Columns::new()
        .add_rendered(overview_table())
        .add_rendered(middle_column())
        .add_rendered(right_column())
        .gap(3)
        .separator(BorderType::Single)
        .print()
}

/// Left column: the striped, row-separated "Overview" table.
fn overview_table() -> Rendered {
    let status = |text: &str, color: Color| {
        Cell::new(Text::styled(text, Style::new().fg(color).bold()))
    };
    Table::new()
        .title("Overview")
        .title_style(Style::new().fg(Color::Magenta).bold())
        .border(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Magenta))
        .header_style(Style::new().fg(Color::Cyan).bold())
        .striped(true)
        .stripe_style(Style::new().bg(Color::Rgb(40, 40, 60)))
        .row_separators(true)
        .columns([
            Column::new("Service"),
            Column::new("Status").align(Align::Center),
            Column::new("Uptime").align(Align::Right),
        ])
        .row([
            Cell::new("api-gateway"),
            status("● OK", Color::Green),
            Cell::new("99.98%"),
        ])
        .row([
            Cell::new("auth"),
            status("● OK", Color::Green),
            Cell::new("99.91%"),
        ])
        .row([
            Cell::new("billing"),
            status("● WARN", Color::Yellow),
            Cell::new("97.40%"),
        ])
        .row([
            Cell::new("scheduler"),
            status("● FAIL", Color::Red),
            Cell::new("82.10%"),
        ])
        .render(40)
}

/// Middle column: a numbered list above a tree beside titled rules.
fn middle_column() -> Rendered {
    let list = List::ordered(Marker::Number)
        .marker_style(Style::new().fg(Color::Yellow).bold())
        .item("Compose widgets side by side")
        .item("Capture, pad, and align them")
        .item("Style with Rich-style markup")
        .item("Render to any UTF-8 terminal")
        .render(36);

    let tree = Tree::new()
        .dashes(2)
        .connector_style(Style::new().fg(Color::Cyan))
        .node(
            TreeNode::new("project/")
                .child(
                    TreeNode::new("api/")
                        .child(TreeNode::new("routes.c"))
                        .child(TreeNode::new("auth.c")),
                )
                .child(TreeNode::new("worker.c"))
                .child(TreeNode::new("store.c")),
        )
        .render(18);

    let rules = vstack(
        &[
            rule("Pipeline", BorderType::Single, Color::Magenta),
            blank(),
            rule("Workers", BorderType::Double, Color::Green),
            blank(),
            rule("Storage", BorderType::Thick, Color::Yellow),
            blank(),
        ],
        0,
    );

    let tree_and_rules = Columns::new()
        .add_rendered(tree)
        .add_rendered(rules)
        .gap(2)
        .render(40);

    vstack(&[list, blank(), tree_and_rules], 0)
}

/// One titled rule with a colored title and line.
fn rule(title: &str, border: BorderType, color: Color) -> Rendered {
    Rule::with_title(Text::styled(title, Style::new().fg(color).bold()))
        .border(border)
        .style(Style::new().fg(color))
        .width(20)
        .render(20)
}

/// Right column: key-value pairs, badges and a staggered columns demo.
fn right_column() -> Rendered {
    let kv = KeyValue::new()
        .key_style(Style::new().fg(Color::Cyan).bold())
        .value_style(Style::new())
        .add("host", "localhost")
        .add("port", "8080")
        .add("scheme", "https")
        .add("timeout", "30s")
        .render(24);

    let badge = |text: &str, color: Color| {
        Badge::new(text)
            .pad(1)
            .style(Style::new().fg(Color::Black).bg(color).bold())
            .span()
    };
    let badges = Rendered::new(vec![
        Line::new(vec![
            badge("DONE", Color::Green),
            Span::raw(" "),
            badge("INFO", Color::LightBlue),
        ]),
        Line::default(),
        Line::new(vec![
            badge("WARN", Color::Yellow),
            Span::raw(" "),
            badge("FAIL", Color::Red),
        ]),
    ]);

    let label = |text: &str| {
        Rendered::new(vec![Line::styled(
            text,
            Style::new().fg(Color::Cyan).bold(),
        )])
    };
    let staggered = Columns::new()
        .add_rendered(label("Col 1"))
        .add_rendered(vstack(&[blank(), label("Col 2")], 0))
        .add_rendered(vstack(&[blank(), blank(), label("Col 3")], 0))
        .gap(2)
        .separator(BorderType::Single)
        .render(40);

    vstack(&[kv, blank(), badges, blank(), staggered], 0)
}

/// The bottom progress bar.
fn progress() -> Result<()> {
    ProgressBar::new()
        .label("Building")
        .fill_color(Color::Green)
        .caps("[", "]")
        .width(29)
        .show_value(true)
        .bar(92.0, 100.0)
        .print()
}
