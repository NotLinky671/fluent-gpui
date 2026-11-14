//! Fluent 2 `Switch`.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div,
};
use fluent_core::spec::ControlSize;
use fluent_core::spec::switch::{self, SwitchProps};
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui_base::Switch as BaseSwitch;

pub use crate::selection::ChangeHandler;

/// A Fluent 2 switch.
///
/// Uncontrolled by default. Pass [`Switch::checked`] and update it in
/// [`Switch::on_change`]. Use [`Switch::default_checked`] for an uncontrolled
/// switch that keeps its own state.
#[derive(IntoElement)]
pub struct Switch {
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

impl Switch {
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

    /// Uncontrolled initial value: the switch keeps its own state.
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

    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);

        // Uncontrolled switches keep their value in window-scoped keyed state.
        let store = window.use_keyed_state(
            SharedString::from(format!("switch-state-{:?}", self.id)),
            cx,
            |_, _| self.default_checked,
        );
        let checked = if self.controlled {
            self.checked
        } else {
            *store.read(cx)
        };

        let spec = switch::resolve(
            &theme,
            &SwitchProps {
                size: self.size,
                checked,
                disabled: self.disabled,
            },
        );

        // Hover and press drive animated colours, same as the checkbox.
        let hovered_state = crate::interaction::flag(
            SharedString::from(format!("switch-hovered-{:?}", self.id)),
            window,
            cx,
        );
        let pressed_state = crate::interaction::flag(
            SharedString::from(format!("switch-pressed-{:?}", self.id)),
            window,
            cx,
        );
        let hovered = *hovered_state.read(cx) && !self.disabled;
        let pressed = *pressed_state.read(cx) && !self.disabled;
        let target_track_bg = match (pressed, hovered) {
            (true, _) => spec.pressed_track_bg,
            (_, true) => spec.hover_track_bg,
            _ => spec.track_bg,
        };
        let target_track_border = match (pressed, hovered) {
            (true, _) => spec.pressed_track_border,
            (_, true) => spec.hover_track_border,
            _ => spec.track_border,
        };
        let target_thumb = match (pressed, hovered) {
            (true, _) => spec.pressed_thumb,
            (_, true) => spec.hover_thumb,
            _ => spec.thumb,
        };
        let track_bg = crate::motion::color(
            SharedString::from(format!("switch-track-bg-{:?}", self.id)),
            target_track_bg,
            fluent_core::spec::motion::motion_policy(),
            window,
            cx,
        );
        let track_border = crate::motion::color(
            SharedString::from(format!("switch-track-border-{:?}", self.id)),
            target_track_border,
            fluent_core::spec::motion::motion_policy(),
            window,
            cx,
        );
        let thumb_color = crate::motion::color(
            SharedString::from(format!("switch-thumb-{:?}", self.id)),
            target_thumb,
            fluent_core::spec::motion::motion_policy(),
            window,
            cx,
        );
        // The thumb slides rather than jumping; Fluent runs this over 200 ms,
        // the same window as the track colours.
        let thumb_left = crate::motion::length(
            SharedString::from(format!("switch-thumb-x-{:?}", self.id)),
            if checked {
                spec.thumb_offset
            } else {
                spec.thumb_rest_offset
            },
            fluent_core::spec::motion::motion_policy(),
            window,
            cx,
        );

        let track = div()
            .flex_shrink_0()
            .w(spec.track_width)
            .h(spec.track_height)
            .rounded(spec.radius)
            .border(spec.border_width)
            .border_color(track_border)
            .bg(track_bg)
            .flex()
            .items_center()
            .pl(thumb_left)
            .child(
                div()
                    .flex_shrink_0()
                    .size(spec.thumb_size)
                    .rounded(spec.radius)
                    .bg(thumb_color),
            );

        let content = div()
            .flex()
            .items_center()
            .gap(spec.label_gap)
            .child(track)
            .when_some(self.label, |el, label| el.child(label));

        let controlled = self.controlled;
        let handler = self.on_change;
        let store_for_click = store.clone();

        let mut base = BaseSwitch::new(self.id.clone());
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
