//! Fluent 2 `Button`.
//!
//! Behavior (click, Enter/Space, focus, disabled semantics, a11y role) comes
//! from `gpui_base::Button`. Every visual value comes from
//! [`fluent_core::spec::button::resolve`], so this file contains no colours.

use fluent_core::spec::ControlSize;
use fluent_core::spec::button::{self, ButtonAppearance, ButtonProps, ButtonShape, IconPosition};
use fluent_theme::FluentTheme;
use gpui::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window,
};
use gpui_base::StyledExt as _;

/// A Fluent 2 button.
///
/// ```
/// # use fluent_components::Button;
/// Button::new("save").appearance(fluent_components::ButtonAppearance::Primary).child("Save");
/// ```
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    props: ButtonProps,
    children: Vec<AnyElement>,
    style: StyleRefinement,
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

    /// Reserves an icon slot; the icon element itself arrives with T-010.
    pub fn icon(mut self, position: IconPosition) -> Self {
        self.props.icon = Some(position);
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

        if let Some(label) = self.a11y_label {
            el = el.accessibility_label(label);
        }
        if let Some(handler) = self.on_click {
            el = el.on_click(handler);
        }
        for child in self.children {
            el = el.child(child);
        }

        el
    }
}