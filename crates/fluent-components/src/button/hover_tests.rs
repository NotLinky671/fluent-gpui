//! Hover and press colour fades must never pass through a colour that is darker
//! than both ends.
//!
//! Fluent's `subtle` and `transparent` button appearances rest on
//! `transparent_black()`. Mixing raw HSLA channels from that to an opaque fill
//! walks through dark translucent greys — the surface flashes darker than either
//! endpoint for a frame or two before settling. These tests watch the *painted*
//! quads frame by frame while the fade runs.

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
                Button::new("fade-probe")
                    .appearance(ButtonAppearance::Subtle)
                    .child("Cancel"),
            ),
        )
    }
}

/// One frame's visible quads, keyed by where they were painted.
fn painted_quads(
    cx: &mut TestAppContext,
    handle: WindowHandle<View>,
) -> Vec<((f32, f32), gpui::Hsla)> {
    let mut out = Vec::new();
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        out = window
            .painted_quads()
            .into_iter()
            .filter_map(|quad| {
                let color = quad.background.as_solid()?;
                let origin = (quad.bounds.origin.x.0, quad.bounds.origin.y.0);
                Some((origin, color))
            })
            .collect();
    })
    .unwrap();
    out
}

fn settle(cx: &mut TestAppContext, handle: WindowHandle<View>) {
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

fn hover(cx: &mut TestAppContext, handle: WindowHandle<View>) {
    let position: Point<gpui::Pixels> = point(px(40.), px(36.));
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

/// Runs the fade and returns the darkest lightness painted while it settles.
fn darkest_during_fade(
    cx: &mut TestAppContext,
    handle: WindowHandle<View>,
    frames: usize,
    step: std::time::Duration,
) -> f32 {
    // The page background does not change, so only quads whose colour differs from
    // the resting frame are the control being faded. That keeps a dark page from
    // being mistaken for a dark fade.
    let rest = painted_quads(cx, handle);
    let mut darkest = f32::MAX;
    for _ in 0..frames {
        cx.executor().advance_clock(step);
        cx.run_until_parked();
        for (origin, color) in painted_quads(cx, handle) {
            let unchanged = rest
                .iter()
                .any(|(rest_origin, rest_color)| *rest_origin == origin && *rest_color == color);
            if !unchanged && color.a > 0.2 && color.l < darkest {
                darkest = color.l;
            }
        }
    }
    darkest
}

fn check_theme(cx: &mut TestAppContext, theme: fluent_theme::FluentTheme, floor: f32) {
    cx.update(|cx| *fluent_theme::FluentTheme::global_mut(cx) = theme);
    let handle = cx.open_window(size(px(400.), px(200.)), |_, _| View);
    cx.run_until_parked();
    settle(cx, handle);

    hover(cx, handle);
    let darkest = darkest_during_fade(cx, handle, 14, std::time::Duration::from_millis(16));
    assert!(
        darkest >= floor,
        "the hover fade painted lightness {darkest}, darker than both endpoints; \
         a transparent rest state must fade in alpha only"
    );
}

#[gpui::test]
fn hovering_a_subtle_button_never_darkens_past_both_ends(cx: &mut TestAppContext) {
    // Light: rest transparent, hover #f5f5f5 (l = 0.9607…).
    check_theme(cx, fluent_theme::FluentTheme::web_light(), 0.9);
}

#[gpui::test]
fn hovering_a_subtle_button_in_dark_never_darkens_past_both_ends(cx: &mut TestAppContext) {
    // Dark: rest transparent, hover #383838 (l ≈ 0.2196).
    check_theme(cx, fluent_theme::FluentTheme::web_dark(), 0.2);
}
