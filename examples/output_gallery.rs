//! A gallery of the output widgets. Run with `cargo run --example
//! output_gallery` (add `--features markup,fuzzy,pager` to see those too).

use sparcli::prelude::*;
use sparcli::{
    Alert, Badge, Cell, Columns, Diff, KeyValue, List, Marker, Panel,
    ProgressBar, Rule, Table, Tree, TreeNode,
};

fn main() -> Result<()> {
    Rule::with_title("sparcli output gallery").print()?;
    println!();

    Panel::new("A bordered panel with a title.")
        .title(Title::new("Panel"))
        .print()?;
    println!();

    Alert::success("Everything compiled.").print()?;
    Alert::warning("Low on disk space.").print()?;
    Alert::error("Connection refused.").print()?;
    println!();

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

    List::ordered(Marker::Number)
        .item("First step")
        .item_with("Second step", List::new().item("detail a").item("detail b"))
        .item("Third step")
        .print()?;
    println!();

    Tree::new()
        .node(
            TreeNode::new("project")
                .child(TreeNode::new("src").child(TreeNode::new("main.rs")))
                .child(TreeNode::new("Cargo.toml")),
        )
        .print()?;
    println!();

    KeyValue::new()
        .add("Version", "0.1.0")
        .add("License", "MIT OR Apache-2.0")
        .print()?;
    println!();

    let mut badges = Rendered::empty();
    badges.push(Line::new(vec![
        Badge::new("PASS")
            .style(Style::new().fg(Color::Green).bold())
            .span(),
        Span::raw(" "),
        Badge::new("v0.1").caps("(", ")").span(),
    ]));
    badges.print()?;
    println!();

    ProgressBar::new()
        .label("download")
        .width(24)
        .bar(7.0, 10.0)
        .print()?;
    println!();

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
        .gap(2)
        .print()?;

    Ok(())
}
