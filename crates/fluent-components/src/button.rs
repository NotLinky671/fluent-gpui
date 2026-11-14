//! Fluent 2 `Button`.
//!
//! Behavior (click, Enter/Space, focus, disabled semantics, a11y role) comes
//! from `gpui_base::Button`. Every visual value comes from
//! [`fluent_core::spec::button::resolve`], so this file contains no colours.

use crate::icon::Icon;
use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, BoxShadow, ClickEvent, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, div, point, px,
};
use fluent_core::spec::ControlSize;
use fluent_core::spec::button::{self, ButtonAppearance, ButtonProps, ButtonShape, IconPosition};
use fluent_icons::FluentIcon;
use fluent_theme::FluentTheme;
use gpui::AppContext as _;

/// Click handler shape, matching `gpui_base::Button::on_click`.
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// A Fluent 2 button.
///
/// ```
/// use fluent_components::{Button, ButtonAppearance};
/// use gpui::ParentElement as _;
///
/// Button::new("save")
///     .appearance(ButtonAppearance::Primary)
///     .child("Save");
/// ```
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    props: ButtonProps,
    children: Vec<AnyElement>,
    style: StyleRefinement,
    icon: Option<FluentIcon>,
    tooltip: Option<SharedString>,
    on_click: Option<ClickHandler>,
    a11y_label: Option<SharedString>,
    theme: Option<FluentTheme>,
}

impl Button {
    /// Creates a button. `id` must stay stable across renders so GPUI can keep
    /// focus and element state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            props: ButtonProps::default(),
            children: Vec::new(),
            style: StyleRefinement::default(),
            icon: None,
            tooltip: None,
            on_click: None,
            a11y_label: None,
            theme: None,
        }
    }

    /// Fluent `appearance` prop.
    pub fn appearance(mut self, appearance: ButtonAppearance) -> Self {
        self.props.appearance = appearance;
        self
    }

    /// Fluent `size` prop.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.props.size = size;
        self
    }

    /// Fluent `shape` prop.
    pub fn shape(mut self, shape: ButtonShape) -> Self {
        self.props.shape = shape;
        self
    }

    /// Marks the button disabled. Disabled buttons do not take focus.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Shows a Fluent tooltip while the button is hovered.
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Places an icon before the label.
    pub fn icon(mut self, icon: FluentIcon) -> Self {
        self.icon = Some(icon);
        self.props.icon = Some(IconPosition::Before);
        self
    }

    /// Places an icon after the label.
    pub fn icon_after(mut self, icon: FluentIcon) -> Self {
        self.icon = Some(icon);
        self.props.icon = Some(IconPosition::After);
        self
    }

    /// Square, icon-only button.
    pub fn icon_only(mut self) -> Self {
        self.props.icon_only = true;
        self
    }

    /// Explicit accessible name.
    ///
    /// There is no automatic fallback: `.child(..)` takes an opaque
    /// `AnyElement`, so text children (and any other custom content) cannot be
    /// read back or turned into a name. Text buttons must pass the same
    /// `SharedString` here that they render; icon-only buttons pass a short
    /// action description. This does not affect the visible label.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.a11y_label = Some(label.into());
        self
    }

    /// Click handler, in the same shape as `gpui_base::Button::on_click`.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Renders against a specific theme instead of the global one.
    pub fn theme(mut self, theme: &FluentTheme) -> Self {
        self.theme = Some(theme.clone());
        self
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| FluentTheme::global(cx));
        let spec = button::resolve(&theme, &self.props);
        let m = spec.metrics;

        let rest = spec.rest;
        let hover = spec.hover;
        let pressed = spec.pressed;
        let disabled = spec.disabled;
        let is_disabled = self.props.disabled;

        // Hover and press drive animated colours: gpui-base's own .hover() and
        // .active() restyle instantly, which reads as a hard snap next to the
        // rest of the system.
        //
        // `on_hover`'s reported edge is unreliable inside `deferred`: its
        // bootstrap reads `is_hovered` against the *previous* frame's
        // `mouse_hit_test`, so a freshly repainted hitbox reports a spurious
        // `false` for one frame even with a stationary pointer — that edge is
        // the footer flicker. Keep `on_hover` as the occlusion-aware source of
        // real enter/leave edges, but latch `true`: a `false` edge is only
        // accepted when the live pointer is actually outside the button's
        // recorded bounds, so a stale-read `false` cannot flash the surface
        // while the pointer is still on it.
        let hover_state = window.use_keyed_state(
            SharedString::from(format!("button-hovered-{:?}", self.id)),
            cx,
            |_, _| false,
        );
        let hover_bounds = window.use_keyed_state(
            SharedString::from(format!("button-bounds-{:?}", self.id)),
            cx,
            |_, _| std::rc::Rc::new(std::cell::Cell::new(None::<gpui::Bounds<gpui::Pixels>>)),
        );
        let press_state = window.use_keyed_state(
            SharedString::from(format!("button-pressed-{:?}", self.id)),
            cx,
            |_, _| false,
        );
        let hovered = *hover_state.read(cx) && !is_disabled;
        let is_pressed = *press_state.read(cx) && !is_disabled;

        let (target_background, target_foreground, target_border) = if is_disabled {
            (disabled.background, disabled.foreground, disabled.border)
        } else if is_pressed {
            (pressed.background, pressed.foreground, pressed.border)
        } else if hovered {
            (hover.background, hover.foreground, hover.border)
        } else {
            (rest.background, rest.foreground, rest.border)
        };
        let background = crate::motion::state_color(
            SharedString::from(format!("button-bg-{:?}", self.id)),
            target_background,
            window,
            cx,
        );
        let foreground = crate::motion::state_color(
            SharedString::from(format!("button-fg-{:?}", self.id)),
            target_foreground,
            window,
            cx,
        );
        let stroke = crate::motion::state_color(
            SharedString::from(format!("button-border-{:?}", self.id)),
            target_border,
            window,
            cx,
        );

        // The element id is consumed by the base button; keyed state below the
        // tooltip wiring still needs it.
        let element_id = self.id.clone();
        let mut el = gpui_base::Button::new(self.id)
            .disabled(is_disabled)
            .bg(background)
            .text_color(foreground)
            .border_1()
            .border_color(stroke)
            .rounded(m.radius)
            .text_size(m.font_size)
            .line_height(m.line_height)
            .font_weight(m.font_weight)
            .min_w(m.min_width)
            .px(m.padding_x)
            .py(m.padding_y)
            .gap(m.icon_gap)
            .cursor_pointer()
            // Caller overrides apply last (ADR-0002, layer 3).
            .refine_style(&self.style);

        if !is_disabled {
            crate::interaction::press(el.interactivity(), press_state);
        }

        // Fluent's button focus indicator is inset: the border turns focus
        // colour and a 1px ring is drawn just inside it (plus a 2px on-brand
        // ring for the primary appearance). Inset shadows keep layout intact,
        // so the control's bounds never change when focus arrives.
        let focus = spec.focus;
        if !is_disabled {
            el = el.focus_visible(move |s| {
                let mut shadows = vec![BoxShadow {
                    color: focus.inner_ring_color,
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                    spread_radius: focus.inner_ring_width,
                    inset: true,
                }];
                if let Some(color) = focus.outer_ring_color {
                    shadows.push(BoxShadow {
                        color,
                        offset: point(px(0.), px(0.)),
                        blur_radius: px(0.),
                        spread_radius: focus.outer_ring_width,
                        inset: true,
                    });
                }
                s.border_color(focus.border_color).shadow(shadows)
            });
        }

        if let Some(label) = self.a11y_label {
            el = el.accessibility_label(label);
        }

        let tooltip_bounds: Option<
            std::rc::Rc<std::cell::Cell<Option<gpui::Bounds<gpui::Pixels>>>>,
        > = self
            .tooltip
            .as_ref()
            .map(|_| std::rc::Rc::new(std::cell::Cell::new(None::<gpui::Bounds<gpui::Pixels>>)));
        if let Some(text) = self.tooltip {
            let slot_hover = tooltip_bounds.clone().unwrap();
            let bounds_for_edge = hover_bounds.read(cx).clone();
            let state_for_edge = hover_state.clone();
            // Whether the overlay currently holds a request from this button.
            // This has to outlive a repaint: a render-local flag starts as `false`
            // again on the repaint that follows the show, so the leave edge would
            // see `requested == false` and never ask for a hide — the tooltip then
            // stays on screen after the pointer is gone.
            let tooltip_shown = window.use_keyed_state(
                SharedString::from(format!("button-tooltip-shown-{:?}", element_id)),
                cx,
                |_, _| false,
            );
            el = el.on_hover(move |reported, window, cx| {
                // The reported edge can be a one-frame stale-read `false` while
                // the pointer is still on the button; only honour a `false`
                // when the pointer is genuinely outside the recorded bounds.
                window.refresh();
                let inside = bounds_for_edge
                    .get()
                    .map(|b| b.contains(&window.mouse_position()))
                    .unwrap_or(false);
                let genuine = *reported || inside;
                crate::interaction::set(&state_for_edge, genuine, cx);
                let Some(overlay) = crate::tooltip::overlay(window, cx) else {
                    return;
                };
                let shown = *tooltip_shown.read(cx);
                if genuine && !shown {
                    let Some(bounds) = slot_hover.get() else {
                        return;
                    };
                    tooltip_shown.update(cx, |shown, cx| {
                        *shown = true;
                        cx.notify();
                    });
                    let text = text.clone();
                    overlay.update(cx, |overlay, cx| {
                        overlay.request_show(
                            // Fluent leaves a 4px gap between trigger and tooltip.
                            // TooltipPositioner does not expose Positioner::offset,
                            // so dilate the bounds by the gap instead.
                            gpui_base::TooltipRequest::new(bounds.dilate(px(4.)), move |_, cx| {
                                cx.new(|_| crate::feedback::TooltipView::new(text.clone()))
                                    .into()
                            }),
                            window,
                            cx,
                        );
                    });
                } else if !genuine && shown {
                    tooltip_shown.update(cx, |shown, cx| {
                        *shown = false;
                        cx.notify();
                    });
                    overlay.update(cx, |overlay, cx| overlay.request_hide(window, cx));
                }
            });
        } else if !is_disabled {
            let bounds_for_edge = hover_bounds.read(cx).clone();
            let state_for_edge = hover_state.clone();
            el = el.on_hover(move |reported, window, cx| {
                window.refresh();
                let inside = bounds_for_edge
                    .get()
                    .map(|b| b.contains(&window.mouse_position()))
                    .unwrap_or(false);
                let genuine = *reported || inside;
                crate::interaction::set(&state_for_edge, genuine, cx);
            });
        }
        if let Some(handler) = self.on_click {
            el = el.on_click(handler);
        }
        // Icon placement follows the spec's padding rules; the icon inherits
        // the button's text colour, so hover/pressed tint it automatically.
        let icon = self.icon.map(|icon| Icon::new(icon).size(m.icon_size));
        if matches!(self.props.icon, Some(IconPosition::Before)) {
            if let Some(icon) = icon {
                el = el.child(icon);
            }
            for child in self.children {
                el = el.child(child);
            }
        } else {
            for child in self.children {
                el = el.child(child);
            }
            if let Some(icon) = icon {
                el = el.child(icon);
            }
        }
        // Always wrap so `on_children_prepainted` records the base button's
        // painted bounds (the geometry the `on_hover` edge is validated
        // against) and, when present, `tooltip_bounds` (where it anchors).
        let hover_bounds_out = hover_bounds.read(cx).clone();
        let tooltip_bounds_out = tooltip_bounds.clone();
        div()
            .on_children_prepainted(move |children, _, _| {
                if let Some(bounds) = children.first() {
                    hover_bounds_out.set(Some(*bounds));
                    if let Some(slot) = &tooltip_bounds_out {
                        slot.set(Some(*bounds));
                    }
                }
            })
            .child(el)
            .into_any_element()
    }
}

#[cfg(test)]
mod hover_tests;
#[cfg(test)]
mod icon_tests;

#[cfg(test)]
mod tooltip_tests;
