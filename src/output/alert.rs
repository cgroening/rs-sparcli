//! Pre-styled alert panels (info, debug, warning, error, success).

use crate::core::geometry::Edges;
use crate::core::render::{Renderable, Rendered};
use crate::core::style::Style;
use crate::core::text::{Line, Span, Text};
use crate::core::theme::{Theme, theme};
use crate::output::panel::{BoxOpts, draw_box};

/// The severity/kind of an [`Alert`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    /// Informational message.
    Info,
    /// Debug/diagnostic message.
    Debug,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
    /// Success message.
    Success,
}

impl AlertKind {
    /// Returns the leading icon glyph for the given glyph mode.
    fn icon(self, unicode: bool) -> &'static str {
        match (self, unicode) {
            (Self::Info, true) => "ℹ",
            (Self::Debug, true) => "⚙",
            (Self::Warning, true) => "⚠",
            (Self::Error, true) => "✖",
            (Self::Success, true) => "✔",
            (Self::Info, false) => "i",
            (Self::Debug, false) => "*",
            (Self::Warning, false) => "!",
            (Self::Error, false) => "x",
            (Self::Success, false) => "+",
        }
    }

    /// Returns the accent style for this kind from the theme.
    fn style(self, theme: &Theme) -> Style {
        match self {
            Self::Info => theme.info,
            Self::Debug => theme.debug,
            Self::Warning => theme.warning,
            Self::Error => theme.error,
            Self::Success => theme.success,
        }
    }
}

/// A bordered, pre-styled status message.
pub struct Alert {
    kind: AlertKind,
    content: Text,
}

impl Alert {
    /// Creates an alert of the given kind.
    pub fn new(kind: AlertKind, content: impl Into<Text>) -> Self {
        Self {
            kind,
            content: content.into(),
        }
    }

    /// Creates an info alert.
    pub fn info(content: impl Into<Text>) -> Self {
        Self::new(AlertKind::Info, content)
    }

    /// Creates a debug alert.
    pub fn debug(content: impl Into<Text>) -> Self {
        Self::new(AlertKind::Debug, content)
    }

    /// Creates a warning alert.
    pub fn warning(content: impl Into<Text>) -> Self {
        Self::new(AlertKind::Warning, content)
    }

    /// Creates an error alert.
    pub fn error(content: impl Into<Text>) -> Self {
        Self::new(AlertKind::Error, content)
    }

    /// Creates a success alert.
    pub fn success(content: impl Into<Text>) -> Self {
        Self::new(AlertKind::Success, content)
    }
}

impl Renderable for Alert {
    fn render(&self, _max_width: u16) -> Rendered {
        let theme = theme();
        let style = self.kind.style(&theme);
        let body =
            with_icon(&self.content, self.kind.icon(theme.unicode), style);
        let opts = BoxOpts {
            border: theme.border,
            border_style: style,
            padding: Edges::symmetric(0, 1),
            ..BoxOpts::default()
        };
        draw_box(&body, &opts)
    }
}

/// Prefixes the first content line with a styled icon.
fn with_icon(content: &Text, icon: &str, style: Style) -> Rendered {
    let mut lines = content.lines.clone();
    let icon_span = Span::styled(format!("{icon} "), style);
    match lines.first_mut() {
        Some(first) => first.spans.insert(0, icon_span),
        None => lines.push(Line::new(vec![icon_span])),
    }
    Rendered::new(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(rendered: &Rendered) -> Vec<String> {
        rendered.lines.iter().map(Line::plain).collect()
    }

    #[test]
    fn alert_includes_icon_and_message() {
        let lines = plain(&Alert::success("done").render(40));
        assert!(lines.iter().any(|l| l.contains("done")));
        assert!(lines.iter().any(|l| l.contains('✔')));
    }

    #[test]
    fn alert_is_bordered() {
        let lines = plain(&Alert::error("boom").render(40));
        assert_eq!(lines.len(), 3);
    }
}
