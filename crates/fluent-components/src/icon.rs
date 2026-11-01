//! Fluent icon element.
//!
//! Renders [`FluentIcon`] SVG data through GPUI's `svg()` element. Colour is
//! inherited from the surrounding text style unless [`Icon::color`] is set, so
//! an icon inside a button follows the button's hover/pressed foreground for
//! free.

use fluent_core::gpui_compat::{
    App, Hsla, IntoElement, Pixels, RenderOnce, Styled, Window, px, svg,
};
use fluent_icons::{FluentIcon, IconStyle};
use fluent_theme::FluentTheme;

/// A single Fluent icon.
#[derive(IntoElement)]
pub struct Icon {
    icon: FluentIcon,
    style: IconStyle,
    size: Pixels,
    color: Option<Hsla>,
}

impl Icon {
    /// Creates a 20px regular icon.
    pub fn new(icon: FluentIcon) -> Self {
        Self {
            icon,
            style: IconStyle::Regular,
            size: px(20.),
            color: None,
        }
    }

    /// Switches between regular and filled.
    pub fn style(mut self, style: IconStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the icon box; Fluent uses 20px (small/medium) and 24px (large).
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    /// Overrides the inherited text colour.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // GPUI's `Svg` only paints when the element carries an explicit text
        // colour (`svg.rs`: `if let Some(color) = style.text.color`), so an
        // inherited colour would leave the icon laid out but invisible.
        let color = self
            .color
            .unwrap_or_else(|| FluentTheme::global(cx).tokens().color.neutral_foreground_1);
        svg()
            .data(self.icon.svg(self.style))
            .w(self.size)
            .h(self.size)
            .text_color(color)
    }
}
