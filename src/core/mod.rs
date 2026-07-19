//! Foundational building blocks shared by the output and input layers.
//!
//! Nothing here knows about concrete widgets; the modules provide colors and
//! styles, rich text, geometry, borders, the unified theme, width math,
//! terminal capabilities, command-line splitting, in-place redrawing and the
//! render model.

pub mod border;
pub mod command;
pub(crate) mod cursor;
pub mod geometry;
pub(crate) mod inplace;
pub(crate) mod private_file;
pub mod render;
pub mod style;
pub mod terminal;
pub mod text;
pub mod theme;
pub mod width;

#[cfg(feature = "markup")]
pub mod markup;
