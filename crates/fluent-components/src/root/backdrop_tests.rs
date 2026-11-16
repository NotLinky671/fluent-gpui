//! What the root paints when it asks for a window material.
//!
//! The compositor is the only authority: GPUI's setter returns nothing, so a
//! refused request is indistinguishable from one that never happened unless the
//! root consults the answer. These tests pin both directions — a material the
//! platform confirmed must let the window show through, and one it refused,
//! could not confirm, or that high contrast overrules must leave the page
//! opaque.

use super::*;
use crate::backdrop::{BackdropOutcome, MATERIAL_LAYER_ALPHA, WindowBackdrop, prime};
use fluent_theme::FluentTheme;
use gpui::{AppContext, Context, Render, TestAppContext, Window, WindowHandle, px, size};

struct View {
    backdrop: Option<WindowBackdrop>,
}

impl Render for View {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = FluentRoot::new();
        if let Some(backdrop) = self.backdrop {
            root = root.backdrop(backdrop);
        }
        root.child(div().size_full())
    }
}

fn open(cx: &mut TestAppContext, backdrop: Option<WindowBackdrop>) -> WindowHandle<View> {
    cx.update(fluent_core::gpui_compat::init_base);
    let handle = cx.open_window(size(px(600.), px(400.)), |_, _| View { backdrop });
    cx.run_until_parked();
    handle
}

fn open_high_contrast(
    cx: &mut TestAppContext,
    backdrop: Option<WindowBackdrop>,
) -> WindowHandle<View> {
    cx.update(|cx| {
        fluent_core::gpui_compat::init_base(cx);
        FluentTheme::high_contrast().install(cx);
    });
    let handle = cx.open_window(size(px(600.), px(400.)), |_, _| View { backdrop });
    cx.run_until_parked();
    handle
}

/// The colour of the largest painted quad, which is the root's own surface.
fn surface(cx: &mut TestAppContext, handle: WindowHandle<View>) -> gpui::Hsla {
    let mut colour = None;
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            let biggest = window
                .painted_quads()
                .into_iter()
                .filter(|quad| quad.background.as_solid().is_some())
                .max_by(|a, b| {
                    let area_a = a.bounds.size.width.0 * a.bounds.size.height.0;
                    let area_b = b.bounds.size.width.0 * b.bounds.size.height.0;
                    area_a.total_cmp(&area_b)
                });
            colour = biggest.and_then(|quad| quad.background.as_solid());
        })
        .unwrap();
        cx.run_until_parked();
    }
    colour.expect("the root must paint a surface")
}

fn confirmed(cx: &mut TestAppContext, backdrop: WindowBackdrop) {
    cx.update(|cx| prime(cx, backdrop, BackdropOutcome::Applied));
}

#[test]
fn the_default_root_paints_an_opaque_surface() {
    let mut cx = TestAppContext::single();
    let expected = cx.update(|cx| FluentTheme::global(cx).tokens().color.neutral_background_1);
    let handle = open(&mut cx, None);
    let colour = surface(&mut cx, handle);
    assert_eq!(
        colour, expected,
        "a root that asks for nothing must paint the theme surface unchanged"
    );
    assert_eq!(colour.a, 1.0, "and it must be opaque");
}

#[test]
fn a_material_the_compositor_refuses_stays_opaque() {
    let mut cx = TestAppContext::single();
    let handle = open(&mut cx, Some(WindowBackdrop::Mica));
    let colour = surface(&mut cx, handle);
    assert_eq!(
        colour.a, 1.0,
        "the test platform has no compositor, so Mica cannot be confirmed and the page must stay opaque"
    );
}

#[test]
fn a_confirmed_material_lets_the_window_through() {
    let mut cx = TestAppContext::single();
    let handle = open(&mut cx, Some(WindowBackdrop::Mica));
    confirmed(&mut cx, WindowBackdrop::Mica);
    let colour = surface(&mut cx, handle);
    assert!(
        (colour.a - MATERIAL_LAYER_ALPHA).abs() < 0.01,
        "a confirmed material paints a translucent layer, got alpha {}",
        colour.a
    );
}

#[test]
fn high_contrast_refuses_the_material_even_when_it_was_confirmed() {
    let mut cx = TestAppContext::single();
    let handle = open_high_contrast(&mut cx, Some(WindowBackdrop::Mica));
    confirmed(&mut cx, WindowBackdrop::Mica);
    let colour = surface(&mut cx, handle);
    assert_eq!(
        colour.a, 1.0,
        "high contrast overrules the material, so the page must stay opaque"
    );
}
