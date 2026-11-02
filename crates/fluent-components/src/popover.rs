//! Fluent 2 `Popover`.
//!
//! The trigger publishes its window-space bounds during prepaint. A deferred
//! positioner reads those bounds later in the **same frame**, placing the panel
//! above the normal tree without inheriting its clipping. Reading the anchor in
//! `render` instead would make the panel lag behind its trigger during scrolling.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
use fluent_core::spec::feedback;
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Bounds, Display, Element, GlobalElementId, InspectorElementId, LayoutId, Pixels, Point,
    Position, Size, Style, anchored, deferred, point,
};
use std::{cell::Cell, rc::Rc};

/// Shared only by the trigger and its panel for one rendered element tree.
/// Both callbacks run during prepaint, so no cross-frame state is necessary.
type TriggerBounds = Rc<Cell<Option<Bounds<Pixels>>>>;

struct PopoverPositioner {
    trigger_bounds: TriggerBounds,
    panel: AnyElement,
}

/// Clamp against the inset viewport from the outset, rather than applying the
/// margin only after crossing the window edge (which introduces an 8px jump).
fn panel_origin(
    trigger: Bounds<Pixels>,
    panel_size: Size<Pixels>,
    viewport: Size<Pixels>,
    margin: Pixels,
) -> Point<Pixels> {
    let fit = |desired: Pixels, panel: Pixels, viewport: Pixels| {
        desired.clamp(margin, (viewport - panel - margin).max(margin))
    };
    point(
        fit(trigger.left(), panel_size.width, viewport.width),
        fit(
            trigger.bottom() + px(4.),
            panel_size.height,
            viewport.height,
        ),
    )
}

impl IntoElement for PopoverPositioner {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PopoverPositioner {
    type RequestLayoutState = LayoutId;
    type PrepaintState = bool;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, LayoutId) {
        let panel_layout = self.panel.request_layout(window, cx);
        let layout = window.request_layout(
            Style {
                position: Position::Absolute,
                display: Display::Flex,
                ..Style::default()
            },
            [panel_layout],
            cx,
        );
        (layout, panel_layout)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        panel_layout: &mut LayoutId,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        // Deferred prepaint runs after the trigger's measurement callback. Do
        // not capture a copy of these bounds in render/request_layout.
        let Some(trigger) = self.trigger_bounds.get() else {
            return false;
        };
        let panel = window.layout_bounds(*panel_layout);
        let origin = panel_origin(
            trigger,
            panel.size,
            window.viewport_size(),
            px(8.) + window.client_inset().unwrap_or_default(),
        );
        // layout_bounds includes the pixel-snapped ancestor scroll offset.
        // Replace that offset, preserving GPUI's physical-pixel snapping rather
        // than rounding a relative displacement to whole logical pixels.
        let offset = window.pixel_snap_point(window.element_offset()) + origin - panel.origin;
        window.with_absolute_element_offset(offset, |window| self.panel.prepaint(window, cx));
        true
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _panel_layout: &mut LayoutId,
        positioned: &mut bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        if *positioned {
            self.panel.paint(window, cx);
        }
    }
}

/// Called with the new open state whenever the popover shows or hides.
pub type OpenChangeHandler = std::rc::Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

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
        self.on_open_change = Some(std::rc::Rc::new(handler));
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
        let backdrop_id = SharedString::from(format!("popover-backdrop-{:?}", self.id));

        let trigger_bounds = TriggerBounds::default();

        let trigger = self.trigger.map(|trigger| trigger.into_any_element());
        let content = self.content;
        let style = self.style;
        let on_change = self.on_open_change;

        let anchor = div()
            .on_children_prepainted({
                let trigger_bounds = trigger_bounds.clone();
                move |children, _, _| {
                    trigger_bounds.set(children.first().copied());
                }
            })
            .child(
                div()
                    .id(self.id)
                    .when(!controlled, |el| {
                        let store = store.clone();
                        let on_change = on_change.clone();
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

        let close_store = store.clone();
        let close_handler = on_change.clone();

        div()
            .child(anchor)
            .when(is_open, |el| {
                // Clicking anywhere outside the panel dismisses it. The backdrop
                // paints below the panel, so clicks on the panel itself never
                // reach it.
                let viewport = window.viewport_size();
                el.child(deferred(
                    anchored().child(
                        div()
                            .id(backdrop_id)
                            .w(viewport.width)
                            .h(viewport.height)
                            .on_click(move |_, window, cx| {
                                close_store.update(cx, |open, cx| {
                                    if *open {
                                        *open = false;
                                        cx.notify();
                                    }
                                });
                                if let Some(handler) = close_handler.as_ref() {
                                    handler(false, window, cx);
                                }
                            }),
                    ),
                ))
            })
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
                el.child(deferred(PopoverPositioner {
                    trigger_bounds,
                    panel: panel.into_any_element(),
                }))
            })
            .refine_style(&style)
    }
}

#[cfg(test)]
mod tests;
