//! Hierarchical tree views with box-drawing connectors.

use crate::core::border::BorderType;
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::{Line, Span, Text};
use crate::core::theme::theme;

/// A node in a [`Tree`].
pub struct TreeNode {
    content: Text,
    children: Vec<TreeNode>,
}

impl TreeNode {
    /// Creates a leaf node.
    pub fn new(content: impl Into<Text>) -> Self {
        Self {
            content: content.into(),
            children: Vec::new(),
        }
    }

    /// Adds a child node.
    #[must_use]
    pub fn child(mut self, child: TreeNode) -> Self {
        self.children.push(child);
        self
    }
}

/// A tree of [`TreeNode`]s rendered with connector glyphs.
pub struct Tree {
    roots: Vec<TreeNode>,
    border: BorderType,
    connector_style: Style,
    dashes: u16,
    guides: bool,
}

impl Default for Tree {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            border: BorderType::Single,
            connector_style: theme().secondary,
            dashes: 1,
            guides: true,
        }
    }
}

impl Tree {
    /// Creates an empty tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a top-level (root) node.
    #[must_use]
    pub fn node(mut self, node: TreeNode) -> Self {
        self.roots.push(node);
        self
    }

    /// Sets the connector glyph set.
    #[must_use]
    pub fn border(mut self, border: BorderType) -> Self {
        self.border = border;
        self
    }

    /// Sets the connector style.
    #[must_use]
    pub fn connector_style(mut self, style: Style) -> Self {
        self.connector_style = style;
        self
    }

    /// Disables the vertical continuation guides.
    #[must_use]
    pub fn no_guides(mut self) -> Self {
        self.guides = false;
        self
    }

    /// Returns the column width of a connector (`branch + dashes + space`).
    fn connector_width(&self) -> usize {
        2 + self.dashes as usize
    }

    /// Renders the children of a node beneath the given prefix.
    fn render_children(
        &self,
        children: &[TreeNode],
        prefix: &str,
        lines: &mut Vec<Line>,
    ) {
        let chars = self.border.chars();
        let dash = chars.horizontal.to_string().repeat(self.dashes as usize);
        for (index, child) in children.iter().enumerate() {
            let last = index + 1 == children.len();
            let branch = if last {
                chars.bottom_left
            } else {
                chars.tee_right
            };
            let connector = format!("{branch}{dash} ");
            let continuation = self.continuation(last);
            self.push_node_lines(
                child,
                prefix,
                &connector,
                &continuation,
                lines,
            );
            let child_prefix = format!("{prefix}{}", self.child_prefix(last));
            self.render_children(&child.children, &child_prefix, lines);
        }
    }

    /// Emits the content lines of a single node.
    fn push_node_lines(
        &self,
        node: &TreeNode,
        prefix: &str,
        connector: &str,
        continuation: &str,
        lines: &mut Vec<Line>,
    ) {
        for (row, content_line) in node.content.lines.iter().enumerate() {
            let mut spans = vec![Span::raw(prefix.to_string())];
            if row == 0 {
                spans.push(Span::styled(
                    connector.to_string(),
                    self.connector_style,
                ));
            } else {
                spans.push(Span::raw(continuation.to_string()));
            }
            spans.extend(content_line.spans.iter().cloned());
            lines.push(Line::new(spans));
        }
    }

    /// The continuation cell shown under a node's connector.
    fn continuation(&self, last: bool) -> String {
        let width = self.connector_width();
        if last || !self.guides {
            " ".repeat(width)
        } else {
            let guide = self.border.chars().vertical;
            format!("{guide}{}", " ".repeat(width - 1))
        }
    }

    /// The prefix added for a child's own descendants.
    fn child_prefix(&self, last: bool) -> String {
        self.continuation(last)
    }
}

impl Renderable for Tree {
    fn render(&self, _max_width: u16) -> Rendered {
        let mut lines = Vec::new();
        for root in &self.roots {
            for content_line in &root.content.lines {
                lines.push(Line::new(content_line.spans.clone()));
            }
            self.render_children(&root.children, "", &mut lines);
        }
        Rendered::new(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(rendered: &Rendered) -> Vec<String> {
        rendered.lines.iter().map(Line::plain).collect()
    }

    #[test]
    fn renders_root_and_branches() {
        let tree = Tree::new().node(
            TreeNode::new("root")
                .child(TreeNode::new("a"))
                .child(TreeNode::new("b")),
        );
        let lines = plain(&tree.render(40));
        assert_eq!(lines[0], "root");
        assert_eq!(lines[1], "├─ a");
        assert_eq!(lines[2], "└─ b");
    }

    #[test]
    fn nested_children_show_guides() {
        let tree = Tree::new().node(
            TreeNode::new("root")
                .child(TreeNode::new("a").child(TreeNode::new("a1")))
                .child(TreeNode::new("b")),
        );
        let lines = plain(&tree.render(40));
        assert_eq!(lines[1], "├─ a");
        assert_eq!(lines[2], "│  └─ a1");
        assert_eq!(lines[3], "└─ b");
    }
}
