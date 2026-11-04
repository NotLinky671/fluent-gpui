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
    Position, Size, Style, deferred, point,
};

use std::{cell::Cell, rc::Rc};

/// Shared only by the trigger and its panel for one rendered element tree.
/// Both callbacks run during prepaint, so no cross-frame state is necessary.
type TriggerBounds = Rc<Cell<Option<Bounds<Pixels>>>>;

struct PopoverPositioner {
    trigger_bounds: TriggerBounds,
    /// Written back during prepaint so the panel's own mouse-out handler knows
    /// where the surface ended up.
    panel_bounds: TriggerBounds,
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
        self.panel_bounds.set(Some(Bounds::new(origin, panel.size)));
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
    /// Whether `Popover` draws the panel surface, or the content brings its own.
    draw_surface: bool,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<OpenChangeHandler>,
    focus: Option<gpui::FocusHandle>,
    style: StyleRefinement,
}

impl Popover {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            content: None,
            draw_surface: true,
            open: None,
            default_open: false,
            on_open_change: None,
            focus: None,
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

    /// Whether `Popover` draws the panel surface behind the content.
    ///
    /// Panels that bring their own surface ([`Menu`](crate::Menu) does) must turn
    /// this off: otherwise the panel padding, radius, background and shadow stack
    /// a second card around the content and inset its rows.
    pub fn draw_panel_surface(mut self, draw: bool) -> Self {
        self.draw_surface = draw;
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

    /// Focus handle used for keyboard dismissal.
    ///
    /// Escape only reaches the popover while focus is inside it, so an overlay
    /// must own a focus handle to be dismissable from the keyboard.
    pub fn track_focus(mut self, focus: &gpui::FocusHandle) -> Self {
        self.focus = Some(focus.clone());
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
        let draw_surface = self.draw_surface;

        let store = window.use_keyed_state(
            SharedString::from(format!("popover-open-{:?}", self.id)),
            cx,
            |_, _| self.default_open,
        );
        let is_open = self.open.unwrap_or_else(|| *store.read(cx));
        let controlled = self.open.is_some();

        let trigger_bounds = TriggerBounds::default();

        let trap_id = SharedString::from(format!("popover-trap-{:?}", self.id));
        let trigger = self.trigger.map(|trigger| trigger.into_any_element());
        let content = self.content;
        let style = self.style;
        let on_change = self.on_open_change;
        let focus = self.focus.clone();

        // Focus handover. Opening moves focus into the panel and remembers what
        // had it; closing gives it back only if it is still inside, so a click
        // that focused something else is not undone.
        if let Some(panel_focus) = focus.as_ref() {
            let previous = window.use_keyed_state(
                SharedString::from(format!("popover-previous-focus-{:?}", self.id)),
                cx,
                |_, _| None::<gpui::FocusHandle>,
            );
            let was_open = window.use_keyed_state(
                SharedString::from(format!("popover-was-open-{:?}", self.id)),
                cx,
                |_, _| false,
            );
            let previously_open = *was_open.read(cx);
            if is_open && !previously_open {
                let current = window.focused(cx);
                previous.update(cx, |slot, _| *slot = current);
                panel_focus.focus(window, cx);
                was_open.update(cx, |open, _| *open = true);
            } else if !is_open && previously_open {
                let restore = previous.update(cx, |slot, _| slot.take());
                if panel_focus.contains_focused(window, cx)
                    && let Some(handle) = restore
                {
                    handle.focus(window, cx);
                }
                was_open.update(cx, |open, _| *open = false);
            }
        }

        let panel_focus = focus.clone();
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
                    .when(is_open, |el| {
                        let store = store.clone();
                        let on_change = on_change.clone();
                        el.on_action(move |_: &crate::Dismiss, window, cx| {
                            store.update(cx, |open, cx| {
                                if *open {
                                    *open = false;
                                    cx.notify();
                                }
                            });
                            if let Some(handler) = on_change.as_ref() {
                                handler(false, window, cx);
                            }
                        })
                    })
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
                    .key_context(crate::OVERLAY_KEY_CONTEXT)
                    .when_some(focus, |el, focus| el.track_focus(&focus))
                    .when_some(trigger, |el, trigger| el.child(trigger)),
            );

        let close_store = store.clone();
        let close_handler = on_change.clone();
        let panel_bounds: TriggerBounds = TriggerBounds::default();
        let close_bounds = panel_bounds.clone();

        div()
            .child(anchor)
            .when(is_open, |el| {
                let mut panel = div().whitespace_nowrap();
                if draw_surface {
                    panel = panel
                        .min_w(px(160.))
                        .rounded(px(4.))
                        .bg(surface.background)
                        .text_color(surface.foreground)
                        .p(px(12.))
                        .shadow(shadow);
                }
                // Blocking here is what makes "clicks inside the panel keep it
                // open" true rather than merely intended by paint order.
                panel = panel.occlude().key_context(crate::OVERLAY_KEY_CONTEXT);
                {
                    let close_store = close_store.clone();
                    let close_handler = close_handler.clone();
                    let close_bounds = close_bounds.clone();
                    let close_trigger = trigger_bounds.clone();
                    panel = panel.on_mouse_down_out(move |event, window, cx| {
                        // A press that lands on the trigger belongs to the trigger's
                        // own toggle. Closing here too would report the same press
                        // twice and reopen what the user just dismissed.
                        let on_trigger = close_bounds
                            .get()
                            .map(|panel| !panel.contains(&event.position))
                            .unwrap_or(false)
                            && close_trigger
                                .get()
                                .map(|trigger| trigger.contains(&event.position))
                                .unwrap_or(false);
                        if on_trigger {
                            return;
                        }
                        close_store.update(cx, |open, cx| {
                            if *open {
                                *open = false;
                                cx.notify();
                            }
                        });
                        if let Some(handler) = close_handler.as_ref() {
                            handler(false, window, cx);
                        }
                    });
                }
                // Escape must work from inside the panel too: once focus moves
                // here the trigger wrapper is no longer an ancestor of the
                // focused element, so its handler cannot receive the action.
                {
                    let close_store = close_store.clone();
                    let close_handler = close_handler.clone();
                    panel = panel.on_action(move |_: &crate::Dismiss, window, cx| {
                        close_store.update(cx, |open, cx| {
                            if *open {
                                *open = false;
                                cx.notify();
                            }
                        });
                        if let Some(handler) = close_handler.as_ref() {
                            handler(false, window, cx);
                        }
                    });
                }
                if let Some(content) = content {
                    panel = panel.child(content);
                }
                // With a focus handle the panel also traps Tab, so the keyboard
                // stays in the popover instead of walking into the page behind it.
                // Without one there is nothing to trap around.
                let panel: AnyElement = match panel_focus.as_ref() {
                    Some(panel_focus) => {
                        use gpui_base::FocusTrapElement as _;
                        panel.focus_trap(trap_id, panel_focus).into_any_element()
                    }
                    None => panel.into_any_element(),
                };
                el.child(deferred(PopoverPositioner {
                    trigger_bounds,
                    panel_bounds,
                    panel,
                }))
            })
            .refine_style(&style)
    }
}
#[cfg(test)]
mod dismissal_tests;
#[cfg(test)]
mod focus_probe_tests;
#[cfg(test)]
mod focus_trap_tests;
#[cfg(test)]
mod tests;
