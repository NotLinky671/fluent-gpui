//! A Fluent 2 context menu: the `Menu` panel, opened where the pointer pressed.
//!
//! The rows, keyboard model, typeahead and submenus are the menu's own; this
//! module only adds a right-press entry point and a point anchor, so a context
//! menu and a menu button cannot drift apart in behaviour.

use crate::menu::Menu;
use crate::popover::{InteriorRegions, OpenChangeHandler, PointAnchor, Popover};
use fluent_core::gpui_compat::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};
use gpui::prelude::FluentBuilder as _;
use gpui::{MouseButton, MouseDownEvent};
use std::cell::Cell;
use std::rc::Rc;

/// A menu that opens at the pointer, over the region it was attached to.
#[derive(IntoElement)]
pub struct ContextMenu {
    id: SharedString,
    /// The area that answers a right press. Any element: usually a container
    /// around the content the menu acts on.
    region: Option<AnyElement>,
    menu: Menu,
    open: Option<bool>,
    default_open: bool,
    focus: Option<gpui::FocusHandle>,
    on_open_change: Option<OpenChangeHandler>,
}

impl ContextMenu {
    pub fn new(id: impl Into<SharedString>, menu: Menu) -> Self {
        Self {
            id: id.into(),
            region: None,
            menu,
            open: None,
            default_open: false,
            focus: None,
            on_open_change: None,
        }
    }

    /// The element whose area answers a right press.
    pub fn region(mut self, region: impl IntoElement) -> Self {
        self.region = Some(region.into_any_element());
        self
    }

    /// Controls the open state. Uncontrolled when unset.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Focus handle for the panel. One is created when the caller has none.
    pub fn track_focus(mut self, focus: &gpui::FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let open_store = window.use_keyed_state(
            SharedString::from(format!("context-menu-open-{}", id)),
            cx,
            |_, _| self.default_open,
        );
        // The press writes the point and the deferred positioner reads it in the
        // same frame, so the panel never paints once at a stale anchor.
        let pointer: PointAnchor = window
            .use_keyed_state(
                SharedString::from(format!("context-menu-point-{}", id)),
                cx,
                |_, _| Rc::new(Cell::new(None)),
            )
            .read(cx)
            .clone();
        let panel_focus = self.focus.clone().unwrap_or_else(|| {
            window
                .use_keyed_state(
                    SharedString::from(format!("context-menu-focus-{}", id)),
                    cx,
                    |_, cx| cx.focus_handle(),
                )
                .read(cx)
                .clone()
        });
        let open = self.open.unwrap_or_else(|| *open_store.read(cx));
        // A submenu is painted beyond the panel's own hitbox. Publishing its
        // regions keeps a press there from counting as an outside press, which
        // is exactly what `MenuDropdown` does for the same reason.
        let interior = InteriorRegions::default();
        let menu = self
            .menu
            .track_focus(&panel_focus)
            .interior_regions(interior.clone());

        let region = {
            let pointer = pointer.clone();
            let open_store = open_store.clone();
            let handler = self.on_open_change.clone();
            div()
                .id(ElementId::from(SharedString::from(format!(
                    "context-menu-region-{}",
                    id
                ))))
                // Pinned to the layer instead of sized with `size_full`: a
                // percentage height inside a flex column resolves against a
                // height that is still being negotiated, which collapses the
                // layer and leaves the region unable to receive a press.
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .on_mouse_down(
                    MouseButton::Right,
                    move |event: &MouseDownEvent, window, cx| {
                        // A nested region with its own menu must not also open
                        // the one above it.
                        cx.stop_propagation();
                        pointer.set(Some(event.position));
                        if !*open_store.read(cx) {
                            open_store.update(cx, |slot, cx| {
                                *slot = true;
                                cx.notify();
                            });
                            if let Some(handler) = handler.as_ref() {
                                handler(true, window, cx);
                            }
                        }
                        window.refresh();
                    },
                )
                .when_some(self.region, |el, region| el.child(region))
        };

        // Rendered as a sibling rather than as the popover's trigger: the region
        // is the whole area the menu is attached to, so handing it to the
        // popover as a trigger would make every press in that area "on the
        // trigger" and never dismiss.
        let popover = Popover::new(id.clone())
            .anchor_point(pointer)
            // The menu is the surface: a panel chrome around it would stack a
            // second card and inset every row.
            .draw_panel_surface(false)
            .interior_regions(interior)
            .content(menu)
            .open(open)
            .track_focus(&panel_focus)
            .on_open_change(move |next, window, cx| {
                open_store.update(cx, |slot, _| *slot = next);
                if let Some(handler) = self.on_open_change.as_ref() {
                    handler(next, window, cx);
                }
            });

        // The attachment overlays the host instead of taking a slot in its
        // layout. Sized with `size_full` it shared the host's height with the
        // host's own children: in a card that already had a full-size label,
        // the region covered only the lower half, so a press in the upper half
        // did nothing. The host must be a positioned container for the overlay
        // to stay scoped to it.
        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .child(region)
            .child(popover.render(window, cx))
    }
}

#[cfg(test)]
mod tests;
