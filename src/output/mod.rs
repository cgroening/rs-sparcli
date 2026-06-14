//! Printable output widgets.
//!
//! Each widget builds a [`Rendered`](crate::core::render::Rendered) block and
//! implements [`Renderable`](crate::core::render::Renderable), so it can be
//! printed inline or composed with [`compose`].

pub mod alert;
pub mod badge;
pub mod columns;
pub mod compose;
pub mod diff;
pub mod kv;
pub mod list;
pub mod live;
pub mod multiprogress;
pub mod panel;
pub mod progress;
pub mod rule;
pub mod spinner;
pub mod table;
pub mod tree;

#[cfg(feature = "pager")]
pub mod pager;

pub(crate) mod layout;
