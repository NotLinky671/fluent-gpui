//! Fluent 2 `Popover`.
//!
//! The panel is painted with `deferred(anchored(..))`: `anchored` interprets its
//! position in **window** coordinates and `deferred` lifts the element out of any
//! scrolling ancestor, so the position and the paint space agree. gpui-base's
//! `Popup` mixes the two — a window-space anchor painted through the scroll
//! transform — which made the panel drift against its trigger while scrolling.
//!
//! This is the same primitive gpui-component's menus use.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
use fluent_core::spec::feedback;
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui::{Bounds, Pixels, anchored, deferred, point};

/// Called with the new open state whenever the popover shows or hides.
pub type OpenChangeHandler = Box<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// A Fluent 2 popover: a floating panel anchored below a trigger.
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
        let controlled = self.open.is_some();

        // The trigger's window-space bounds. Kept in window keyed state, not in
        // an Rc: a cell rebuilt每 render would lose the value the prepaint wrote
        // and the panel would stay at the default origin.
        let trigger_bounds = window.use_keyed_state(
            SharedString::from(format!("popover-anchor-{:?}", self.id)),
            cx,
            |_, _| None::<Bounds<Pixels>>,
        );
        let anchor_bounds = *trigger_bounds.read(cx);

        let trigger = self.trigger.map(|trigger| trigger.into_any_element());
        let content = self.content;
        let style = self.style;
        let on_change = self.on_open_change;

        let anchor = div()
            .on_children_prepainted({
                let trigger_bounds = trigger_bounds.clone();
                move |children, _, cx| {
                    let Some(child) = children.first() else {
                        return;
                    };
                    trigger_bounds.update(cx, |bounds, cx| {
                        if *bounds != Some(*child) {
                            *bounds = Some(*child);
                            cx.notify();
                        }
                    });
                }
            })
            .child(
                div()
                    .id(self.id)
                    .when(!controlled, |el| {
                        let store = store.clone();
                        let on_change = on_change;
                        el.on_click(move |_, window, cx| {
                            let mut next = false;
                            store.update(cx, |open, cx| {
                                *open = !*open;
                                next = *open;
                                cx.notify();
                            });
                            if let Some(handler) = on_change.as_ref() {
                                handler(next, window, cx);
                            }
                        })
                    })
                    .when_some(trigger, |el, trigger| el.child(trigger)),
            );

        div()
            .child(anchor)
            .when(is_open, |el| {
                let mut panel = div()
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
                let origin = anchor_bounds
                    .map(|bounds: Bounds<Pixels>| {
                        point(
                            bounds.origin.x,
                            bounds.origin.y + bounds.size.height + px(4.),
                        )
                    })
                    .unwrap_or_default();
                el.child(deferred(anchored().position(origin).child(panel)))
            })
            .refine_style(&style)
    }
}
