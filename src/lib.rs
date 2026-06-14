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
//! [`Style`]: core::style::Style
//! [`Color`]: core::style::Color
//! [`Span`]: core::text::Span
//! [`Line`]: core::text::Line
//! [`Text`]: core::text::Text
//! [`Renderable`]: core::render::Renderable

#![warn(missing_docs)]

pub mod core;
pub mod error;
pub mod input;
pub mod output;

pub use core::border::BorderType;
pub use core::geometry::{Align, Edges, Position, Title, VAlign};
pub use core::render::{Renderable, Rendered};
pub use core::style::{Attribute, Color, Modifier, Style};
pub use core::text::{Line, Span, Text};
pub use core::theme::{Theme, set_theme, theme};
pub use error::{Result, SparcliError};
pub use output::alert::{Alert, AlertKind};
pub use output::badge::Badge;
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
pub use input::history::History;
pub use input::number::NumberInput;
pub use input::password::PasswordInput;
pub use input::select::Select;
pub use input::shortcut::Shortcut;
pub use input::text::TextInput;
pub use input::textarea::Textarea;

#[cfg(feature = "fuzzy")]
pub use input::fuzzy::FuzzySelect;

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
    pub use crate::output::compose::{align, pad, vstack};
    pub use crate::output::kv::KeyValue;
    pub use crate::output::list::{List, Marker};
    pub use crate::output::panel::Panel;
    pub use crate::output::rule::Rule;
    pub use crate::output::tree::{Tree, TreeNode};
}
