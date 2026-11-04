//! A button tooltip must disappear when the pointer leaves the button.
//!
//! The show/hide requests are driven from `on_hover` edges. Whether a request is
//! already outstanding has to survive a repaint: a render-local flag starts as
//! `false` again on the repaint that follows the show, so the leave edge would
//! never ask for a hide and the tooltip would stay on screen.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, Modifiers, MouseMoveEvent, PlatformInput, Point, Render, TestAppContext,
    Window, WindowHandle, point, size,
};

struct View;

impl Render for View {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(
            div().size_full().p(px(20.)).child(
                Button::new("tooltip-button")
                    .tooltip("Saves the current document")
                    .child("Hover for tooltip"),
            ),
        )
    }
}

fn draw(cx: &mut TestAppContext, handle: WindowHandle<View>) -> usize {
    let mut visible = 0;
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        visible = window
            .painted_quads()
            .into_iter()
            .filter(|quad| quad.background.as_solid().is_some_and(|c| c.a > 0.01))
            .count();
    })
    .unwrap();
    cx.run_until_parked();
    visible
}

fn settle(cx: &mut TestAppContext, handle: WindowHandle<View>) {
    for _ in 0..2 {
        draw(cx, handle);
    }
}

fn advance(cx: &mut TestAppContext, millis: u64) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(millis));
    cx.run_until_parked();
}

fn move_pointer(cx: &mut TestAppContext, handle: WindowHandle<View>, x: f32, y: f32) {
    let position: Point<gpui::Pixels> = point(px(x), px(y));
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn the_tooltip_goes_away_when_the_pointer_leaves(cx: &mut TestAppContext) {
    let handle = cx.open_window(size(px(500.), px(300.)), |_, _| View);
    cx.run_until_parked();
    settle(cx, handle);
    let resting = draw(cx, handle);

    move_pointer(cx, handle, 40., 36.);
    settle(cx, handle);
    // TooltipDefaults::show_delay is 500ms.
    advance(cx, 700);
    let hovering = draw(cx, handle);
    assert!(
        hovering > resting,
        "the tooltip never appeared: {hovering} painted quads while resting was {resting}"
    );

    move_pointer(cx, handle, 460., 280.);
    settle(cx, handle);
    // TooltipDefaults::grace_period is 300ms.
    advance(cx, 500);
    let after = draw(cx, handle);
    assert_eq!(
        after, resting,
        "the tooltip must be gone after the pointer leaves, still painting {after} quads"
    );
}
