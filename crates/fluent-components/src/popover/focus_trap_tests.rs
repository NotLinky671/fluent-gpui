//! Focus trapping inside a popover panel.
//!
//! An overlay that moves focus into itself also has to keep it there, otherwise
//! Tab walks into the page behind a panel the user is still looking at.
//!
//! Two facts make these tests possible and are easy to get wrong:
//!
//! * A `FocusHandle` is **not** a tab stop by default. Navigation skips non-stops,
//!   so a handle used as a tab stop must say `.tab_stop(true)`.
//! * Tab stops are recorded while painting and only become visible to
//!   `focus_next` on the frame after that, so a test must draw before pressing Tab.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, PlatformInput, Render,
    TestAppContext, Window, WindowHandle, size,
};

struct TrapView {
    panel: FocusHandle,
    first: FocusHandle,
    second: FocusHandle,
    /// Outside the popover; focus must never reach it while the trap holds.
    outside: FocusHandle,
}

impl Render for TrapView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let first = self.first.clone();
        let second = self.second.clone();
        FluentRoot::new().child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .p(px(20.))
                .child(
                    div()
                        .id("outside")
                        .track_focus(&self.outside)
                        .w(px(200.))
                        .h(px(28.))
                        .child("outside"),
                )
                .child(
                    Popover::new("trap-popover")
                        .default_open(true)
                        .track_focus(&self.panel)
                        .trigger(div().id("trap-trigger").w(px(120.)).h(px(28.)).child("t"))
                        .content(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .id("panel-first")
                                        .track_focus(&first)
                                        .w(px(150.))
                                        .h(px(28.))
                                        .child("first"),
                                )
                                .child(
                                    div()
                                        .id("panel-second")
                                        .track_focus(&second)
                                        .w(px(150.))
                                        .h(px(28.))
                                        .child("second"),
                                ),
                        ),
                ),
        )
    }
}

#[derive(Clone)]
struct Handles {
    panel: FocusHandle,
    first: FocusHandle,
    second: FocusHandle,
    outside: FocusHandle,
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<TrapView>, Handles) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("tab", crate::FocusNext, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", crate::FocusPrev, Some(crate::ROOT_KEY_CONTEXT)),
        ]);
    });
    let slot: std::rc::Rc<std::cell::RefCell<Option<Handles>>> = Default::default();
    let for_view = slot.clone();
    let window = cx.open_window(size(px(600.), px(500.)), move |_, cx| {
        // Every handle that must participate in Tab order is marked as a stop;
        // the panel's own handle is deliberately left out so the walk visits the
        // items rather than the container.
        let handles = Handles {
            panel: cx.focus_handle(),
            first: cx.focus_handle().tab_stop(true),
            second: cx.focus_handle().tab_stop(true),
            outside: cx.focus_handle().tab_stop(true),
        };
        for_view.replace(Some(handles.clone()));
        TrapView {
            panel: handles.panel,
            first: handles.first,
            second: handles.second,
            outside: handles.outside,
        }
    });
    cx.run_until_parked();
    let handles = slot.borrow().clone().expect("view built");
    (window, handles)
}

/// Focus `handle`, then draw twice so the resulting tab stops are visible to
/// `focus_next` on the following frame.
fn focus_and_settle(cx: &mut TestAppContext, handle: WindowHandle<TrapView>, focus: &FocusHandle) {
    cx.update_window(handle.into(), |_, window, cx| {
        focus.focus(window, cx);
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
}

fn press_tab(cx: &mut TestAppContext, handle: WindowHandle<TrapView>, shift: bool) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse(if shift { "shift-tab" } else { "tab" }).unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn focused(cx: &mut TestAppContext, handle: WindowHandle<TrapView>) -> Option<FocusHandle> {
    let mut out = None;
    cx.update_window(handle.into(), |_, window, cx| out = window.focused(cx))
        .unwrap();
    out
}

#[gpui::test]
fn tab_cycles_between_the_panel_items(cx: &mut TestAppContext) {
    let (handle, h) = open(cx);
    focus_and_settle(cx, handle, &h.first);
    assert_eq!(focused(cx, handle), Some(h.first.clone()), "start on first");

    press_tab(cx, handle, false);
    assert_eq!(
        focused(cx, handle),
        Some(h.second.clone()),
        "Tab moves from the first panel item to the second"
    );

    press_tab(cx, handle, false);
    assert_eq!(
        focused(cx, handle),
        Some(h.first.clone()),
        "Tab from the last panel item wraps back to the first"
    );
}

#[gpui::test]
fn tab_never_reaches_the_element_behind_the_panel(cx: &mut TestAppContext) {
    let (handle, h) = open(cx);
    focus_and_settle(cx, handle, &h.first);

    for step in 0..4 {
        press_tab(cx, handle, false);
        assert_ne!(
            focused(cx, handle),
            Some(h.outside.clone()),
            "Tab escaped to the element behind the panel on step {step}"
        );
    }
}

#[gpui::test]
fn shift_tab_wraps_backwards_inside_the_panel(cx: &mut TestAppContext) {
    let (handle, h) = open(cx);
    focus_and_settle(cx, handle, &h.first);

    press_tab(cx, handle, true);
    assert_eq!(
        focused(cx, handle),
        Some(h.second.clone()),
        "Shift+Tab from the first item wraps to the last"
    );
}
