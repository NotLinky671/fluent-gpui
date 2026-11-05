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

/// Advances focus, keeping it inside an active focus trap.
///
/// A trap is registered by an overlay that wants Tab to cycle inside it instead
/// of escaping to the page behind. `gpui-base` publishes the active trap but its
/// own Tab handler lives on a different key context, so the walk has to be
/// repeated here.
///
/// The walk stops at the first repeated stop rather than only when it returns to
/// where it began: GPUI's tab order can cycle short of the whole window when a
/// stop's previous stop is itself.
fn advance_focus(window: &mut Window, cx: &mut App, advance: fn(&mut Window, &mut App)) {
    let Some(container) = gpui_base::active_focus_trap(window, cx) else {
        advance(window, cx);
        return;
    };

    advance(window, cx);
    let mut visited = Vec::new();
    while !container.contains_focused(window, cx) {
        let focused = window.focused(cx);
        if visited.contains(&focused) {
            // No stop in the trap is reachable; its own handle is still a safe
            // target even when excluded from ordinary tab order.
            container.focus(window, cx);
            break;
        }
        visited.push(focused);
        advance(window, cx);
    }
}

impl RenderOnce for FluentRoot {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let tokens = theme.tokens();

        // Host the anchored tooltip overlay once per window. Components reach it
        // through the same keyed state, so two windows never share one overlay.
        let overlay = crate::tooltip::register(window, cx);

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
            .on_action(|_: &crate::FocusNext, window, cx| {
                advance_focus(window, cx, gpui::Window::focus_next)
            })
            .on_action(|_: &crate::FocusPrev, window, cx| {
                advance_focus(window, cx, gpui::Window::focus_prev)
            })
            .refine_style(&self.style)
            .children(self.children)
            .child(overlay)
    }
}
