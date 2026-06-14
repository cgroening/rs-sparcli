//! A curated, non-interactive output collage for a README screenshot.
//!
//! `cargo run --example output-readme --features markup`
//!
//! Mirrors the C sparcli `readme_screenshots_output` demo: a hero panel, a
//! side-by-side row (table | list+tree | key-value+badges), then alerts.

use sparcli::prelude::*;
use sparcli::{
    Alert, Badge, Column, Columns, KeyValue, List, Marker, Panel, Table, Tree,
    TreeNode,
};

#[cfg(feature = "markup")]
use sparcli::core::markup::parse;

fn main() -> Result<()> {
    hero()?;
    println!();
    dashboard()?;
    println!();
    footer()?;
    Ok(())
}

/// The top hero panel with styled text.
fn hero() -> Result<()> {
    let accent = theme().accent;
    let body = Text::new(vec![Line::new(vec![
        Span::raw("A native Rust toolkit for "),
        Span::styled("styled output", Style::new().fg(accent).bold()),
        Span::raw(" and "),
        Span::styled("input", Style::new().fg(Color::Magenta).bold()),
        Span::raw("."),
    ])]);
    Panel::new(body)
        .border(BorderType::Rounded)
        .title(Title::new(" sparcli ").align(Align::Center))
        .content_align(Align::Center)
        .width(64)
        .print()
}

/// A three-column dashboard row.
fn dashboard() -> Result<()> {
    let table = Table::new()
        .title("Overview")
        .columns([
            Column::new("Service"),
            Column::new("Status").align(Align::Center),
            Column::new("Uptime").align(Align::Right),
        ])
        .row(["api-gateway", "● OK", "99.98%"])
        .row(["worker", "● OK", "99.95%"])
        .row(["database", "◐ WARN", "99.40%"])
        .striped(true)
        .render(36);

    let list = List::ordered(Marker::Number)
        .item("Build")
        .item("Test")
        .item("Ship")
        .render(20);
    let tree = Tree::new()
        .node(
            TreeNode::new("project")
                .child(TreeNode::new("src").child(TreeNode::new("main.rs")))
                .child(TreeNode::new("Cargo.toml")),
        )
        .render(20);
    let middle = vstack(&[list, Rendered::new(vec![Line::default()]), tree], 0);

    let kv = KeyValue::new()
        .add("version", "0.1.0")
        .add("license", "MIT/Apache")
        .render(22);
    let badges = Rendered::new(vec![Line::new(vec![
        Badge::new("PASS")
            .style(Style::new().fg(Color::Green).bold())
            .span(),
        Span::raw(" "),
        Badge::new("v0.1").caps("(", ")").span(),
    ])]);
    let right = vstack(&[kv, badges], 1);

    Columns::new()
        .add_rendered(table)
        .add_rendered(middle)
        .add_rendered(right)
        .gap(3)
        .separator(BorderType::Single)
        .print()
}

/// Closing alerts and a markup line.
fn footer() -> Result<()> {
    Alert::success("All systems operational.").print()?;
    #[cfg(feature = "markup")]
    parse("[bold green]Hello[/], [italic]sparcli[/]!").print()?;
    Ok(())
}
