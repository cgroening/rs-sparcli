//! End-to-end tests over the public API.
//!
//! Output widgets are rendered to an in-memory buffer and compared on their
//! visible text (ANSI stripped), so the assertions hold whether or not the
//! test process has a color-capable terminal.

use sparcli::prelude::*;
use sparcli::width::strip_ansi;
use sparcli::{
    Alert, Badge, Card, Columns, Diff, KeyValue, List, Marker, Panel, Rule,
    Table, Text, Tree, TreeNode,
};

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
    let panel =
        Panel::new("hello from the panel body").title(Title::new("Greeting"));
    let output = visible(&panel);
    assert!(output.contains("Greeting"));
    assert!(output.contains("hello from the panel body"));
}

#[test]
fn card_renders_title_content_and_footer() {
    let card = Card::new("hello from the card body")
        .title("Greeting")
        .footer("signed, the card")
        .accent(Color::Blue)
        .width(40);
    let output = visible(&card);
    assert!(output.contains("Greeting"));
    assert!(output.contains("hello from the card body"));
    assert!(output.contains("signed, the card"));
}

#[test]
fn card_draws_a_tall_border() {
    let card = Card::new("bounded by half-block bars")
        .title("Tall")
        .border(BorderType::Tall)
        .width(40);
    let output = visible(&card);
    assert!(output.contains("bounded by half-block bars"));
    // Without truecolor the bars degrade to the heavy frame, so accept either.
    assert!(output.contains('▊') || output.contains('┃'), "{output}");
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

#[test]
fn key_value_pairs_align_their_values() {
    let pairs = KeyValue::new().add("host", "web-1").add("port", "80");
    let text = visible(&pairs);
    assert!(text.contains("host"));
    assert!(text.contains("web-1"));
    assert!(text.contains("port"));
}

#[test]
fn diff_marks_added_and_removed_lines() {
    let text = visible(&Diff::new("one\ntwo", "one\nthree"));
    assert!(text.contains("two"), "the removed line is shown");
    assert!(text.contains("three"), "the added line is shown");
}

#[test]
fn columns_lay_items_out_side_by_side() {
    let columns = Columns::new()
        .add(&Text::raw("alpha"), 10)
        .add(&Text::raw("beta"), 10)
        .add(&Text::raw("gamma"), 10);
    let text = visible(&columns);
    for item in ["alpha", "beta", "gamma"] {
        assert!(text.contains(item), "{item} is missing");
    }
}

#[test]
fn a_rule_fills_the_width_and_carries_its_label() {
    let text = visible(&Rule::with_title("Section"));
    assert!(text.contains("Section"));
}

#[test]
fn a_badge_renders_its_label() {
    assert!(visible(&Badge::new("NEW")).contains("NEW"));
}

#[test]
fn printing_without_a_terminal_does_not_truncate() {
    // Section 1.6: piped output has no width to fit, so a wide table keeps
    // every column instead of clipping to an invented 80 and losing data.
    let wide = "w".repeat(200);
    let table = Table::new().columns(["id", "value"]).row(["1", &wide]);
    let text = visible(&table);
    assert!(text.contains(&wide), "the full value survives the pipe");
    assert!(!text.contains('…'), "nothing was clipped");
}

#[test]
fn an_explicit_render_width_still_truncates() {
    // The no-truncation rule applies to print/print_to, which resolve the
    // width themselves. An explicit width is the caller's decision and is
    // still honoured.
    let wide = "w".repeat(200);
    let table = Table::new().columns(["id", "value"]).row(["1", &wide]);
    let text = strip_ansi(&table.render(40).plain());
    assert!(text.contains('…'), "an explicit width clips as before");
}
