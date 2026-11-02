//! Root surface for a Fluent application.
//!
//! GPUI has no global text style, so an application has to apply the theme's
//! surface colour, foreground and UI font once, at the top of its tree.
//! [`FluentRoot`] does exactly that and nothing else.

use fluent_core::gpui_compat::InteractiveElement as _;
use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div,
    px,
};
use fluent_theme::FluentTheme;
use gpui_base::TooltipOverlay;

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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let tokens = theme.tokens();

        // Host the anchored tooltip overlay once per window and publish it so
        // components can request tooltips with their own trigger bounds.
        let overlay =
            window.use_keyed_state("fluent-tooltip-overlay", cx, |_, _| TooltipOverlay::new());
        cx.set_global(crate::tooltip::TooltipService {
            overlay: overlay.clone(),
        });

        div()
            .size_full()
            .bg(tokens.color.neutral_background_1)
            .text_color(tokens.color.neutral_foreground_1)
            .font_family(theme.font_family().clone())
            .text_size(px(14.))
            // Tab order: gpui-base binds tab to its own Root context, which only
            // fires under its own root view. Carrying this context here is what
            // lets the binding reach controls mounted under FluentRoot.
            .key_context(crate::ROOT_KEY_CONTEXT)
            .on_action(|_: &crate::FocusNext, window, cx| window.focus_next(cx))
            .on_action(|_: &crate::FocusPrev, window, cx| window.focus_prev(cx))
            .refine_style(&self.style)
            .children(self.children)
            .child(overlay)
    }
}
