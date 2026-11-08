//! The render pipeline of a [`Card`]: capabilities, regions and assembly.
//!
//! The pipeline resolves the terminal capabilities and the palette, cuts the
//! card into its three regions, and stacks their rows. The rows themselves are
//! built in [`super::rows`], the style math lives in [`super::style`].

use crate::core::border::BorderType;
use crate::core::geometry::Position;
use crate::core::render::{Renderable, Rendered};
use crate::core::terminal::{ColorSupport, UNCONSTRAINED_WIDTH, color_support};
use crate::core::theme::theme;
use crate::output::card::Card;
use crate::output::card::palette::{self, CardStyles};
use crate::output::card::rows::{Region, border_columns, edge_row, push_block};
use crate::output::card::style::{border_over, surface_of};

/// The terminal capabilities a card renders against.
///
/// Bundling them keeps the render seam at one parameter and makes both
/// degradations testable without touching the global theme or environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RenderCaps {
    /// How much color the terminal supports.
    pub support: ColorSupport,
    /// Whether Unicode glyphs may be used.
    pub unicode: bool,
}

impl RenderCaps {
    /// Returns the capabilities of a fully capable terminal.
    #[cfg(test)]
    pub(crate) fn truecolor() -> Self {
        Self {
            support: ColorSupport::TrueColor,
            unicode: true,
        }
    }
}

impl Renderable for Card {
    fn render(&self, max_width: u16) -> Rendered {
        let caps = RenderCaps {
            support: color_support(),
            unicode: theme().unicode,
        };
        self.render_with(max_width, caps)
    }
}

impl Card {
    /// Renders the card for explicit terminal capabilities.
    ///
    /// [`Renderable::render`] detects them from the environment and the theme;
    /// this is the seam that keeps both degradations testable, since under
    /// `cargo test` standard output is not a terminal and detection would
    /// always report [`ColorSupport::None`].
    pub(crate) fn render_with(
        &self,
        max_width: u16,
        caps: RenderCaps,
    ) -> Rendered {
        let border = self.effective_border(caps);
        let border_columns = border_columns(border);
        let outer = self.outer_width(max_width, border_columns) as usize;
        if outer <= border_columns {
            return Rendered::empty();
        }
        let styles = self.resolved_styles(caps.support);
        let surface = outer - border_columns;
        self.assemble(&self.regions(&styles, surface, border), border)
    }

    /// Resolves the card's outer width in columns.
    ///
    /// A card fills the width it is given, which is what makes it read as a
    /// panel rather than a label. Without a terminal there is no width to
    /// fill, so an unconstrained card falls back to its natural content width
    /// instead of stretching to [`UNCONSTRAINED_WIDTH`]. An explicit
    /// [`Card::width`] always wins, capped at what is available.
    fn outer_width(&self, max_width: u16, border_columns: usize) -> u16 {
        if let Some(width) = self.opts.width {
            return width.min(max_width);
        }
        if max_width != UNCONSTRAINED_WIDTH {
            return max_width;
        }
        let natural = self.natural_width() + border_columns;
        u16::try_from(natural).unwrap_or(UNCONSTRAINED_WIDTH)
    }

    /// Returns the widest slot's content width plus that slot's padding.
    ///
    /// The three slots pad independently, so the widest row is not simply the
    /// widest text: a narrow title with wide padding can still be the widest.
    fn natural_width(&self) -> usize {
        let optional = [
            (self.title.as_ref(), self.opts.title_padding),
            (self.footer.as_ref(), self.opts.footer_padding),
        ];
        let content =
            self.content.width() + self.opts.padding.horizontal() as usize;
        optional
            .into_iter()
            .filter_map(|(block, padding)| {
                block.map(|b| b.width() + padding.horizontal() as usize)
            })
            .chain(std::iter::once(content))
            .max()
            .unwrap_or(0)
    }

    /// Resolves the border type actually drawn.
    ///
    /// A tall border is built from half blocks whose bar only reads against a
    /// contrasting surface, so it needs both truecolor and Unicode glyphs.
    /// Without either it degrades to the heavy frame its glyph set already
    /// maps to. Other border types are returned unchanged, so a card does not
    /// diverge from [`Panel`](crate::output::panel::Panel) here.
    fn effective_border(&self, caps: RenderCaps) -> BorderType {
        if !self.opts.border.is_tall() {
            return self.opts.border;
        }
        if !caps.unicode {
            return BorderType::Ascii;
        }
        if caps.support != ColorSupport::TrueColor {
            return BorderType::Thick;
        }
        BorderType::Tall
    }

    /// Builds all rows of the card, top to bottom.
    fn assemble(&self, regions: &CardRegions, border: BorderType) -> Rendered {
        let mut lines = Vec::new();
        let has_border = !border.is_none();
        if has_border {
            lines.push(edge_row(regions.top(), Position::Top));
        }
        if let Some(title) = &self.title {
            push_block(&mut lines, &title.lines, &regions.title);
        }
        push_block(&mut lines, &self.content.lines, &regions.content);
        if let Some(footer) = &self.footer {
            push_block(&mut lines, &footer.lines, &regions.footer);
        }
        if has_border {
            lines.push(edge_row(regions.bottom(), Position::Bottom));
        }
        Rendered::new(lines)
    }

    /// Derives the palette and applies the per-slot overrides on top.
    ///
    /// The surface background is the single source for both the blank padding
    /// cells and the text cells, so a custom [`Card::fill`] reaches the body
    /// text as well; an explicit [`Card::content_style`] still wins over it.
    fn resolved_styles(&self, support: ColorSupport) -> CardStyles {
        let mut styles = palette::derive(self.opts.accent, support);
        styles.fill = styles.fill.patch(self.opts.fill);
        styles.content.bg = styles.fill.bg;
        if self.opts.flat_title {
            styles.title.bg = styles.fill.bg;
        }
        if self.opts.flat_footer {
            styles.footer.bg = styles.fill.bg;
        }
        styles.border = styles.border.patch(self.opts.border_style);
        styles.title = styles.title.patch(self.opts.title_style);
        styles.content = styles.content.patch(self.opts.content_style);
        styles.footer = styles.footer.patch(self.opts.footer_style);
        styles
    }

    /// Builds the three regions a card is made of.
    fn regions(
        &self,
        styles: &CardStyles,
        surface: usize,
        border: BorderType,
    ) -> CardRegions {
        let title_surface = surface_of(styles.title);
        let footer_surface = surface_of(styles.footer);
        CardRegions {
            title: Region {
                width: surface,
                padding: self.opts.title_padding,
                align: self.opts.title_align,
                text: styles.title,
                surface: title_surface,
                border: border_over(styles.border, title_surface),
                border_type: border,
                wrap: self.opts.wrap,
            },
            content: Region {
                width: surface,
                padding: self.opts.padding,
                align: self.opts.content_align,
                text: styles.content,
                surface: styles.fill,
                border: border_over(styles.border, styles.fill),
                border_type: border,
                wrap: self.opts.wrap,
            },
            footer: Region {
                width: surface,
                padding: self.opts.footer_padding,
                align: self.opts.footer_align,
                text: styles.footer,
                surface: footer_surface,
                border: border_over(styles.border, footer_surface),
                border_type: border,
                wrap: self.opts.wrap,
            },
            has_title: self.title.is_some(),
            has_footer: self.footer.is_some(),
        }
    }
}

/// The three regions of a card plus which optional ones are present.
struct CardRegions {
    /// The title row region.
    title: Region,
    /// The body content region.
    content: Region,
    /// The footer row region.
    footer: Region,
    /// Whether a title row is rendered.
    has_title: bool,
    /// Whether a footer row is rendered.
    has_footer: bool,
}

impl CardRegions {
    /// Returns the region the top border sits against.
    fn top(&self) -> &Region {
        if self.has_title {
            &self.title
        } else {
            &self.content
        }
    }

    /// Returns the region the bottom border sits against.
    fn bottom(&self) -> &Region {
        if self.has_footer {
            &self.footer
        } else {
            &self.content
        }
    }
}

#[cfg(test)]
mod tests;
