//! Filled cards: a colored surface with its own title and footer rows.
//!
//! Where [`Panel`](crate::output::panel::Panel) draws a frame and embeds the
//! title in the top border, a card is a surface: the background carries the
//! shape, an outer border is optional, and the title sits on its own row. All
//! of its colors are derived from a single accent, so one call is enough.

mod palette;
mod render;

use crate::core::border::BorderType;
use crate::core::geometry::{Align, Edges};
use crate::core::render::Rendered;
use crate::core::style::{Color, Style};
use crate::core::text::Text;
use crate::core::theme::theme;

/// Layout and styling options of a [`Card`].
pub(crate) struct CardOpts {
    /// Accent color every derived tone is built from.
    pub accent: Color,
    /// Border type; [`BorderType::None`] leaves the surface unframed.
    pub border: BorderType,
    /// Optional fixed outer width in columns.
    pub width: Option<u16>,
    /// Padding around the body content.
    pub padding: Edges,
    /// Padding around the title row.
    pub title_padding: Edges,
    /// Padding around the footer row.
    pub footer_padding: Edges,
    /// Horizontal alignment of the title.
    pub title_align: Align,
    /// Horizontal alignment of the body content.
    pub content_align: Align,
    /// Horizontal alignment of the footer.
    pub footer_align: Align,
    /// Whether overlong lines wrap instead of being truncated.
    pub wrap: bool,
    /// Whether the title row shares the content background.
    pub flat_title: bool,
    /// Whether the footer row shares the content background.
    pub flat_footer: bool,
    /// Override patched onto the derived title style.
    pub title_style: Style,
    /// Override patched onto the derived body text style.
    pub content_style: Style,
    /// Override patched onto the derived surface background.
    pub fill: Style,
    /// Override patched onto the derived border style.
    pub border_style: Style,
    /// Override patched onto the derived footer style.
    pub footer_style: Style,
}

impl Default for CardOpts {
    fn default() -> Self {
        Self {
            accent: theme().accent,
            border: BorderType::None,
            width: None,
            padding: Edges::symmetric(1, 1),
            title_padding: Edges::symmetric(0, 1),
            footer_padding: Edges::symmetric(0, 1),
            title_align: Align::Left,
            content_align: Align::Left,
            footer_align: Align::Left,
            wrap: true,
            flat_title: false,
            flat_footer: false,
            title_style: Style::new(),
            content_style: Style::new(),
            fill: Style::new(),
            border_style: Style::new(),
            footer_style: Style::new(),
        }
    }
}

/// A filled card: a colored surface with an optional title and footer row.
///
/// Every color comes from one accent (by default [`Theme::accent`]): the title
/// keeps it saturated, the body text and both surfaces are desaturated, darker
/// shades of the same hue. The individual style setters patch the derived
/// values rather than replacing them, so `.title_style(Style::new().bold())`
/// keeps the derived colors and only adds the attribute.
///
/// A card fills the whole width it is rendered into unless [`Card::width`]
/// narrows it, and it carries no border unless [`Card::border`] adds one - note
/// that this ignores [`Theme::border`], unlike every framed widget. Below
/// truecolor support the surfaces are dropped and the card renders as accented
/// text, because the derived shades would collapse onto one ANSI-16 color.
///
/// [`Theme::accent`]: crate::core::theme::Theme::accent
/// [`Theme::border`]: crate::core::theme::Theme::border
///
/// # Examples
///
/// ```
/// use sparcli::{Card, Color, Renderable};
///
/// let out = Card::new("All systems nominal.")
///     .title("Status")
///     .accent(Color::from_hex("#89b4fa").unwrap_or(Color::Blue))
///     .render(40);
/// assert!(out.plain().contains("Status"));
/// assert!(out.plain().contains("All systems nominal."));
/// ```
pub struct Card {
    /// The body content.
    content: Rendered,
    /// The optional title row content.
    title: Option<Text>,
    /// The optional footer row content.
    footer: Option<Text>,
    /// Layout and styling options.
    opts: CardOpts,
}

impl Card {
    /// Creates a card around text content.
    pub fn new(content: impl Into<Text>) -> Self {
        let text = content.into();
        Self::from_rendered(Rendered::new(text.lines))
    }

    /// Creates a card around an already rendered block.
    pub fn from_rendered(content: Rendered) -> Self {
        Self {
            content,
            title: None,
            footer: None,
            opts: CardOpts::default(),
        }
    }

    /// Sets the title row content.
    ///
    /// Unlike [`Panel::title`](crate::output::panel::Panel::title) this takes
    /// plain text: a card's title is a row of its own, so the position and pad
    /// of a [`Title`](crate::core::geometry::Title) would carry no meaning and
    /// its alignment would duplicate [`Card::title_align`].
    #[must_use]
    pub fn title(mut self, title: impl Into<Text>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the footer row content.
    #[must_use]
    pub fn footer(mut self, footer: impl Into<Text>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Sets the accent color all other tones are derived from.
    ///
    /// Works with any [`Color`]: named colors and palette indices resolve
    /// through [`Color::to_rgb`]. An achromatic accent yields a neutral gray
    /// card rather than picking up an arbitrary hue.
    ///
    /// # Examples
    ///
    /// ```
    /// use sparcli::{Card, Color, Renderable};
    ///
    /// let out = Card::new("Done.").accent(Color::Green).render(20);
    /// assert_eq!(out.height(), 3);
    /// ```
    #[must_use]
    pub fn accent(mut self, accent: Color) -> Self {
        self.opts.accent = accent;
        self
    }

    /// Sets a fixed outer width in columns.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.opts.width = Some(width);
        self
    }

    /// Adds an outer border around the surface.
    ///
    /// [`BorderType::Tall`] is the one border a card draws natively: a thin
    /// block frame whose strokes come out equally thick on both axes and whose
    /// corners close. It needs both truecolor and Unicode glyphs to read, and
    /// degrades to [`BorderType::Thick`] (or to [`BorderType::Ascii`] when the
    /// theme disables Unicode) otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use sparcli::{BorderType, Card, Renderable};
    ///
    /// let out = Card::new("Deployed.").border(BorderType::Tall).render(30);
    /// assert_eq!(out.height(), 5);
    /// ```
    #[must_use]
    pub fn border(mut self, border: BorderType) -> Self {
        self.opts.border = border;
        self
    }

    /// Sets the padding around the body content.
    ///
    /// The vertical padding is what separates the body from the title row; with
    /// `Edges::symmetric(0, _)` the two sit on adjacent rows, separated only by
    /// the background step.
    #[must_use]
    pub fn padding(mut self, padding: Edges) -> Self {
        self.opts.padding = padding;
        self
    }

    /// Sets the padding around the title row.
    #[must_use]
    pub fn title_padding(mut self, padding: Edges) -> Self {
        self.opts.title_padding = padding;
        self
    }

    /// Sets the padding around the footer row.
    #[must_use]
    pub fn footer_padding(mut self, padding: Edges) -> Self {
        self.opts.footer_padding = padding;
        self
    }

    /// Sets the horizontal alignment of the title.
    #[must_use]
    pub fn title_align(mut self, align: Align) -> Self {
        self.opts.title_align = align;
        self
    }

    /// Sets the horizontal alignment of the body content.
    #[must_use]
    pub fn content_align(mut self, align: Align) -> Self {
        self.opts.content_align = align;
        self
    }

    /// Sets the horizontal alignment of the footer.
    #[must_use]
    pub fn footer_align(mut self, align: Align) -> Self {
        self.opts.footer_align = align;
        self
    }

    /// Lets the title row share the content background.
    ///
    /// The title then reads only through its saturated text color, which suits
    /// cards whose surface should stay one uninterrupted block.
    ///
    /// # Examples
    ///
    /// ```
    /// use sparcli::{Card, Renderable};
    ///
    /// let out = Card::new("body").title("Heading").flat_title().render(30);
    /// assert!(out.plain().contains("Heading"));
    /// ```
    #[must_use]
    pub fn flat_title(mut self) -> Self {
        self.opts.flat_title = true;
        self
    }

    /// Lets the footer row share the content background.
    #[must_use]
    pub fn flat_footer(mut self) -> Self {
        self.opts.flat_footer = true;
        self
    }

    /// Enables or disables automatic wrapping of overlong lines.
    ///
    /// Wrapping is on by default; disabling it truncates with `…` instead.
    #[must_use]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.opts.wrap = wrap;
        self
    }

    /// Patches the derived title style.
    #[must_use]
    pub fn title_style(mut self, style: Style) -> Self {
        self.opts.title_style = style;
        self
    }

    /// Patches the derived body text style.
    #[must_use]
    pub fn content_style(mut self, style: Style) -> Self {
        self.opts.content_style = style;
        self
    }

    /// Patches the derived surface background.
    #[must_use]
    pub fn fill(mut self, style: Style) -> Self {
        self.opts.fill = style;
        self
    }

    /// Patches the derived border style.
    #[must_use]
    pub fn border_style(mut self, style: Style) -> Self {
        self.opts.border_style = style;
        self
    }

    /// Patches the derived footer style.
    #[must_use]
    pub fn footer_style(mut self, style: Style) -> Self {
        self.opts.footer_style = style;
        self
    }
}
