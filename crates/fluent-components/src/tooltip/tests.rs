//! Behaviour of the root-hosted tooltip service.
//!
//! The overlay used to be published through an App global, so a second window's
//! root replaced the first one's entry and tooltips crossed windows. These tests
//! pin the per-window contract: each window gets its own overlay, and it survives
//! a redraw.

use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext,
    Window, div, px, size,
};

type Ids = std::rc::Rc<std::cell::RefCell<Vec<u64>>>;

struct RootHost {
    seen: Ids,
}

impl Render for RootHost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let seen = self.seen.clone();
        FluentRoot::new().child(
            div()
                .w(px(1.))
                .h(px(1.))
                .on_children_prepainted(move |_, window, cx| {
                    let overlay = crate::tooltip::overlay(window, cx);
                    seen.borrow_mut().push(
                        overlay
                            .map(|overlay| overlay.entity_id().as_u64())
                            .unwrap_or(0),
                    );
                })
                .child("probe"),
        )
    }
}

fn open(cx: &mut TestAppContext) -> (gpui::WindowHandle<RootHost>, Ids) {
    let seen: Ids = Default::default();
    let for_view = seen.clone();
    let handle = cx.open_window(size(px(200.), px(120.)), move |_, _| RootHost {
        seen: for_view,
    });
    cx.run_until_parked();
    (handle, seen)
}

fn draw(cx: &mut TestAppContext, handle: gpui::WindowHandle<RootHost>) {
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

#[gpui::test]
fn each_window_gets_its_own_overlay(cx: &mut TestAppContext) {
    let (first, first_ids) = open(cx);
    draw(cx, first);
    let (second, second_ids) = open(cx);
    draw(cx, second);
    draw(cx, first);

    let first_id = *first_ids.borrow().last().expect("the first window probed");
    let second_id = *second_ids
        .borrow()
        .last()
        .expect("the second window probed");
    assert_ne!(
        first_id, second_id,
        "two windows must not share one tooltip overlay"
    );
}

#[gpui::test]
fn the_overlay_survives_redraws(cx: &mut TestAppContext) {
    let (handle, ids) = open(cx);
    draw(cx, handle);
    let before = *ids.borrow().last().expect("probed");
    draw(cx, handle);
    let after = *ids.borrow().last().expect("probed again");
    assert_eq!(before, after, "a redraw must not replace the overlay");
}
