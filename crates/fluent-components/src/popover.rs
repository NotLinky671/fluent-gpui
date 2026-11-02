//! Fluent 2 `Popover`.
//!
//! The panel is positioned **relative to the trigger**, inside the same layout
//! box. gpui-base's `Popup` anchors in window coordinates while painting inside
//! the scroll layer, so scrolling applied the offset twice and the panel drifted
//! against its trigger. Relative positioning has no second coordinate space to
//! disagree with, so it stays locked.
//!
//! Trade-off: the panel is clipped by an enclosing scroll container. A portal to
//! the root would avoid that and is the next step if it becomes a problem.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
use fluent_core::spec::feedback;
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;

/// Called with the new open state whenever the popover shows or hides.
pub type OpenChangeHandler = Box<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// A Fluent 2 popover: a floating panel anchored to a trigger.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    trigger: Option<AnyElement>,
    content: Option<AnyElement>,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<OpenChangeHandler>,
    style: StyleRefinement,
}

impl Popover {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            content: None,
            open: None,
            default_open: false,
            on_open_change: None,
            style: StyleRefinement::default(),
        }
    }

    /// The element that toggles the popover.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// The floating panel's body.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    /// Controlled open state: the caller owns it.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Uncontrolled initial state.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Box::new(handler));
        self
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let surface = feedback::tooltip(&theme);
        let shadow = surface.shadow.to_vec();

        let store = window.use_keyed_state(
            SharedString::from(format!("popover-open-{:?}", self.id)),
            cx,
            |_, _| self.default_open,
        );
        let is_open = self.open.unwrap_or_else(|| *store.read(cx));

        let trigger = self.trigger;
        let content = self.content;
        let controlled = self.open.is_some();
        let style = self.style;

        let root = div()
            .relative()
            .child(
                div()
                    .id(self.id)
                    .when_some(trigger, |el, trigger| el.child(trigger))
                    .when(!controlled, |el| {
                        let store = store.clone();
                        let handler = self.on_open_change;
                        el.on_click(move |_, window, cx| {
                            let mut next = false;
                            store.update(cx, |open, cx| {
                                *open = !*open;
                                next = *open;
                                cx.notify();
                            });
                            if let Some(handler) = handler.as_ref() {
                                handler(next, window, cx);
                            }
                        })
                    }),
            )
            .when(is_open, |el| {
                let mut panel = div()
                    .absolute()
                    .top_full()
                    .left(px(0.))
                    .mt(px(4.))
                    // An absolutely positioned box inherits the trigger's width as
                    // its available space; without these it shrinks and the text
                    // wraps.
                    .min_w(px(160.))
                    .whitespace_nowrap()
                    .rounded(px(4.))
                    .bg(surface.background)
                    .text_color(surface.foreground)
                    .p(px(12.))
                    .shadow(shadow);
                if let Some(content) = content {
                    panel = panel.child(content);
                }
                el.child(panel)
            });

        root.refine_style(&style)
    }
}
