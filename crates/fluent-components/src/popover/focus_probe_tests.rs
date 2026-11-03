//! Probe: does content inside a deferred popover panel join the tab order?
//!
//! Two focusable boxes are placed in a normal tree, then the same two boxes are
//! placed inside a popover panel. Tab must advance in the first case; the second
//! is what was unclear when the focus-trap tests could not distinguish a working
//! trap from a missing one.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, PlatformInput, Render,
    TestAppContext, Window, size,
};

#[derive(Clone)]
struct Pair {
    first: FocusHandle,
    second: FocusHandle,
}

/// Renders the pair directly, with no overlay involved.
struct PlainView(Pair);

impl Render for PlainView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .p(px(20.))
                .child(boxed("plain-first", &self.0.first, "first"))
                .child(boxed("plain-second", &self.0.second, "second")),
        )
    }
}

/// Renders the same pair inside an open popover panel.
struct InPanelView(Pair);

impl Render for InPanelView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let first = self.0.first.clone();
        let second = self.0.second.clone();
        FluentRoot::new().child(
            div().flex().flex_col().gap(px(8.)).p(px(20.)).child(
                Popover::new("probe-popover")
                    .default_open(true)
                    .trigger(div().id("probe-trigger").w(px(120.)).h(px(28.)).child("t"))
                    .content(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(boxed("panel-first", &first, "first"))
                            .child(boxed("panel-second", &second, "second")),
                    ),
            ),
        )
    }
}

fn boxed(id: &'static str, focus: &FocusHandle, label: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .track_focus(focus)
        .w(px(160.))
        .h(px(28.))
        .child(label)
}

fn bind_tab(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("tab", crate::FocusNext, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", crate::FocusPrev, Some(crate::ROOT_KEY_CONTEXT)),
        ]);
    });
}

fn press_tab(cx: &mut gpui::App, window: &mut Window) {
    window.dispatch_event(
        PlatformInput::KeyDown(KeyDownEvent {
            keystroke: Keystroke::parse("tab").unwrap(),
            is_held: false,
            prefer_character_input: false,
        }),
        cx,
    );
}

#[gpui::test]
fn tab_advances_between_boxes_in_a_plain_tree(cx: &mut TestAppContext) {
    bind_tab(cx);
    let handles: std::rc::Rc<std::cell::RefCell<Option<Pair>>> = Default::default();
    let slot = handles.clone();
    let handle = cx.open_window(size(px(500.), px(400.)), move |_, cx| {
        let pair = Pair {
            // A FocusHandle is not a tab stop by default, and navigation skips
            // non-stops, so a test that expects Tab to move has to say so.
            first: cx.focus_handle().tab_stop(true),
            second: cx.focus_handle().tab_stop(true),
        };
        slot.replace(Some(Pair {
            first: pair.first.clone(),
            second: pair.second.clone(),
        }));
        PlainView(pair)
    });
    cx.run_until_parked();
    let pair = handles.borrow().clone().expect("pair built");

    // Focus and draw in separate passes: tab stops are recorded while painting and
    // only become visible to focus_next on the frame after that.
    cx.update_window(handle.into(), |_, window, cx| {
        pair.first.focus(window, cx);
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();

    let mut after = None;
    cx.update_window(handle.into(), |_, window, cx| {
        press_tab(cx, window);
        after = window.focused(cx);
    })
    .unwrap();

    assert_eq!(
        after,
        Some(pair.second.clone()),
        "baseline: Tab must advance between two tracked boxes in a plain tree"
    );
}

#[gpui::test]
fn tab_advances_between_boxes_inside_a_popover_panel(cx: &mut TestAppContext) {
    bind_tab(cx);
    let handles: std::rc::Rc<std::cell::RefCell<Option<Pair>>> = Default::default();
    let slot = handles.clone();
    let handle = cx.open_window(size(px(500.), px(400.)), move |_, cx| {
        let pair = Pair {
            // A FocusHandle is not a tab stop by default, and navigation skips
            // non-stops, so a test that expects Tab to move has to say so.
            first: cx.focus_handle().tab_stop(true),
            second: cx.focus_handle().tab_stop(true),
        };
        slot.replace(Some(Pair {
            first: pair.first.clone(),
            second: pair.second.clone(),
        }));
        InPanelView(pair)
    });
    cx.run_until_parked();
    let pair = handles.borrow().clone().expect("pair built");

    let mut after = None;
    cx.update_window(handle.into(), |_, window, cx| {
        pair.first.focus(window, cx);
        // Tab stops are written into `next_frame` while painting and swapped into
        // `rendered_frame` at the start of the following frame, which is what
        // focus_next reads. One draw is therefore never enough.
        window.refresh();
        window.draw(cx).clear(cx);
        window.refresh();
        window.draw(cx).clear(cx);
        press_tab(cx, window);
        after = window.focused(cx);
    })
    .unwrap();

    assert_eq!(
        after,
        Some(pair.second.clone()),
        "content inside a deferred popover panel must still join the tab order"
    );
}
