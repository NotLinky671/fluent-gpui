//! Fluent 2 `Checkbox`, `Radio` and `RadioGroup`.

mod group;
pub use group::RadioGroup;
#[cfg(test)]
mod group_tests;

use crate::icon::Icon;
use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, px,
};
use fluent_core::spec::ControlSize;
use fluent_core::spec::selection::{self, CheckboxProps, RadioProps};
use fluent_icons::FluentIcon;
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui_base::Checkbox as BaseCheckbox;
use gpui_base::CheckboxState;
use gpui_base::Radio as BaseRadio;

/// Change handler shape shared by the selection controls.
pub type ChangeHandler = Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

/// A Fluent 2 checkbox.
///
/// Uncontrolled by default; use [`Checkbox::checked`] for a controlled value, or
/// [`Checkbox::default_checked`] for an
/// uncontrolled checkbox that keeps its own state.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    controlled: bool,
    checked: bool,
    default_checked: bool,
    disabled: bool,
    size: ControlSize,
    label: Option<SharedString>,
    on_change: Option<ChangeHandler>,
    style: StyleRefinement,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            controlled: false,
            checked: false,
            default_checked: false,
            disabled: false,
            size: ControlSize::Medium,
            label: None,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    /// Controlled value: the caller owns the state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.controlled = true;
        self.checked = checked;
        self
    }

    /// Uncontrolled initial value: the control keeps its own state.
    pub fn default_checked(mut self, checked: bool) -> Self {
        self.controlled = false;
        self.default_checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Reports the next checked value.
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let store = window.use_keyed_state(
            SharedString::from(format!("checkbox-state-{:?}", self.id)),
            cx,
            |_, _| self.default_checked,
        );
        let checked = if self.controlled {
            self.checked
        } else {
            *store.read(cx)
        };

        let spec = selection::checkbox(
            &theme,
            &CheckboxProps {
                size: self.size,
                checked,
                disabled: self.disabled,
            },
        );
        let mark = spec.mark;

        // Hover and press are tracked per control and animated, so the colour
        // eases over Fluent's 100 ms instead of snapping.
        let hovered_key = SharedString::from(format!("cb-hovered-{:?}", self.id));
        let pressed_key = SharedString::from(format!("cb-pressed-{:?}", self.id));
        let hovered_state = crate::interaction::flag(hovered_key, window, cx);
        let pressed_state = crate::interaction::flag(pressed_key, window, cx);
        let hovered = *hovered_state.read(cx) && !self.disabled;
        let pressed = *pressed_state.read(cx) && !self.disabled;

        let target_background = match (pressed, hovered) {
            (true, _) => spec.pressed_background,
            (_, true) => spec.hover_background,
            _ => spec.background,
        };
        let target_border = match (pressed, hovered) {
            (true, _) => spec.pressed_border,
            (_, true) => spec.hover_border,
            _ => spec.border,
        };
        let background = crate::motion::state_color(
            SharedString::from(format!("cb-bg-{:?}", self.id)),
            target_background,
            window,
            cx,
        );
        let border = crate::motion::state_color(
            SharedString::from(format!("cb-border-{:?}", self.id)),
            target_border,
            window,
            cx,
        );

        let indicator = div()
            .flex_shrink_0()
            .size(spec.box_size)
            .rounded(spec.radius)
            .border(spec.border_width)
            .border_color(border)
            .bg(background)
            .flex()
            .items_center()
            .justify_center()
            .when(checked, |el| {
                el.child(
                    Icon::new(FluentIcon::Checkmark)
                        .size(spec.mark_size)
                        .color(mark),
                )
            });

        let content = div()
            .flex()
            .items_center()
            .gap(spec.label_gap)
            .child(indicator)
            .when_some(self.label.clone(), |el, label| el.child(label));

        let controlled = self.controlled;
        let handler = self.on_change;
        let store_for_click = store.clone();

        let mut base = BaseCheckbox::new(self.id);
        crate::interaction::hover(base.interactivity(), hovered_state);
        crate::interaction::press(base.interactivity(), pressed_state);
        base.checked(checked)
            .disabled(self.disabled)
            .child(content)
            .on_change(move |state, _, window, cx| {
                let value = state == CheckboxState::Checked;
                if !controlled {
                    store_for_click.update(cx, |state, cx| {
                        *state = value;
                        cx.notify();
                    });
                }
                if let Some(handler) = handler.as_ref() {
                    handler(&value, window, cx);
                }
            })
            .refine_style(&self.style)
    }
}

/// A Fluent 2 radio button.
#[derive(IntoElement)]
pub struct Radio {
    id: ElementId,
    group_focus: Option<gpui::FocusHandle>,
    tab_stop: bool,
    position_in_set: Option<(usize, usize)>,
    controlled: bool,
    checked: bool,
    default_checked: bool,
    disabled: bool,
    size: ControlSize,
    label: Option<SharedString>,
    on_change: Option<ChangeHandler>,
    style: StyleRefinement,
}

impl Radio {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            group_focus: None,
            tab_stop: true,
            position_in_set: None,
            controlled: false,
            checked: false,
            default_checked: false,
            disabled: false,
            size: ControlSize::Medium,
            label: None,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    /// Controlled value: the caller owns the state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.controlled = true;
        self.checked = checked;
        self
    }

    /// Uncontrolled initial value: the control keeps its own state.
    pub fn default_checked(mut self, checked: bool) -> Self {
        self.controlled = false;
        self.default_checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the visual size; small/medium use a 16px ring, large uses 20px.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl Styled for Radio {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Radio {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let store = window.use_keyed_state(
            SharedString::from(format!("radio-state-{:?}", self.id)),
            cx,
            |_, _| self.default_checked,
        );
        let checked = if self.controlled {
            self.checked
        } else {
            *store.read(cx)
        };

        let spec = selection::radio(
            &theme,
            &RadioProps {
                size: self.size,
                checked,
                disabled: self.disabled,
            },
        );

        // Same animated treatment as the checkbox.
        let hovered_state = crate::interaction::flag(
            SharedString::from(format!("radio-hovered-{:?}", self.id)),
            window,
            cx,
        );
        let pressed_state = crate::interaction::flag(
            SharedString::from(format!("radio-pressed-{:?}", self.id)),
            window,
            cx,
        );
        let hovered = *hovered_state.read(cx) && !self.disabled;
        let pressed = *pressed_state.read(cx) && !self.disabled;
        let target_border = match (pressed, hovered) {
            (true, _) => spec.pressed_border,
            (_, true) => spec.hover_border,
            _ => spec.border,
        };
        let target_dot = match (pressed, hovered) {
            (true, _) => spec.pressed_dot,
            (_, true) => spec.hover_dot,
            _ => spec.dot_color,
        };
        let border = crate::motion::state_color(
            SharedString::from(format!("radio-border-{:?}", self.id)),
            target_border,
            window,
            cx,
        );
        let dot_color = crate::motion::state_color(
            SharedString::from(format!("radio-dot-{:?}", self.id)),
            target_dot,
            window,
            cx,
        );
        // Ring painted as an inset overlay: keeping the border out of layout
        // means flex centring puts the dot exactly in the middle.
        let indicator = div()
            .relative()
            .flex_shrink_0()
            .size(spec.circle)
            .rounded(spec.radius)
            .flex()
            .items_center()
            .justify_center()
            .when(checked, |el| {
                el.child(
                    div()
                        .flex_shrink_0()
                        .size(spec.dot)
                        .rounded(spec.radius)
                        .bg(dot_color),
                )
            })
            .child(
                div()
                    .absolute()
                    .top(px(0.))
                    .left(px(0.))
                    .size(spec.circle)
                    .rounded(spec.radius)
                    .border(spec.border_width)
                    .border_color(border),
            );

        let content = div()
            .flex()
            .items_center()
            .gap(spec.label_gap)
            .child(indicator)
            .when_some(self.label.clone(), |el, label| el.child(label));

        let controlled = self.controlled;
        let handler = self.on_change;
        let store_for_click = store.clone();

        let mut base = BaseRadio::new(self.id.clone()).tab_stop(self.tab_stop);
        if let Some(focus) = self.group_focus {
            base = base.track_focus(&focus);
        }
        if let Some((position, count)) = self.position_in_set {
            base = base.set_position(position, count);
        }
        if let Some(label) = &self.label {
            base = base.accessibility_label(label.clone());
        }
        crate::interaction::hover(base.interactivity(), hovered_state);
        crate::interaction::press(base.interactivity(), pressed_state);
        base.checked(checked)
            .disabled(self.disabled)
            .child(content)
            .on_change(move |value, _, window, cx| {
                if !controlled {
                    store_for_click.update(cx, |state, cx| {
                        *state = value;
                        cx.notify();
                    });
                }
                if let Some(handler) = handler.as_ref() {
                    handler(&value, window, cx);
                }
            })
            .refine_style(&self.style)
    }
}
