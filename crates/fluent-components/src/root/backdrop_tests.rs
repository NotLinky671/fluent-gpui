//! What asking for a window material does to the window.
//!
//! The test platform has no compositor: its appearance getter always reports
//! `Opaque` and its setter does nothing. Asserting on the policy or the painted
//! colour alone would therefore miss the case that matters — a request that
//! leaves the window composing against the desktop while the page paints an
//! opaque surface. These tests install a recorded platform, drive the production
//! path (`policy_for` → `apply` → the appearance setter) and check the calls a
//! real window would have received.

use super::*;
use crate::backdrop::{
    BackdropOutcome, MATERIAL_LAYER_ALPHA, RecordedCalls, WindowBackdrop, fake_calls, install_fake,
    last_request,
};
use fluent_theme::{Appearance, FluentTheme};
use gpui::{
    AppContext, Context, Render, TestAppContext, Window, WindowBackgroundAppearance, WindowHandle,
    px, size,
};
use std::cell::Cell;
use std::rc::Rc;

type Asked = Rc<Cell<Option<WindowBackdrop>>>;

struct View {
    asked: Asked,
}

impl Render for View {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = FluentRoot::new();
        if let Some(backdrop) = self.asked.get() {
            root = root.backdrop(backdrop);
        }
        root.child(div().size_full())
    }
}

struct Setup {
    handle: WindowHandle<View>,
    asked: Asked,
}

fn open(cx: &mut TestAppContext, backdrop: Option<WindowBackdrop>) -> Setup {
    open_with(cx, backdrop, None)
}

fn open_with(
    cx: &mut TestAppContext,
    backdrop: Option<WindowBackdrop>,
    appearance: Option<Appearance>,
) -> Setup {
    cx.update(|cx| {
        fluent_core::gpui_compat::init_base(cx);
        if let Some(appearance) = appearance {
            FluentTheme::for_appearance(appearance).install(cx);
        }
    });
    let asked: Asked = Rc::new(Cell::new(backdrop));
    let for_view = asked.clone();
    let handle = cx.open_window(size(px(600.), px(400.)), move |_, _| View {
        asked: for_view,
    });
    cx.run_until_parked();
    Setup { handle, asked }
}

/// Asks for another backdrop and lets one frame run.
fn ask(cx: &mut TestAppContext, setup: &Setup, backdrop: Option<WindowBackdrop>) {
    setup.asked.set(backdrop);
    let _ = surface(cx, setup.handle);
}

/// The colour of the largest painted quad, which is the root's own surface.
fn surface(cx: &mut TestAppContext, handle: WindowHandle<View>) -> gpui::Hsla {
    let mut colour = None;
    for _ in 0..2 {
        colour = Some(paint_frame(cx, handle));
        cx.update_window(handle.into(), |_, window, cx| {
            window.simulate_next_frame(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    colour.expect("the root must paint a surface")
}

/// Draw exactly one frame, without delivering its next-frame callbacks.
fn paint_frame(cx: &mut TestAppContext, handle: WindowHandle<View>) -> gpui::Hsla {
    let mut colour = None;
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
    colour.expect("the root must paint a surface")
}

fn calls() -> RecordedCalls {
    fake_calls().expect("this test installed a recorded platform")
}

/// What the window was last asked for. Read after a frame, because the request
/// is made while the root renders.
fn last_of(cx: &mut TestAppContext, setup: &Setup) -> Option<(WindowBackdrop, BackdropOutcome)> {
    let mut last = None;
    cx.update_window(setup.handle.into(), |_, window, cx| {
        last = last_request(window, cx);
    })
    .unwrap();
    last
}

#[test]
fn the_default_root_paints_an_opaque_surface() {
    let mut cx = TestAppContext::single();
    let expected = cx.update(|cx| FluentTheme::global(cx).tokens().color.neutral_background_1);
    let setup = open(&mut cx, None);
    let colour = surface(&mut cx, setup.handle);
    assert_eq!(
        colour, expected,
        "a root that asks for nothing must paint the theme surface unchanged"
    );
    assert_eq!(colour.a, 1.0, "and it must be opaque");
    assert!(
        fake_calls().is_none(),
        "a root with no request must not talk to the platform at all"
    );
}

#[test]
fn a_refused_material_leaves_the_window_opaque() {
    let mut cx = TestAppContext::single();
    install_fake(vec![(
        WindowBackdrop::Mica,
        BackdropOutcome::Unsupported("no compositor"),
    )]);
    let setup = open(&mut cx, Some(WindowBackdrop::Mica));
    let colour = surface(&mut cx, setup.handle);
    let recorded = calls();

    assert_eq!(
        recorded.requests,
        vec![(WindowBackdrop::Mica, false)],
        "the request must still be made"
    );
    assert_eq!(
        recorded.appearances,
        vec![WindowBackgroundAppearance::Opaque],
        "a refused request must never make the window transparent, not even for one setter call"
    );
    assert_eq!(
        recorded.clears, 1,
        "the refused material must be cleared explicitly"
    );
    assert_eq!(colour.a, 1.0, "and the page must stay opaque");
    assert_eq!(
        last_of(&mut cx, &setup),
        Some((
            WindowBackdrop::Mica,
            BackdropOutcome::Unsupported("no compositor")
        )),
        "the window must keep the refusal, not the fallback's success"
    );
}

#[test]
fn a_failed_request_keeps_its_reason() {
    let mut cx = TestAppContext::single();
    install_fake(vec![(
        WindowBackdrop::Mica,
        BackdropOutcome::Failed(-2147467259),
    )]);
    let setup = open(&mut cx, Some(WindowBackdrop::Mica));
    let colour = surface(&mut cx, setup.handle);
    let recorded = calls();

    assert_eq!(
        recorded.appearances,
        vec![WindowBackgroundAppearance::Opaque]
    );
    assert_eq!(recorded.clears, 1);
    assert_eq!(colour.a, 1.0);
    assert_eq!(
        last_of(&mut cx, &setup),
        Some((WindowBackdrop::Mica, BackdropOutcome::Failed(-2147467259))),
        "clearing the material must not overwrite the failure that caused it"
    );
}

#[test]
fn a_mismatched_material_is_treated_as_a_refusal() {
    let mut cx = TestAppContext::single();
    install_fake(vec![(
        WindowBackdrop::Mica,
        BackdropOutcome::Mismatch {
            asked: 2,
            reported: 1,
        },
    )]);
    let setup = open(&mut cx, Some(WindowBackdrop::Mica));
    let colour = surface(&mut cx, setup.handle);
    let recorded = calls();

    assert_eq!(
        recorded.appearances,
        vec![WindowBackgroundAppearance::Opaque]
    );
    assert_eq!(recorded.clears, 1);
    assert_eq!(colour.a, 1.0);
    assert_eq!(
        last_of(&mut cx, &setup),
        Some((
            WindowBackdrop::Mica,
            BackdropOutcome::Mismatch {
                asked: 2,
                reported: 1
            }
        ))
    );
}

#[test]
fn a_confirmed_material_turns_the_window_transparent() {
    let mut cx = TestAppContext::single();
    install_fake(vec![(WindowBackdrop::Mica, BackdropOutcome::Applied)]);
    let setup = open(&mut cx, Some(WindowBackdrop::Mica));
    let colour = surface(&mut cx, setup.handle);
    let recorded = calls();

    assert_eq!(
        recorded.appearances,
        vec![WindowBackgroundAppearance::MicaBackdrop]
    );
    assert_eq!(
        recorded.clears, 0,
        "a confirmed material must not be cleared again"
    );
    assert!(
        (colour.a - MATERIAL_LAYER_ALPHA).abs() < 0.01,
        "a confirmed material paints a translucent layer, got alpha {}",
        colour.a
    );
    assert_eq!(
        last_of(&mut cx, &setup),
        Some((WindowBackdrop::Mica, BackdropOutcome::Applied))
    );
}

#[test]
fn high_contrast_asks_for_solid_and_stays_opaque() {
    let mut cx = TestAppContext::single();
    install_fake(Vec::new());
    let setup = open_with(
        &mut cx,
        Some(WindowBackdrop::Mica),
        Some(Appearance::HighContrast),
    );
    let colour = surface(&mut cx, setup.handle);
    let recorded = calls();

    assert_eq!(
        recorded.requests,
        vec![(WindowBackdrop::Solid, false)],
        "high contrast downgrades the window request itself, otherwise turning it off could not bring the material back"
    );
    assert_eq!(
        recorded.appearances,
        vec![WindowBackgroundAppearance::Opaque]
    );
    assert_eq!(recorded.clears, 0, "the request already asked for none");
    assert_eq!(colour.a, 1.0);
    assert_eq!(
        last_of(&mut cx, &setup),
        Some((WindowBackdrop::Solid, BackdropOutcome::Applied))
    );
}

#[test]
fn a_repaint_does_not_ask_the_compositor_again() {
    let mut cx = TestAppContext::single();
    install_fake(vec![(WindowBackdrop::Mica, BackdropOutcome::Applied)]);
    let setup = open(&mut cx, Some(WindowBackdrop::Mica));
    for _ in 0..4 {
        let _ = surface(&mut cx, setup.handle);
    }
    let recorded = calls();

    assert_eq!(
        recorded.requests.len(),
        1,
        "each further frame must not re-issue the platform call: {:?}",
        recorded.requests
    );
    assert_eq!(recorded.appearances.len(), 1);
    assert_eq!(recorded.clears, 0);
}

#[test]
fn solid_removes_the_material_and_mica_can_come_back() {
    let mut cx = TestAppContext::single();
    install_fake(vec![(WindowBackdrop::Mica, BackdropOutcome::Applied)]);
    let setup = open(&mut cx, Some(WindowBackdrop::Mica));
    let _ = surface(&mut cx, setup.handle);

    ask(&mut cx, &setup, Some(WindowBackdrop::Solid));
    let solid = surface(&mut cx, setup.handle);
    assert_eq!(
        solid.a, 1.0,
        "asking for Solid must remove the material, not merely stop painting it"
    );

    ask(&mut cx, &setup, Some(WindowBackdrop::Mica));
    let back = surface(&mut cx, setup.handle);
    assert!((back.a - MATERIAL_LAYER_ALPHA).abs() < 0.01);

    let recorded = calls();
    assert_eq!(
        recorded.requests,
        vec![
            (WindowBackdrop::Mica, false),
            (WindowBackdrop::Solid, false),
            (WindowBackdrop::Mica, false),
        ],
        "each change of request is one platform call, in order"
    );
    assert_eq!(
        recorded.appearances,
        vec![
            WindowBackgroundAppearance::MicaBackdrop,
            WindowBackgroundAppearance::Opaque,
            WindowBackgroundAppearance::MicaBackdrop,
        ]
    );
    assert_eq!(recorded.clears, 0, "the Solid request clears by itself");
}

#[test]
fn solid_paints_an_opaque_frame_before_removing_either_material() {
    for material in [WindowBackdrop::Mica, WindowBackdrop::MicaAlt] {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let mut cx = TestAppContext::single();
            install_fake(Vec::new());
            let setup = open_with(&mut cx, Some(material), Some(appearance));
            let _ = surface(&mut cx, setup.handle);
            let before = calls();

            setup.asked.set(Some(WindowBackdrop::Solid));
            assert_eq!(paint_frame(&mut cx, setup.handle).a, 1.0);
            assert_eq!(
                calls().requests,
                before.requests,
                "removing the material before this opaque frame is presented exposes the old translucent frame"
            );
            assert_eq!(
                calls().appearances,
                before.appearances,
                "the platform appearance must also wait until after the opaque frame"
            );
            assert_eq!(
                last_of(&mut cx, &setup),
                Some((material, BackdropOutcome::Applied)),
                "diagnostics must report the completed request while Solid is pending"
            );

            let _ = surface(&mut cx, setup.handle);
            let after = calls();
            assert_eq!(after.requests.len(), 2);
            assert_eq!(
                after.requests[1],
                (WindowBackdrop::Solid, appearance == Appearance::Dark)
            );
            assert_eq!(
                after.appearances.last(),
                Some(&WindowBackgroundAppearance::Opaque)
            );
            assert_eq!(
                last_of(&mut cx, &setup),
                Some((WindowBackdrop::Solid, BackdropOutcome::Applied))
            );
            let _ = surface(&mut cx, setup.handle);
            assert_eq!(
                calls().requests.len(),
                2,
                "ordinary repaint must not repeat the removal"
            );
        }
    }
}

#[test]
fn returning_to_the_material_cancels_a_pending_solid_switch() {
    for back in [WindowBackdrop::MicaAlt, WindowBackdrop::Mica] {
        let mut cx = TestAppContext::single();
        install_fake(Vec::new());
        let setup = open(&mut cx, Some(WindowBackdrop::MicaAlt));
        let _ = surface(&mut cx, setup.handle);

        setup.asked.set(Some(WindowBackdrop::Solid));
        assert_eq!(paint_frame(&mut cx, setup.handle).a, 1.0);
        setup.asked.set(Some(back));
        assert!((paint_frame(&mut cx, setup.handle).a - MATERIAL_LAYER_ALPHA).abs() < 0.01);
        let _ = surface(&mut cx, setup.handle);
        let mut expected = vec![(WindowBackdrop::MicaAlt, false)];
        if back != WindowBackdrop::MicaAlt {
            expected.push((back, false));
        }
        assert_eq!(
            calls().requests,
            expected,
            "a stale callback must not clear the material after the user switched back"
        );
        assert_eq!(
            calls().appearances.last(),
            Some(&crate::backdrop::window_appearance(back))
        );

        ask(&mut cx, &setup, Some(WindowBackdrop::Solid));
        assert_eq!(
            calls().requests.last(),
            Some(&(WindowBackdrop::Solid, false)),
            "cancelling one switch must not prevent a later Solid request"
        );
    }
}

#[test]
fn high_contrast_paints_opaque_before_revoking_a_material() {
    let mut cx = TestAppContext::single();
    install_fake(Vec::new());
    let setup = open(&mut cx, Some(WindowBackdrop::MicaAlt));
    let _ = surface(&mut cx, setup.handle);
    cx.update(|cx| FluentTheme::for_appearance(Appearance::HighContrast).install(cx));
    assert_eq!(paint_frame(&mut cx, setup.handle).a, 1.0);
    assert_eq!(calls().requests, vec![(WindowBackdrop::MicaAlt, false)]);
    let _ = surface(&mut cx, setup.handle);
    assert_eq!(
        calls().requests.last(),
        Some(&(WindowBackdrop::Solid, false))
    );
    assert_eq!(
        calls().appearances.last(),
        Some(&WindowBackgroundAppearance::Opaque)
    );
}

#[test]
fn a_deferred_solid_request_keeps_its_actual_failure() {
    let mut cx = TestAppContext::single();
    install_fake(vec![(
        WindowBackdrop::Solid,
        BackdropOutcome::Failed(-2147467259),
    )]);
    let setup = open(&mut cx, Some(WindowBackdrop::MicaAlt));
    let _ = surface(&mut cx, setup.handle);
    ask(&mut cx, &setup, Some(WindowBackdrop::Solid));
    assert_eq!(
        last_of(&mut cx, &setup),
        Some((WindowBackdrop::Solid, BackdropOutcome::Failed(-2147467259)))
    );
    assert_eq!(calls().clears, 1);
    assert_eq!(surface(&mut cx, setup.handle).a, 1.0);
}

#[test]
fn the_dark_flag_is_part_of_the_request() {
    let mut cx = TestAppContext::single();
    install_fake(vec![(WindowBackdrop::Mica, BackdropOutcome::Applied)]);
    let setup = open_with(&mut cx, Some(WindowBackdrop::Mica), Some(Appearance::Dark));
    let dark_alpha = surface(&mut cx, setup.handle).a;

    cx.update(|cx| FluentTheme::for_appearance(Appearance::Light).install(cx));
    let light_alpha = surface(&mut cx, setup.handle).a;

    let recorded = calls();
    assert_eq!(
        recorded.requests,
        vec![(WindowBackdrop::Mica, true), (WindowBackdrop::Mica, false)],
        "the app mode must be part of the cached request, or a theme switch would not re-tint the material"
    );
    assert!((dark_alpha - MATERIAL_LAYER_ALPHA).abs() < 0.01);
    assert!((light_alpha - MATERIAL_LAYER_ALPHA).abs() < 0.01);
}
