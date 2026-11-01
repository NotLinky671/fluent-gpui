//! Root surface for a Fluent application.
//!
//! GPUI has no global text style, so an application has to apply the theme's
//! surface colour, foreground and UI font once, at the top of its tree.
//! [`FluentRoot`] does exactly that and nothing else.

use fluent_core::gpui_compat::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div,
    px,
};
use fluent_core::gpui_compat::StyledExt as _;
use fluent_theme::FluentTheme;

/// Applies the theme's background, foreground and font family to a subtree.
///
/// ```
/// use fluent_components::FluentRoot;
/// use gpui::ParentElement as _;
///
/// FluentRoot::new().child("Hello");
/// ```
#[derive(IntoElement)]
pub struct FluentRoot {
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl FluentRoot {
    /// Creates an empty root.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }
}

impl Default for FluentRoot {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for FluentRoot {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for FluentRoot {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for FluentRoot {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let tokens = theme.tokens();
        div()
            .size_full()
            .bg(tokens.color.neutral_background_1)
            .text_color(tokens.color.neutral_foreground_1)
            .font_family(theme.font_family().clone())
            .text_size(px(14.))
            .refine_style(&self.style)
            .children(self.children)
    }
}