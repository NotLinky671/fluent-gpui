//! Shared pointer state wiring. Bounds-qualified hover is kept in Button;
//! release-outside cancellation is intentionally left to the existing behavior.

use gpui::{App, ElementId, Entity, Interactivity, MouseButton, Window};

pub(crate) fn flag(key: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> Entity<bool> {
    window.use_keyed_state(key, cx, |_, _| false)
}

pub(crate) fn set(flag: &Entity<bool>, value: bool, cx: &mut App) {
    flag.update(cx, |state, cx| {
        if *state != value {
            *state = value;
            cx.notify();
        }
    });
}

pub(crate) fn hover(interactivity: &mut Interactivity, state: Entity<bool>) {
    interactivity.on_hover(move |value, _, cx| set(&state, *value, cx));
}

pub(crate) fn press(interactivity: &mut Interactivity, state: Entity<bool>) {
    let down = state.clone();
    interactivity.on_mouse_down(MouseButton::Left, move |_, _, cx| set(&down, true, cx));
    interactivity.on_mouse_up(MouseButton::Left, move |_, _, cx| set(&state, false, cx));
}
