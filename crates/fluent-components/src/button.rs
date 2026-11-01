//! Fluent 2 `Button`.
//!
//! Behavior (click, Enter/Space, focus, disabled semantics, a11y role) comes
//! from `gpui_base::Button`. Every visual value comes from
//! [`fluent_core::spec::button::resolve`], so this file contains no colours.

use fluent_icons::FluentIcon;
use crate::icon::Icon;
use fluent_core::spec::ControlSize;
use fluent_core::spec::button::{self, ButtonAppearance, ButtonProps, ButtonShape, IconPosition};
use fluent_theme::FluentTheme;
use fluent_core::gpui_compat::{
    AnyElement, App, BoxShadow, ClickEvent, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, point, px,
};
use fluent_core::gpui_compat::StyledExt as _;

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
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
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

    /// Marks the button disabled. Disabled buttons stay focusable only when
    /// `disabled_focusable` lands (T-020).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
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

    /// Accessible name. Falls back to the rendered label when omitted.
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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| FluentTheme::global(cx));
        let spec = button::resolve(&theme, &self.props);
        let m = spec.metrics;

        let rest = spec.rest;
        let hover = spec.hover;
        let pressed = spec.pressed;
        let disabled = spec.disabled;
        let is_disabled = self.props.disabled;

        let mut el = gpui_base::Button::new(self.id)
            .disabled(is_disabled)
            .bg(rest.background)
            .text_color(rest.foreground)
            .border_1()
            .border_color(rest.border)
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

        if is_disabled {
            el = el
                .bg(disabled.background)
                .text_color(disabled.foreground)
                .border_color(disabled.border);
        } else {
            el = el
                .hover(move |s| {
                    s.bg(hover.background)
                        .text_color(hover.foreground)
                        .border_color(hover.border)
                })
                .active(move |s| {
                    s.bg(pressed.background)
                        .text_color(pressed.foreground)
                        .border_color(pressed.border)
                });
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

        el
    }
}