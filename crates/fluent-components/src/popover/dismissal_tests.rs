//! Behaviour of the popover's dismissal paths.
//!
//! These tests drive real pointer and key events through a window instead of
//! calling the closures directly, because the interesting question is *which*
//! element receives the event — a hit-test result, not a callback shape.

use super::*;
use gpui::{
    AppContext, Context, FocusHandle, InputEvent as _, KeyBinding, KeyDownEvent, Keystroke,
    Modifiers, MouseButton, MouseDownEvent, PlatformInput, Render, TestAppContext, WindowHandle,
    canvas, size,
};

/// What the caller was told, in order.
#[derive(Clone, Default)]
struct Log {
    events: std::rc::Rc<std::cell::RefCell<Vec<bool>>>,
    /// Presses that reached the element sitting under the panel.
    beneath_hits: std::rc::Rc<std::cell::Cell<usize>>,
    /// Where each interesting element ended up, so tests can aim at them.
    trigger: TriggerBounds,
    content: TriggerBounds,
    inner_button: TriggerBounds,
}

impl Log {
    fn take(&self) -> Vec<bool> {
        std::mem::take(&mut *self.events.borrow_mut())
    }
}

struct DismissView {
    log: Log,
    open: Option<bool>,
    default_open: bool,
    focus: FocusHandle,
    /// Focus that exists outside the popover, to prove it is handed back.
    outside: FocusHandle,
}

impl Render for DismissView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let log = self.log.clone();
        let events = log.events.clone();
        let trigger = log.trigger.clone();
        let content = log.content.clone();
        let inner_button = log.inner_button.clone();

        let mut popover = Popover::new("dismiss-popover")
            .trigger(
                canvas(
                    move |bounds, _, _| trigger.set(Some(bounds)),
                    |_, (), _, _| {},
                )
                .w(px(120.))
                .h(px(32.)),
            )
            .content(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        canvas(
                            move |bounds, _, _| content.set(Some(bounds)),
                            |_, (), _, _| {},
                        )
                        .w(px(160.))
                        .h(px(24.)),
                    )
                    .child(
                        div()
                            .id("popover-inner-button")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(160.))
                            .h(px(32.))
                            .on_click(|_, _, _| {})
                            .child(
                                canvas(
                                    move |bounds, _, _| inner_button.set(Some(bounds)),
                                    |_, (), _, _| {},
                                )
                                .size_full(),
                            ),
                    ),
            )
            .on_open_change(move |next, _, _| events.borrow_mut().push(next));

        if let Some(open) = self.open {
            popover = popover.open(open);
        } else {
            popover = popover.default_open(self.default_open);
        }

        let beneath = log.beneath_hits.clone();
        div()
            .size_full()
            .p(px(40.))
            // A press that reaches this means the panel failed to block the
            // mouse, so it sits underneath where the panel will open.
            .child(
                div()
                    .id("beneath-panel")
                    .absolute()
                    .left(px(40.))
                    .top(px(80.))
                    .w(px(240.))
                    .h(px(80.))
                    .on_mouse_down(MouseButton::Left, move |_, _, _| {
                        beneath.set(beneath.get() + 1)
                    }),
            )
            .child(popover.track_focus(&self.focus))
    }
}

fn open_view(
    cx: &mut TestAppContext,
    open: Option<bool>,
    default_open: bool,
) -> (WindowHandle<DismissView>, Log) {
    let log = Log::default();
    let handle_log = log.clone();
    let window = cx.open_window(size(px(600.), px(400.)), move |_, cx| DismissView {
        log: handle_log,
        open,
        default_open,
        focus: cx.focus_handle(),
        outside: cx.focus_handle(),
    });
    cx.run_until_parked();
    (window, log)
}

/// A press-and-release at `position`, as the platform would deliver it.
fn click_at(cx: &mut TestAppContext, handle: WindowHandle<DismissView>, position: Point<Pixels>) {
    cx.update_window(handle.into(), |_, window, cx| {
        let event = MouseDownEvent {
            button: MouseButton::Left,
            position,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        };
        window.dispatch_event(event.to_platform_input(), cx);
        window.dispatch_event(
            PlatformInput::MouseUp(gpui::MouseUpEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
}

fn press_escape(cx: &mut TestAppContext, handle: WindowHandle<DismissView>) {
    cx.update(|cx| {
        cx.bind_keys([KeyBinding::new(
            "escape",
            crate::Dismiss,
            Some(crate::OVERLAY_KEY_CONTEXT),
        )]);
    });
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<DismissView>().unwrap();
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse("escape").unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
}

/// Centre of a measured bounds, in window coordinates.
fn centre(bounds: Bounds<Pixels>) -> Point<Pixels> {
    point(
        bounds.origin.x + bounds.size.width / 2.,
        bounds.origin.y + bounds.size.height / 2.,
    )
}

#[gpui::test]
fn outside_click_requests_a_close(cx: &mut TestAppContext) {
    let (handle, log) = open_view(cx, None, true);
    let trigger = log.trigger.get().expect("trigger measured");
    // Well below the trigger and outside the panel.
    click_at(
        cx,
        handle,
        point(trigger.left(), trigger.bottom() + px(300.)),
    );
    assert_eq!(log.take(), vec![false], "outside click must report false");
}

#[gpui::test]
fn click_on_panel_surface_keeps_it_open(cx: &mut TestAppContext) {
    let (handle, log) = open_view(cx, None, true);
    let content = log.content.get().expect("panel measured");
    // Blank surface area inside the panel, away from the inner button.
    click_at(cx, handle, centre(content));
    assert!(
        log.take().is_empty(),
        "a click on the panel body must not reach the backdrop"
    );

    assert_eq!(
        log.beneath_hits.get(),
        0,
        "a click on the panel must not fall through to the element underneath"
    );
}

#[gpui::test]
fn click_on_interactive_child_inside_panel_keeps_it_open(cx: &mut TestAppContext) {
    let (handle, log) = open_view(cx, None, true);
    let button = log.inner_button.get().expect("inner button measured");
    click_at(cx, handle, centre(button));
    assert!(
        log.take().is_empty(),
        "a click on a panel child must not reach the backdrop"
    );
    assert_eq!(
        log.beneath_hits.get(),
        0,
        "a click on a panel child must not fall through to the element underneath"
    );
}

#[gpui::test]
fn clicking_the_trigger_again_toggles_instead_of_double_reporting(cx: &mut TestAppContext) {
    let (handle, log) = open_view(cx, None, true);
    let trigger = log.trigger.get().expect("trigger measured");
    click_at(cx, handle, centre(trigger));
    // The trigger's own toggle closes it; the backdrop must not add a second
    // report for the same press.
    assert_eq!(
        log.take(),
        vec![false],
        "one press on the trigger reports exactly one state change"
    );
}

#[gpui::test]
fn controlled_caller_that_ignores_the_request_stays_open(cx: &mut TestAppContext) {
    let (handle, log) = open_view(cx, Some(true), false);
    let trigger = log.trigger.get().expect("trigger measured");
    click_at(
        cx,
        handle,
        point(trigger.left(), trigger.bottom() + px(300.)),
    );
    assert_eq!(
        log.take(),
        vec![false],
        "the caller is told about the close request"
    );
    // The view never updated `open`, so the panel must still be there.
    assert!(
        log.content.get().is_some(),
        "an ignored close request must not hide a controlled popover"
    );
}

#[gpui::test]
fn escape_requests_a_close_while_focus_is_inside(cx: &mut TestAppContext) {
    let (handle, log) = open_view(cx, None, true);
    press_escape(cx, handle);
    assert_eq!(
        log.take(),
        vec![false],
        "Escape must request a close when the trigger subtree holds focus"
    );
}

/// Focus the view's outside handle, as an application would before opening.
fn focus_outside(cx: &mut TestAppContext, handle: WindowHandle<DismissView>) {
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<DismissView>().unwrap();
        let outside = view.read(cx).outside.clone();
        window.focus(&outside, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn panel_focused(cx: &mut TestAppContext, handle: WindowHandle<DismissView>) -> bool {
    let mut focused = false;
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<DismissView>().unwrap();
        let focus = view.read(cx).focus.clone();
        focused = focus.is_focused(window);
    })
    .unwrap();
    focused
}

fn outside_focused(cx: &mut TestAppContext, handle: WindowHandle<DismissView>) -> bool {
    let mut focused = false;
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<DismissView>().unwrap();
        let outside = view.read(cx).outside.clone();
        focused = outside.is_focused(window);
    })
    .unwrap();
    focused
}

#[gpui::test]
fn opening_moves_focus_into_the_panel(cx: &mut TestAppContext) {
    let (handle, _log) = open_view(cx, None, false);
    focus_outside(cx, handle);
    assert!(outside_focused(cx, handle), "precondition: focus outside");

    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<DismissView>().unwrap();
        view.update(cx, |view, cx| {
            view.open = Some(true);
            cx.notify();
        });
        window.refresh();
    })
    .unwrap();
    cx.run_until_parked();

    assert!(
        panel_focused(cx, handle),
        "opening must move focus into the panel so the keyboard reaches its content"
    );
}

#[gpui::test]
fn closing_returns_focus_to_where_it_was(cx: &mut TestAppContext) {
    let (handle, _log) = open_view(cx, None, false);
    focus_outside(cx, handle);

    // Open first and prove focus actually moved, otherwise "focus is back
    // outside" would pass even with no handover at all.
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<DismissView>().unwrap();
        view.update(cx, |view, cx| {
            view.open = Some(true);
            cx.notify();
        });
        window.refresh();
    })
    .unwrap();
    cx.run_until_parked();
    assert!(
        panel_focused(cx, handle),
        "precondition: opening moved focus into the panel"
    );

    // Now close.
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<DismissView>().unwrap();
        view.update(cx, |view, cx| {
            view.open = Some(false);
            cx.notify();
        });
        window.refresh();
    })
    .unwrap();
    cx.run_until_parked();

    assert!(
        outside_focused(cx, handle),
        "closing must hand focus back to what had it, otherwise the keyboard \
         is stranded on a panel that no longer exists"
    );
}
