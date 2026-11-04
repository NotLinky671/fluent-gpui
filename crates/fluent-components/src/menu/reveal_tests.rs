//! The menu surface fades in when a dropdown opens it.
//!
//! Measured from a recording of the Fluent docs menu: the panel appears at its
//! final size and position, and only its opacity ramps (width, left/right edges
//! and the top edge stay constant across the transition). It is a fade, not a
//! grow — this test pins both halves of that: constant size, rising opacity.

use super::*;
use crate::root::FluentRoot;
use gpui::{AppContext, Context, Render, TestAppContext, Window, WindowHandle, px, size};

struct View;

impl Render for View {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = Menu::new("reveal-panel")
            .item(MenuItem::new("one", "One"))
            .item(MenuItem::new("two", "Two"))
            .item(MenuItem::new("three", "Three"));

        FluentRoot::new().child(
            div().size_full().p(px(40.)).child(
                MenuDropdown::new(
                    "reveal-trigger",
                    div().id("reveal-btn").w(px(160.)).h(px(32.)).child("Open"),
                    menu,
                )
                .default_open(true)
                .track_focus(&cx.focus_handle()),
            ),
        )
    }
}

/// The widest opaque-ish block that is not the page background: its height and
/// current alpha. `None` when the menu has not painted anything yet.
fn surface(cx: &mut TestAppContext, handle: WindowHandle<View>) -> Option<(f32, f32, f32)> {
    let mut found: Option<(f32, f32, f32)> = None;
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        let scale = window.scale_factor();
        found = window
            .painted_quads()
            .into_iter()
            .filter(|quad| quad.bounds.size.width.0 / scale < 400.)
            .filter_map(|quad| quad.background.as_solid().map(|c| (quad, c.a)))
            .max_by(|a, b| {
                a.0.bounds
                    .size
                    .height
                    .0
                    .total_cmp(&b.0.bounds.size.height.0)
            })
            .map(|(quad, alpha)| {
                (
                    quad.bounds.size.height.0 / scale,
                    alpha,
                    quad.bounds.origin.y.0 / scale,
                )
            });
    })
    .unwrap();
    cx.run_until_parked();
    found
}

fn advance(cx: &mut TestAppContext, millis: u64) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(millis));
    cx.run_until_parked();
}

#[gpui::test]
fn the_menu_fades_in_at_its_final_size(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    let handle = cx.open_window(size(px(500.), px(500.)), |_, _| View);
    cx.run_until_parked();

    // The menu is laid out at its final size immediately; only alpha is zero.
    let (first_height, first_alpha, first_top) =
        surface(cx, handle).expect("the menu must be laid out");
    assert!(
        first_alpha < 0.01,
        "the menu starts transparent, got alpha {first_alpha}"
    );

    // 120 ms into a 350 ms ramp, the quartic ease-out is at ~0.81.
    advance(cx, 120);
    let (during_height, during_alpha, during_top) =
        surface(cx, handle).expect("the menu must be partially visible mid-fade");
    assert!(
        during_alpha > 0.5 && during_alpha < 0.99,
        "the fade must be partway through, got alpha {during_alpha}"
    );

    advance(cx, 400);
    let (settled_height, settled_alpha, settled_top) =
        surface(cx, handle).expect("the menu must be visible");
    assert_eq!(settled_alpha, 1.0, "the fade must finish fully opaque");
    // The surface starts a few pixels higher and settles down, matching the
    // measured motion; a pure fade would keep the top edge fixed.
    assert!(
        first_top < settled_top,
        "the menu must start higher and settle down: first {first_top}, settled {settled_top}"
    );
    assert!(
        during_top > first_top && during_top <= settled_top,
        "the offset must close as the fade runs: first {first_top}, during {during_top}, settled {settled_top}"
    );
    // Both must run on the same curve so they finish together: whatever fraction of
    // the offset is left has to match the fraction of opacity still missing. The
    // tolerance covers pixel snapping of the painted origin.
    let offset_left = (settled_top - during_top) / 4.0;
    let opacity_left = 1.0 - during_alpha;
    assert!(
        (offset_left - opacity_left).abs() < 0.1,
        "movement and fade must share a curve: {offset_left} of the offset left vs {opacity_left} of the fade left"
    );
    assert_eq!(
        first_height, settled_height,
        "the surface must be at its final size while fading: {first_height} then {settled_height}"
    );
    assert_eq!(
        during_height, settled_height,
        "the surface must not grow during the fade: {during_height} then {settled_height}"
    );
}
