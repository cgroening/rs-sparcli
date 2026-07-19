//! `sparcli` is a lightweight, cross-platform toolkit for styled command-line
//! output and interactive input widgets.
//!
//! It renders directly to the terminal via [`crossterm`] (no `ratatui`
//! dependency) but mirrors ratatui's familiar vocabulary ([`Style`],
//! [`Color`], [`Span`], [`Line`], [`Text`]). Output widgets implement
//! [`Renderable`] and can be printed inline; input widgets run small,
//! self-contained prompt loops.
//!
//! The base crate stays small; heavier features live behind cargo features
//! (`markup`, `fuzzy`, `pager`).
//!
//! # Examples
//!
//! Build a styled panel and render it to a string (no terminal required),
//! which is exactly how the output widgets are tested:
//!
//! ```
//! use sparcli::{Color, Panel, Renderable, Style, Title};
//!
//! let panel = Panel::new("Build succeeded.")
//!     .title(Title::new("Status"))
//!     .border_style(Style::new().fg(Color::Green));
//!
//! let out = panel.render(40);
//! assert!(out.plain().contains("Build succeeded."));
//! ```
//!
//! In a real program you would print it straight to the terminal with
//! `panel.print()?` instead of rendering to a string.
//!
//! [`Style`]: crate::Style
//! [`Color`]: crate::Color
//! [`Span`]: crate::Span
//! [`Line`]: crate::Line
//! [`Text`]: crate::Text
//! [`Renderable`]: crate::Renderable

#![deny(missing_docs)]

pub(crate) mod core;
pub(crate) mod error;
pub(crate) mod input;
pub(crate) mod output;

pub use core::border::BorderType;
pub use core::geometry::{Align, Edges, Position, Title, VAlign};
pub use core::render::{Renderable, Rendered};
pub use core::style::{Attribute, Color, Modifier, Style};
pub use core::text::{Line, Span, Text};
pub use core::theme::{Theme, set_theme, theme};
pub use error::{Result, SparcliError};
pub use output::alert::{Alert, AlertKind};
pub use output::badge::Badge;
pub use output::card::Card;
pub use output::columns::Columns;
pub use output::diff::Diff;
pub use output::kv::KeyValue;
pub use output::list::{List, Marker};
pub use output::live::Live;
pub use output::multiprogress::MultiProgress;
pub use output::panel::Panel;
pub use output::progress::{ProgressBar, ProgressStyle, Thresholds};
pub use output::rule::Rule;
pub use output::spinner::{Spinner, SpinnerStyle};
pub use output::table::{Cell, Column, Table};
pub use output::tree::{Tree, TreeNode};

#[cfg(feature = "pager")]
pub use output::pager::Pager;

pub use input::Outcome;
pub use input::confirm::Confirm;
pub use input::datepicker::{Date, DatePicker};
pub use input::editor::edit_file;
pub use input::history::History;
pub use input::number::NumberInput;
pub use input::password::PasswordInput;
pub use input::select::Select;
pub use input::shortcut::Shortcut;
pub use input::text::TextInput;
pub use input::textarea::Textarea;

#[cfg(feature = "fuzzy")]
pub use input::fuzzy::FuzzySelect;

/// Rich-style inline markup parsing (`[bold red]text[/]`).
#[cfg(feature = "markup")]
pub mod markup {
    pub use crate::core::markup::{markup_print, markup_println, parse};
}

/// Value validators and character filters for text prompts.
pub mod validate {
    pub use crate::input::validate::{
        CharFilter, Validator, alnum, alpha, decimal, digits, min_len,
        no_space, non_empty,
    };
}

/// Keyboard events and the dependency-injected event source (for headless
/// testing and custom input backends).
pub mod event {
    pub use crate::input::event::{
        CrosstermSource, EventSource, InputEvent, KeyCode, KeyPress,
    };
}

/// Keyboard shortcuts and their footer-hint / help-overlay rendering.
pub mod shortcut {
    pub use crate::input::shortcut::{
        Shortcut, find, help_overlay, hint_line, key_name,
    };
}

/// Unicode-aware display-width helpers (width, ANSI stripping, wrap, truncate).
pub mod width {
    pub use crate::core::width::{
        strip_ansi, truncate, truncate_line, visible_width, wrap, wrap_line,
    };
}

/// Terminal capability and size detection.
pub mod terminal {
    pub use crate::core::terminal::{
        ColorSupport, color_support, is_input_tty, is_output_tty, term_height,
        term_width, terminal_size,
    };
}

/// Commonly used types, re-exported for `use sparcli::prelude::*;`.
pub mod prelude {
    pub use crate::core::border::BorderType;
    pub use crate::core::geometry::{Align, Edges, Position, Title, VAlign};
    pub use crate::core::render::{Renderable, Rendered};
    pub use crate::core::style::{Attribute, Color, Modifier, Style};
    pub use crate::core::text::{Line, Span, Text};
    pub use crate::core::theme::{Theme, set_theme, theme};
    pub use crate::error::{Result, SparcliError};
    pub use crate::output::alert::{Alert, AlertKind};
    pub use crate::output::badge::Badge;
    pub use crate::output::card::Card;
    pub use crate::output::compose::{align, pad, vstack};
    pub use crate::output::kv::KeyValue;
    pub use crate::output::list::{List, Marker};
    pub use crate::output::panel::Panel;
    pub use crate::output::rule::Rule;
    pub use crate::output::tree::{Tree, TreeNode};
}
