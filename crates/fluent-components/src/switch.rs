//! Fluent 2 `Switch`.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, div, px,
};
use fluent_core::spec::ControlSize;
use fluent_core::spec::switch::{self, SwitchProps};
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui_base::Switch as BaseSwitch;

/// Change handler shape shared by the selection controls.
pub type ChangeHandler = Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

/// A Fluent 2 switch.
///
/// Controlled by default: pass [`Switch::checked`] and update it in
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
        let hovered_state = window.use_keyed_state(
            SharedString::from(format!("switch-hovered-{:?}", self.id)),
            cx,
            |_, _| false,
        );
        let pressed_state = window.use_keyed_state(
            SharedString::from(format!("switch-pressed-{:?}", self.id)),
            cx,
            |_, _| false,
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
        let track_bg = crate::motion::state_color(
            SharedString::from(format!("switch-track-bg-{:?}", self.id)),
            target_track_bg,
            window,
            cx,
        );
        let track_border = crate::motion::state_color(
            SharedString::from(format!("switch-track-border-{:?}", self.id)),
            target_track_border,
            window,
            cx,
        );
        let thumb_color = crate::motion::state_color(
            SharedString::from(format!("switch-thumb-{:?}", self.id)),
            target_thumb,
            window,
            cx,
        );
        let track = div()
            .flex_shrink_0()
            .w(spec.track_width)
            .h(spec.track_height)
            .rounded(px(10000.))
            .border_1()
            .border_color(track_border)
            .bg(track_bg)
            .flex()
            .items_center()
            .pl(if checked { spec.thumb_offset } else { px(3.) })
            .child(
                div()
                    .flex_shrink_0()
                    .size(spec.thumb_size)
                    .rounded(px(10000.))
                    .bg(thumb_color),
            );

        let content = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(track)
            .when_some(self.label, |el, label| el.child(label));

        let controlled = self.controlled;
        let handler = self.on_change;
        let store_for_click = store.clone();

        let hover_listener = hovered_state.clone();
        let press_down = pressed_state.clone();
        let press_up = pressed_state.clone();
        let mut base = BaseSwitch::new(self.id.clone());
        base.interactivity().on_hover(move |is_hovered, _, cx| {
            hover_listener.update(cx, |state, cx| {
                if *state != *is_hovered {
                    *state = *is_hovered;
                    cx.notify();
                }
            });
        });
        base.interactivity()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                press_down.update(cx, |state, cx| {
                    *state = true;
                    cx.notify();
                });
            });
        base.interactivity()
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                press_up.update(cx, |state, cx| {
                    *state = false;
                    cx.notify();
                });
            });
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
