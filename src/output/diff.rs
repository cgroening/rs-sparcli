//! Colored unified diff rendering (line-based LCS).

use crate::core::render::{Renderable, Rendered};
use crate::core::style::{Color, Style};
use crate::core::text::Line;
use crate::core::theme::theme;

/// Default number of context lines around each change.
const DEFAULT_CONTEXT: usize = 3;
/// Maximum number of lines per side before falling back to a full listing.
const MAX_DIFF_LINES: usize = 5_000;

/// A single line operation in the edit script.
enum Op {
    Equal(String),
    Delete(String),
    Insert(String),
}

/// A unified diff between two texts.
///
/// # Examples
///
/// ```
/// use sparcli::{Diff, Renderable};
///
/// let out = Diff::new("alpha\nbeta\n", "alpha\ngamma\n").render(40);
/// assert!(out.plain().contains("gamma"));
/// ```
pub struct Diff {
    old: String,
    new: String,
    context: usize,
    no_header: bool,
    old_label: String,
    new_label: String,
    add_style: Style,
    del_style: Style,
    hunk_style: Style,
}

impl Diff {
    /// Creates a diff between `old` and `new`.
    pub fn new(old: impl Into<String>, new: impl Into<String>) -> Self {
        let theme = theme();
        Self {
            old: old.into(),
            new: new.into(),
            context: DEFAULT_CONTEXT,
            no_header: false,
            old_label: "old".to_string(),
            new_label: "new".to_string(),
            add_style: Style::new().fg(Color::Green),
            del_style: Style::new().fg(Color::Red),
            hunk_style: theme.secondary,
        }
    }

    /// Sets the number of context lines around changes.
    #[must_use]
    pub fn context(mut self, lines: usize) -> Self {
        self.context = lines;
        self
    }

    /// Hides the `---`/`+++` header.
    #[must_use]
    pub fn no_header(mut self) -> Self {
        self.no_header = true;
        self
    }

    /// Sets the old and new side labels.
    #[must_use]
    pub fn labels(
        mut self,
        old: impl Into<String>,
        new: impl Into<String>,
    ) -> Self {
        self.old_label = old.into();
        self.new_label = new.into();
        self
    }
}

impl Renderable for Diff {
    fn render(&self, _max_width: u16) -> Rendered {
        let old: Vec<&str> = self.old.lines().collect();
        let new: Vec<&str> = self.new.lines().collect();
        let ops = diff_ops(&old, &new);
        let mut lines = Vec::new();
        if !self.no_header {
            self.push_header(&mut lines);
        }
        self.push_ops(&mut lines, &ops);
        Rendered::new(lines)
    }
}

impl Diff {
    /// Pushes the `---`/`+++` header lines.
    fn push_header(&self, lines: &mut Vec<Line>) {
        lines.push(Line::styled(
            format!("--- {}", self.old_label),
            self.del_style,
        ));
        lines.push(Line::styled(
            format!("+++ {}", self.new_label),
            self.add_style,
        ));
    }

    /// Pushes the diff body, collapsing unchanged regions to context.
    fn push_ops(&self, lines: &mut Vec<Line>, ops: &[Op]) {
        let visible = mark_visible(ops, self.context);
        let mut last_visible = false;
        for (index, op) in ops.iter().enumerate() {
            if !visible[index] {
                last_visible = false;
                continue;
            }
            if !last_visible && index > 0 {
                lines.push(Line::styled("…".to_string(), self.hunk_style));
            }
            lines.push(self.format_op(op));
            last_visible = true;
        }
    }

    /// Formats a single operation as a styled line.
    fn format_op(&self, op: &Op) -> Line {
        match op {
            Op::Equal(text) => Line::raw(format!("  {text}")),
            Op::Delete(text) => {
                Line::styled(format!("- {text}"), self.del_style)
            }
            Op::Insert(text) => {
                Line::styled(format!("+ {text}"), self.add_style)
            }
        }
    }
}

/// Marks which ops are visible given the surrounding context window.
fn mark_visible(ops: &[Op], context: usize) -> Vec<bool> {
    let mut visible = vec![false; ops.len()];
    for (index, op) in ops.iter().enumerate() {
        if matches!(op, Op::Equal(_)) {
            continue;
        }
        let start = index.saturating_sub(context);
        let end = (index + context + 1).min(ops.len());
        for flag in visible.iter_mut().take(end).skip(start) {
            *flag = true;
        }
    }
    visible
}

/// Computes a line-based edit script using an LCS table.
fn diff_ops(old: &[&str], new: &[&str]) -> Vec<Op> {
    if old.len() > MAX_DIFF_LINES || new.len() > MAX_DIFF_LINES {
        return fallback_ops(old, new);
    }
    let table = lcs_table(old, new);
    backtrack(&table, old, new)
}

/// Builds the LCS length table.
fn lcs_table(old: &[&str], new: &[&str]) -> Vec<Vec<usize>> {
    let mut table = vec![vec![0usize; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            table[i][j] = if old[i] == new[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    table
}

/// Backtracks the LCS table into an ordered edit script.
fn backtrack(table: &[Vec<usize>], old: &[&str], new: &[&str]) -> Vec<Op> {
    let mut ops = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < old.len() && j < new.len() {
        if old[i] == new[j] {
            ops.push(Op::Equal(old[i].to_string()));
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            ops.push(Op::Delete(old[i].to_string()));
            i += 1;
        } else {
            ops.push(Op::Insert(new[j].to_string()));
            j += 1;
        }
    }
    ops.extend(old[i..].iter().map(|l| Op::Delete(l.to_string())));
    ops.extend(new[j..].iter().map(|l| Op::Insert(l.to_string())));
    ops
}

/// A naive diff for very large inputs: delete all old, insert all new.
fn fallback_ops(old: &[&str], new: &[&str]) -> Vec<Op> {
    let mut ops = Vec::new();
    ops.extend(old.iter().map(|l| Op::Delete(l.to_string())));
    ops.extend(new.iter().map(|l| Op::Insert(l.to_string())));
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_added_and_removed_lines() {
        let diff = Diff::new("a\nb\nc", "a\nB\nc").no_header().context(1);
        let lines = diff.render(80).plain_lines();
        assert!(lines.iter().any(|l| l == "- b"));
        assert!(lines.iter().any(|l| l == "+ B"));
        assert!(lines.iter().any(|l| l == "  a"));
    }

    #[test]
    fn identical_input_has_no_changes() {
        let diff = Diff::new("x\ny", "x\ny").no_header();
        let lines = diff.render(80).plain_lines();
        assert!(lines.iter().all(|l| !l.starts_with('-')));
        assert!(lines.iter().all(|l| !l.starts_with('+')));
    }

    #[test]
    fn header_shows_labels() {
        let diff = Diff::new("a", "b").labels("before", "after");
        let lines = diff.render(80).plain_lines();
        assert_eq!(lines[0], "--- before");
        assert_eq!(lines[1], "+++ after");
    }
}
