//! End-to-end tests over the public API.
//!
//! Output widgets are rendered to an in-memory buffer and compared on their
//! visible text (ANSI stripped), so the assertions hold whether or not the
//! test process has a color-capable terminal.

use sparcli::core::width::strip_ansi;
use sparcli::prelude::*;
use sparcli::{Alert, List, Marker, Panel, Table, Tree, TreeNode};

/// Renders a widget and returns its visible text (no ANSI escapes).
fn visible(widget: &impl Renderable) -> String {
    let mut buffer = Vec::new();
    widget.print_to(&mut buffer).expect("render to buffer");
    let text = String::from_utf8(buffer).expect("utf8 output");
    strip_ansi(&text)
}

#[test]
fn table_renders_headers_and_rows() {
    let table = Table::new()
        .columns(["City", "Pop"])
        .row(["Berlin", "3.7M"])
        .row(["Kyoto", "1.5M"]);
    let output = visible(&table);
    assert!(output.contains("City"));
    assert!(output.contains("Berlin"));
    assert!(output.contains("Kyoto"));
}

#[test]
fn panel_frames_titled_content() {
    let panel = Panel::new("hello").title(Title::new("Greeting"));
    let output = visible(&panel);
    assert!(output.contains("Greeting"));
    assert!(output.contains("hello"));
}

#[test]
fn alert_renders_message() {
    assert!(visible(&Alert::success("done")).contains("done"));
}

#[test]
fn nested_list_indents_children() {
    let list = List::ordered(Marker::Number)
        .item("a")
        .item_with("b", List::new().item("child"));
    let output = visible(&list);
    assert!(output.contains("1. a"));
    assert!(output.contains("child"));
}

#[test]
fn tree_renders_branches() {
    let tree = Tree::new().node(
        TreeNode::new("root")
            .child(TreeNode::new("x"))
            .child(TreeNode::new("y")),
    );
    let output = visible(&tree);
    assert!(output.contains("root"));
    assert!(output.contains("x"));
    assert!(output.contains("y"));
}
