//! What a `MenuTrigger` does with the open state and with the keyboard.
//!
//! Three failures here look fine in the source and wrong on screen. A
//! controlled trigger that toggles its own mirror asks its caller for the state
//! that is already showing. A close that grabs the keyboard before the popover
//! runs its hand-off leaves the keyboard on the trigger instead of the next
//! control. And a caller that moves the focus itself has to keep it.
//!
//! Each test drives the real input path and asserts what the user would see: the
//! requests the caller receives, whether the panel is painted, and which control
//! holds the keyboard.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, InputEvent, KeyBinding, KeyDownEvent, Keystroke, Modifiers,
    MouseButton, MouseDownEvent, MouseUpEvent, PlatformInput, Point, Render, TestAppContext,
    Window, WindowHandle, point, size,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Requests = Rc<RefCell<Vec<bool>>>;

struct TriggerView {
    before: FocusHandle,
    after: FocusHandle,
    /// Whether the caller owns the open state, and the value it renders.
    controlled: bool,
    open: Rc<Cell<bool>>,
    /// The initial value of the trigger's own store, which a controlled caller
    /// can disagree with.
    store_default: bool,
    /// Whether the caller accepts the requests it receives.
    accept: bool,
    /// Moves the keyboard to `after` from the close callback.
    move_focus_on_close: bool,
    requests: Requests,
}

impl Render for TriggerView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let requests = self.requests.clone();
        let accept = self.accept;
        let open = self.open.clone();
        let move_focus = self.move_focus_on_close;
        let after = self.after.clone();
        let trigger = MenuTrigger::new(
            "trigger-state",
            "Open",
            Menu::new("trigger-menu")
                .item(MenuItem::new("one", "One").on_click(|_, _| {}))
                .item(MenuItem::new("two", "Two").on_click(|_, _| {})),
        )
        .on_open_change(move |next, window, cx| {
            requests.borrow_mut().push(next);
            if accept {
                open.set(next);
            }
            if !next && move_focus {
                after.focus(window, cx);
            }
        });
        let trigger = if self.controlled {
            trigger
                .open(self.open.get())
                .default_open(self.store_default)
        } else {
            trigger.default_open(self.store_default)
        };

        FluentRoot::new().child(
            div()
                .relative()
                .size_full()
                .p(px(20.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(control("trigger-before", &self.before))
                        .child(trigger),
                )
                // Far enough to the right that the panel, which opens under the
                // trigger, never covers it.
                .child(
                    div()
                        .absolute()
                        .left(px(380.))
                        .top(px(20.))
                        .child(control("trigger-after", &self.after)),
                ),
        )
    }
}

fn control(id: &'static str, focus: &FocusHandle) -> impl IntoElement {
    div()
        .id(id)
        .track_focus(focus)
        .w(px(120.))
        .h(px(28.))
        .child(id)
}

#[derive(Clone)]
struct Handles {
    before: FocusHandle,
    after: FocusHandle,
}

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("escape", crate::MenuEscape, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("escape", crate::Dismiss, Some(crate::OVERLAY_KEY_CONTEXT)),
            KeyBinding::new("tab", crate::MenuTabOut, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new(
                "shift-tab",
                crate::MenuTabBackward,
                Some(crate::MENU_KEY_CONTEXT),
            ),
            // The Tab that follows the close happens outside the menu and has to
            // reach the page's traversal.
            KeyBinding::new("tab", crate::FocusNext, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", crate::FocusPrev, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("enter", crate::MenuActivate, Some(crate::MENU_KEY_CONTEXT)),
        ]);
    });
}

struct Setup {
    window: WindowHandle<TriggerView>,
    handles: Handles,
    requests: Requests,
    open: Rc<Cell<bool>>,
}

struct Config {
    controlled: bool,
    store_default: bool,
    accept: bool,
    move_focus_on_close: bool,
    /// The value the caller renders when it owns the state.
    open: bool,
}

fn open_view(cx: &mut TestAppContext, config: Config) -> Setup {
    bind_keys(cx);
    let requests: Requests = Default::default();
    let open = Rc::new(Cell::new(config.open));
    let slot: Rc<RefCell<Option<Handles>>> = Default::default();
    let (for_view, for_slot, for_open) = (requests.clone(), slot.clone(), open.clone());
    let window = cx.open_window(size(px(600.), px(400.)), move |_, cx| {
        let handles = Handles {
            before: cx.focus_handle().tab_stop(true),
            after: cx.focus_handle().tab_stop(true),
        };
        for_slot.borrow_mut().replace(handles.clone());
        TriggerView {
            before: handles.before.clone(),
            after: handles.after.clone(),
            controlled: config.controlled,
            open: for_open,
            store_default: config.store_default,
            accept: config.accept,
            move_focus_on_close: config.move_focus_on_close,
            requests: for_view,
        }
    });
    cx.run_until_parked();
    let handles = slot.borrow().clone().expect("the view built");
    let setup = Setup {
        window,
        handles,
        requests,
        open,
    };
    settle(cx, setup.window);
    setup
}

/// Draws twice with the next frame delivered, so a state change from the last
/// event is measured and any frame callback runs.
///
/// The clock moves past the menu reveal as well: a panel painted on the frame it
/// opened is still transparent, and these tests look for an opaque surface.
fn settle(cx: &mut TestAppContext, handle: WindowHandle<TriggerView>) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.simulate_next_frame(cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

/// The painted menu surface in logical pixels, when there is one.
fn panel(cx: &mut TestAppContext, handle: WindowHandle<TriggerView>) -> Option<Bounds<Pixels>> {
    let mut panel = None;
    cx.update_window(handle.into(), |_, window, _| {
        let scale = window.scale_factor();
        let to_px = |value: gpui::ScaledPixels| px(value.0 / scale);
        for quad in window.painted_quads() {
            let Some(colour) = quad.background.as_solid() else {
                continue;
            };
            let bounds = Bounds {
                origin: Point {
                    x: to_px(quad.bounds.origin.x),
                    y: to_px(quad.bounds.origin.y),
                },
                size: Size {
                    width: to_px(quad.bounds.size.width),
                    height: to_px(quad.bounds.size.height),
                },
            };
            // The menu is the only opaque surface narrower than the window and
            // taller than a single row.
            if colour.a > 0.9 && bounds.size.width < px(400.) && bounds.size.height > px(40.) {
                panel = Some(bounds);
            }
        }
    })
    .unwrap();
    panel
}

fn click(cx: &mut TestAppContext, handle: WindowHandle<TriggerView>, position: Point<Pixels>) {
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            MouseDownEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
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
    settle(cx, handle);
}

/// Clicks the trigger, which sits under the 20px page inset.
fn click_trigger(cx: &mut TestAppContext, handle: WindowHandle<TriggerView>) {
    click(cx, handle, point(px(60.), px(80.)));
}

fn press(cx: &mut TestAppContext, handle: WindowHandle<TriggerView>, key: &'static str) {
    let keystroke = Keystroke::parse(key).unwrap();
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        // A button activates when the key comes back up.
        window.dispatch_event(PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }), cx);
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
}

fn focused(cx: &mut TestAppContext, handle: WindowHandle<TriggerView>) -> Option<FocusHandle> {
    let mut out = None;
    cx.update_window(handle.into(), |_, window, cx| out = window.focused(cx))
        .unwrap();
    out
}

fn requests(setup: &Setup) -> Vec<bool> {
    setup.requests.borrow().clone()
}

/// Re-renders the view after the test changed something the caller owns.
fn notify(cx: &mut TestAppContext, handle: WindowHandle<TriggerView>) {
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<TriggerView>().unwrap();
        view.update(cx, |_, cx| cx.notify());
        window.refresh();
    })
    .unwrap();
    settle(cx, handle);
}

#[gpui::test]
fn a_rejected_controlled_request_is_asked_again(cx: &mut TestAppContext) {
    // The caller refuses every request, so the menu stays closed. Its second
    // click must ask for the same thing again: from the button's point of view
    // nothing has changed.
    let setup = open_view(
        cx,
        Config {
            controlled: true,
            store_default: false,
            accept: false,
            move_focus_on_close: false,
            open: false,
        },
    );
    click_trigger(cx, setup.window);
    click_trigger(cx, setup.window);

    assert_eq!(
        requests(&setup),
        [true, true],
        "a trigger whose request was refused must ask for the same state again"
    );
    assert!(
        panel(cx, setup.window).is_none(),
        "the panel must stay closed while the caller refuses to open it"
    );
}

#[gpui::test]
fn a_controlled_trigger_toggles_the_state_on_screen(cx: &mut TestAppContext) {
    // The caller accepts the first request and then closes the menu itself,
    // which the trigger's own store never hears about.
    let setup = open_view(
        cx,
        Config {
            controlled: true,
            store_default: false,
            accept: true,
            move_focus_on_close: false,
            open: false,
        },
    );

    click_trigger(cx, setup.window);
    assert!(panel(cx, setup.window).is_some(), "the click must open it");

    setup.open.set(false);
    notify(cx, setup.window);
    assert!(
        panel(cx, setup.window).is_none(),
        "the caller closed the menu"
    );

    click_trigger(cx, setup.window);
    assert_eq!(
        requests(&setup),
        [true, true],
        "the second click must reopen from the closed state on screen"
    );
    assert!(panel(cx, setup.window).is_some(), "and it must reopen");
}

#[gpui::test]
fn an_uncontrolled_trigger_still_toggles(cx: &mut TestAppContext) {
    let setup = open_view(
        cx,
        Config {
            controlled: false,
            store_default: false,
            accept: false,
            move_focus_on_close: false,
            open: false,
        },
    );
    click_trigger(cx, setup.window);
    assert!(
        panel(cx, setup.window).is_some(),
        "an uncontrolled trigger opens on the first click"
    );

    click_trigger(cx, setup.window);
    assert!(
        panel(cx, setup.window).is_none(),
        "and closes on the second"
    );
    assert_eq!(
        requests(&setup),
        [true, false],
        "the caller sees the requests, not the transition of the trigger's own store"
    );
}

#[gpui::test]
fn props_the_internal_default_disagrees_with_are_what_the_button_toggles(cx: &mut TestAppContext) {
    // Open from the props while the trigger's own store still says closed: the
    // button sits on an open menu, so its next click has to ask to close it.
    let setup = open_view(
        cx,
        Config {
            controlled: true,
            store_default: false,
            accept: false,
            move_focus_on_close: false,
            open: true,
        },
    );
    assert!(
        panel(cx, setup.window).is_some(),
        "the props must open the menu even though the store says closed"
    );

    click_trigger(cx, setup.window);
    assert_eq!(
        requests(&setup),
        [false],
        "the click must ask to close the menu that is on screen"
    );
}

#[gpui::test]
fn tab_hands_the_keyboard_to_the_next_control(cx: &mut TestAppContext) {
    let setup = open_view(
        cx,
        Config {
            controlled: false,
            store_default: false,
            accept: false,
            move_focus_on_close: false,
            open: false,
        },
    );
    click_trigger(cx, setup.window);
    assert!(panel(cx, setup.window).is_some(), "the menu must be open");

    press(cx, setup.window, "tab");
    assert!(panel(cx, setup.window).is_none(), "Tab must close the menu");
    assert_eq!(
        focused(cx, setup.window),
        Some(setup.handles.after.clone()),
        "Tab must leave the keyboard on the control after the trigger, not on the trigger"
    );
}

#[gpui::test]
fn shift_tab_hands_the_keyboard_backwards(cx: &mut TestAppContext) {
    let setup = open_view(
        cx,
        Config {
            controlled: false,
            store_default: false,
            accept: false,
            move_focus_on_close: false,
            open: false,
        },
    );
    click_trigger(cx, setup.window);
    assert!(panel(cx, setup.window).is_some(), "the menu must be open");

    press(cx, setup.window, "shift-tab");
    assert!(
        panel(cx, setup.window).is_none(),
        "Shift+Tab must close the menu"
    );
    assert_eq!(
        focused(cx, setup.window),
        Some(setup.handles.before.clone()),
        "Shift+Tab must move backwards from the trigger"
    );
}

#[gpui::test]
fn escape_with_no_previous_focus_returns_to_the_trigger(cx: &mut TestAppContext) {
    // A menu can open while nothing is focused: a programmatic open, or a press
    // that landed on a surface with no focus. The popover then has no previous
    // control to restore, and the trigger it opened from is the only sensible
    // place for the keyboard. Falling through would send it to the next control.
    let setup = open_view(
        cx,
        Config {
            controlled: true,
            store_default: false,
            accept: true,
            move_focus_on_close: false,
            open: false,
        },
    );
    assert!(
        focused(cx, setup.window).is_none(),
        "precondition: no control holds the keyboard before the menu opens"
    );

    setup.open.set(true);
    notify(cx, setup.window);
    assert!(panel(cx, setup.window).is_some(), "the menu must be open");

    press(cx, setup.window, "escape");
    assert!(
        panel(cx, setup.window).is_none(),
        "Escape must close the menu"
    );

    // Enter only reopens the menu if the keyboard really is on the trigger.
    press(cx, setup.window, "enter");
    assert!(
        panel(cx, setup.window).is_some(),
        "Escape must leave the keyboard on the trigger, so Enter reopens it"
    );
}
#[gpui::test]
fn pressing_another_control_keeps_its_focus(cx: &mut TestAppContext) {
    let setup = open_view(
        cx,
        Config {
            controlled: false,
            store_default: false,
            accept: false,
            move_focus_on_close: false,
            open: false,
        },
    );
    click_trigger(cx, setup.window);
    assert!(panel(cx, setup.window).is_some(), "the menu must be open");

    click(cx, setup.window, point(px(400.), px(30.)));
    assert!(
        panel(cx, setup.window).is_none(),
        "a press outside must close the menu"
    );
    assert_eq!(
        focused(cx, setup.window),
        Some(setup.handles.after.clone()),
        "closing because the user pressed another control must not take its focus"
    );
}

#[gpui::test]
fn a_caller_that_moves_focus_on_close_keeps_it(cx: &mut TestAppContext) {
    // The caller may want the keyboard somewhere specific after a close. The
    // popover must not undo that by restoring the trigger.
    let setup = open_view(
        cx,
        Config {
            controlled: false,
            store_default: false,
            accept: false,
            move_focus_on_close: true,
            open: false,
        },
    );
    click_trigger(cx, setup.window);
    assert!(panel(cx, setup.window).is_some(), "the menu must be open");

    press(cx, setup.window, "escape");
    assert!(
        panel(cx, setup.window).is_none(),
        "Escape must close the menu"
    );
    assert_eq!(
        focused(cx, setup.window),
        Some(setup.handles.after.clone()),
        "a focus move made in the close callback must survive"
    );
}
