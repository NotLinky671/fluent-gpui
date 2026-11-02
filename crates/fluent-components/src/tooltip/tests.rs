//! Behaviour of the root-hosted tooltip service.
//!
//! These tests cover the part that is testable without a GPU surface: whether the
//! service is published by the root, whether it survives redraws, and whether
//! callers degrade safely when no root is present. Placement and hover timing are
//! not covered here — see the subsystem notes for what still needs a real window.

use crate::root::FluentRoot;
use gpui::{
    Context, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext, Window, div, px,
    size,
};

struct RootHost {
    seen: std::rc::Rc<std::cell::RefCell<Vec<bool>>>,
}

impl Render for RootHost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child({
            // Reads the service after the root published it.
            let seen = self.seen.clone();
            div()
                .w(px(1.))
                .h(px(1.))
                .on_children_prepainted(move |_, _, cx| {
                    seen.borrow_mut()
                        .push(crate::tooltip::overlay(cx).is_some())
                })
                .child("probe")
        })
    }
}

#[test]
fn no_root_means_no_overlay() {
    // Button::tooltip queries the service on every hover, so absence has to be a
    // plain `None` rather than a panic.
    let app = TestAppContext::single();
    app.update(|cx| {
        assert!(
            crate::tooltip::overlay(cx).is_none(),
            "a process without FluentRoot must not advertise an overlay"
        );
    });
}

#[gpui::test]
fn root_publishes_the_overlay(cx: &mut TestAppContext) {
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let _handle = cx.open_window(size(px(400.), px(300.)), |_, _| RootHost {
        seen: seen.clone(),
    });
    cx.run_until_parked();

    let observed = seen.borrow().clone();
    assert!(
        !observed.is_empty(),
        "the view must have rendered at least once"
    );
    // The first render happens before the root publishes, later ones after.
    assert!(
        observed.last().copied().unwrap_or(false),
        "after rendering, the overlay must be published: {observed:?}"
    );
}

#[gpui::test]
fn overlay_identity_is_stable_across_redraws(cx: &mut TestAppContext) {
    // Requesting a tooltip captures the overlay entity. If a redraw produced a
    // new entity, an in-flight request would target a surface that is no longer
    // mounted.
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let _handle = cx.open_window(size(px(400.), px(300.)), |_, _| RootHost {
        seen: seen.clone(),
    });
    cx.run_until_parked();

    let mut ids = Vec::new();
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|cx| {
            if let Some(overlay) = crate::tooltip::overlay(cx) {
                ids.push(overlay.entity_id());
            }
        });
    }

    assert_eq!(ids.len(), 3, "the overlay must be reachable on every frame");
    assert!(
        ids.windows(2).all(|pair| pair[0] == pair[1]),
        "the overlay entity must not change identity between frames: {ids:?}"
    );
}
