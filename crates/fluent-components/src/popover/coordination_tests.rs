//! Overlay coordination: Escape from each focus position, controlled callers
//! accepting a request, uncontrolled state across redraws, and per-id isolation.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, PlatformInput, Render,
    TestAppContext, Window, WindowHandle, div, px, size,
};

type Log = std::rc::Rc<std::cell::RefCell<Vec<(&'static str, bool)>>>;

/// A screen with one controlled or uncontrolled popover, plus a focusable box
/// outside it.
struct CoordView {
    popover_focus: FocusHandle,
    outside_focus: FocusHandle,
    log: Log,
    controlled: bool,
    open: bool,
}

impl Render for CoordView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let log = self.log.clone();
        let mut popover = Popover::new("coord-popover")
            .trigger(
                div()
                    .id("coord-trigger")
                    .w(px(140.))
                    .h(px(28.))
                    .child("open"),
            )
            .content(div().id("coord-panel").w(px(120.)).h(px(60.)).child("body"))
            .track_focus(&self.popover_focus)
            .on_open_change(move |next, _, _| log.borrow_mut().push(("popover", next)));
        popover = if self.controlled {
            popover.open(self.open)
        } else {
            popover.default_open(true)
        };

        let outside = div()
            .id("coord-outside")
            .track_focus(&self.outside_focus)
            .w(px(80.))
            .h(px(24.))
            .child("outside");

        FluentRoot::new().child(
            div()
                .size_full()
                .p(px(24.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(popover)
                .child(outside),
        )
    }
}

fn bind_escape(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([KeyBinding::new(
            "escape",
            crate::Dismiss,
            Some(crate::OVERLAY_KEY_CONTEXT),
        )]);
    });
}

fn open_view(cx: &mut TestAppContext, controlled: bool) -> (WindowHandle<CoordView>, Log) {
    bind_escape(cx);
    let log: Log = Default::default();
    let for_view = log.clone();
    let handle = cx.open_window(size(px(500.), px(400.)), move |_, cx| CoordView {
        popover_focus: cx.focus_handle().tab_stop(true),
        outside_focus: cx.focus_handle().tab_stop(true),
        log: for_view,
        controlled,
        open: true,
    });
    cx.run_until_parked();
    settle(cx, handle);
    (handle, log)
}

fn settle(cx: &mut TestAppContext, handle: WindowHandle<CoordView>) {
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

fn focus(cx: &mut TestAppContext, handle: WindowHandle<CoordView>, inside: bool) {
    cx.update_window(handle.into(), move |view, window, cx| {
        let view = view.downcast::<CoordView>().unwrap();
        let target = view.read(cx);
        let focus = if inside {
            target.popover_focus.clone()
        } else {
            target.outside_focus.clone()
        };
        focus.focus(window, cx);
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
}

fn escape(cx: &mut TestAppContext, handle: WindowHandle<CoordView>) {
    let keystroke = Keystroke::parse("escape").unwrap();
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke,
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
}

fn closed(log: &Log) -> bool {
    log.borrow().iter().any(|(_, next)| !next)
}

#[gpui::test]
fn escape_closes_while_focus_is_inside_the_panel(cx: &mut TestAppContext) {
    let (handle, log) = open_view(cx, false);
    focus(cx, handle, true);
    escape(cx, handle);
    assert!(closed(&log), "Escape inside the panel must request a close");
}

#[gpui::test]
fn escape_outside_the_overlay_does_not_close_it(cx: &mut TestAppContext) {
    let (handle, log) = open_view(cx, false);
    focus(cx, handle, false);
    escape(cx, handle);
    assert!(
        !closed(&log),
        "Escape with focus elsewhere must not close a popover the user is not in"
    );
}

#[gpui::test]
fn a_controlled_caller_that_accepts_the_request_stays_closed(cx: &mut TestAppContext) {
    let (handle, log) = open_view(cx, true);
    focus(cx, handle, true);
    escape(cx, handle);
    assert!(
        closed(&log),
        "the close request must be reported to the caller"
    );

    // Accepting is the caller's job: drive the controlled value down and make sure
    // the popover does not resurrect itself on the next frames.
    cx.update_window(handle.into(), |view, _, cx| {
        view.downcast::<CoordView>()
            .unwrap()
            .update(cx, |view, cx| {
                view.open = false;
                cx.notify();
            });
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
    settle(cx, handle);
    let reopened = log.borrow().iter().any(|(_, next)| *next);
    assert!(
        !reopened,
        "a controlled popover with open == false must not report an open again"
    );
}

#[gpui::test]
fn an_uncontrolled_popover_stays_open_across_redraws(cx: &mut TestAppContext) {
    let (handle, log) = open_view(cx, false);
    settle(cx, handle);
    settle(cx, handle);
    assert!(
        !closed(&log),
        "a redraw must not reset an uncontrolled popover's open state"
    );
}
