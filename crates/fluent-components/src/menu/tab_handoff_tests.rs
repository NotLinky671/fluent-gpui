//! Tab and Shift+Tab must close the menu **and** leave the keyboard at the
//! neighbouring control. Closing alone used to strand focus on the panel handle:
//! the popover had also tracked that handle on the always-mounted trigger
//! wrapper, so gpui-base still considered the closed popover the active trap and
//! `FluentRoot` wrapped every following Tab back into it.
//!
//! These tests use a controlled dropdown so the open edge happens after the
//! trigger is focused (that is what a real click does). They assert focus
//! identity, not just that the panel disappeared.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AnyWindowHandle, AppContext, Context, FocusHandle, InputEvent, KeyBinding, KeyDownEvent,
    Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, PlatformInput, Point, Render,
    TestAppContext, Window, WindowHandle, point, size,
};

type Changes = std::rc::Rc<std::cell::RefCell<Vec<bool>>>;
type Slot<T> = std::rc::Rc<std::cell::RefCell<Option<T>>>;

struct HandoffView {
    trigger: FocusHandle,
    panel: FocusHandle,
    before: FocusHandle,
    after: FocusHandle,
    after_second: FocusHandle,
    controlled: bool,
    open: bool,
    default_open: bool,
    accept_close: bool,
    changes: Changes,
}

impl Render for HandoffView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let changes = self.changes.clone();
        let controlled = self.controlled;
        let accept_close = self.accept_close;
        let trigger = div()
            .id("handoff-trigger")
            .track_focus(&self.trigger)
            .w(px(200.))
            .h(px(28.))
            .child("open");

        let mut menu = Menu::new("handoff-panel").track_focus(&self.panel);
        menu = menu.item(MenuItem::new("more", "More").submenu(
            Menu::new("handoff-submenu").item(MenuItem::new("leaf", "Leaf").on_click(|_, _| {})),
        ));

        let mut dropdown = MenuDropdown::new("handoff-menu", trigger, menu)
            .track_focus(&self.panel)
            .on_open_change(move |next, _, cx| {
                changes.borrow_mut().push(next);
                if controlled && accept_close {
                    this.update(cx, |view, cx| {
                        view.open = next;
                        cx.notify();
                    });
                }
            });
        dropdown = if controlled {
            dropdown.open(self.open)
        } else {
            dropdown.default_open(self.default_open)
        };

        FluentRoot::new().child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .p(px(20.))
                .child(control("handoff-before", &self.before))
                .child(dropdown)
                .child(control("handoff-after", &self.after))
                .child(control("handoff-after-second", &self.after_second)),
        )
    }
}

fn control(id: &'static str, focus: &FocusHandle) -> impl IntoElement {
    div().id(id).track_focus(focus).h(px(28.)).child(id)
}

#[derive(Clone)]
struct Handles {
    trigger: FocusHandle,
    panel: FocusHandle,
    before: FocusHandle,
    after: FocusHandle,
    after_second: FocusHandle,
}

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("tab", crate::MenuTabOut, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new(
                "shift-tab",
                crate::MenuTabBackward,
                Some(crate::MENU_KEY_CONTEXT),
            ),
            // The next Tab happens outside the menu; it must reach FluentRoot's
            // traversal, which is where a leaked trap used to recapture it.
            KeyBinding::new("tab", crate::FocusNext, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", crate::FocusPrev, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("down", crate::MenuNext, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new(
                "right",
                crate::MenuSubmenuOpen,
                Some(crate::MENU_KEY_CONTEXT),
            ),
        ]);
    });
}

fn open_view(
    cx: &mut TestAppContext,
    controlled: bool,
    accept_close: bool,
) -> (WindowHandle<HandoffView>, Handles, Changes) {
    bind_keys(cx);
    let changes: Changes = Default::default();
    let for_view = changes.clone();
    let slot: Slot<Handles> = Default::default();
    let for_slot = slot.clone();
    let window = cx.open_window(size(px(600.), px(500.)), move |_, cx| {
        let handles = Handles {
            trigger: cx.focus_handle().tab_stop(true),
            panel: cx.focus_handle().tab_stop(true),
            before: cx.focus_handle().tab_stop(true),
            after: cx.focus_handle().tab_stop(true),
            after_second: cx.focus_handle().tab_stop(true),
        };
        for_slot.replace(Some(handles.clone()));
        HandoffView {
            trigger: handles.trigger.clone(),
            panel: handles.panel.clone(),
            before: handles.before.clone(),
            after: handles.after.clone(),
            after_second: handles.after_second.clone(),
            controlled,
            open: false,
            default_open: false,
            accept_close,
            changes: for_view,
        }
    });
    cx.run_until_parked();
    let handles = slot.borrow().clone().expect("view built");
    (window, handles, changes)
}

fn settle(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    for _ in 0..2 {
        cx.update_window(handle, |_, window, cx| {
            // Tests have no platform frame loop; deliver the callback that the
            // close render scheduled at the start of the next frame.
            window.simulate_next_frame(cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

fn focus_and_settle(cx: &mut TestAppContext, handle: AnyWindowHandle, focus: &FocusHandle) {
    for _ in 0..2 {
        cx.update_window(handle, |_, window, cx| {
            focus.focus(window, cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

fn set_open(cx: &mut TestAppContext, window: WindowHandle<HandoffView>, open: bool) {
    cx.update_window(window.into(), |view, window, cx| {
        let view = view.downcast::<HandoffView>().unwrap();
        view.update(cx, |view, cx| {
            view.open = open;
            cx.notify();
        });
        window.refresh();
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, window.into());
}

/// Press and release inside the trigger. An uncontrolled dropdown has no other
/// way to open, and a real click is also what makes the trigger the saved focus.
fn click_trigger(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    let position: Point<gpui::Pixels> = point(px(120.), px(70.));
    cx.update_window(handle, move |_, window, cx| {
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

fn press(cx: &mut TestAppContext, handle: AnyWindowHandle, key: &'static str) {
    let keystroke = Keystroke::parse(key).unwrap();
    cx.update_window(handle, move |_, window, cx| {
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

fn focused(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Option<FocusHandle> {
    let mut out = None;
    cx.update_window(handle, |_, window, cx| out = window.focused(cx))
        .unwrap();
    out
}

fn open_menu(
    cx: &mut TestAppContext,
    controlled: bool,
    accept_close: bool,
) -> (AnyWindowHandle, Handles, Changes) {
    let (window, handles, changes) = open_view(cx, controlled, accept_close);
    let any: AnyWindowHandle = window.into();
    // Focus the trigger first, then open, so the popover saves the trigger as
    // the focus to return to and advance from.
    focus_and_settle(cx, any, &handles.trigger);
    if controlled {
        set_open(cx, window, true);
    } else {
        click_trigger(cx, any);
    }
    assert_eq!(
        focused(cx, any),
        Some(handles.panel.clone()),
        "precondition: the open dropdown moved focus into the menu"
    );
    // The uncontrolled open reports `true`; each test only cares about the
    // close it is about to trigger.
    changes.borrow_mut().clear();
    (any, handles, changes)
}

#[gpui::test]
fn tab_closes_and_hands_focus_to_the_next_control(cx: &mut TestAppContext) {
    let (handle, h, changes) = open_menu(cx, false, false);

    press(cx, handle, "tab");
    assert_eq!(*changes.borrow(), vec![false]);
    assert_eq!(
        focused(cx, handle),
        Some(h.after.clone()),
        "Tab must close the menu and land on the control after the trigger"
    );

    // The old shared-handle trap also stole the *following* Tab. If the trap is
    // gone, traversal continues from `after`.
    press(cx, handle, "tab");
    assert_eq!(
        focused(cx, handle),
        Some(h.after_second.clone()),
        "the closed menu must not recapture the next Tab"
    );
}

#[gpui::test]
fn shift_tab_closes_and_hands_focus_to_the_previous_control(cx: &mut TestAppContext) {
    let (handle, h, changes) = open_menu(cx, false, false);

    press(cx, handle, "shift-tab");
    assert_eq!(*changes.borrow(), vec![false]);
    assert_eq!(
        focused(cx, handle),
        Some(h.before.clone()),
        "Shift+Tab must close the menu and move backwards from the trigger"
    );
}

#[gpui::test]
fn tab_closes_a_submenu_chain_and_hands_focus_forward(cx: &mut TestAppContext) {
    let (handle, h, changes) = open_menu(cx, false, false);
    press(cx, handle, "down");
    press(cx, handle, "right");

    press(cx, handle, "tab");
    assert_eq!(*changes.borrow(), vec![false]);
    assert_eq!(
        focused(cx, handle),
        Some(h.after.clone()),
        "Tab on a submenu closes the whole chain and still hands off once"
    );
}

#[gpui::test]
fn a_controlled_menu_that_accepts_close_hands_focus_forward(cx: &mut TestAppContext) {
    let (handle, h, changes) = open_menu(cx, true, true);

    press(cx, handle, "tab");
    assert_eq!(*changes.borrow(), vec![false]);
    assert_eq!(
        focused(cx, handle),
        Some(h.after.clone()),
        "an accepted controlled close must consume the forward direction"
    );
}

#[gpui::test]
fn a_controlled_menu_that_accepts_close_hands_focus_backward(cx: &mut TestAppContext) {
    let (handle, h, changes) = open_menu(cx, true, true);

    press(cx, handle, "shift-tab");
    assert_eq!(*changes.borrow(), vec![false]);
    assert_eq!(
        focused(cx, handle),
        Some(h.before.clone()),
        "an accepted controlled close must consume the backward direction"
    );
}

#[gpui::test]
fn a_controlled_menu_that_rejects_close_keeps_focus_inside(cx: &mut TestAppContext) {
    let (handle, h, changes) = open_menu(cx, true, false);

    press(cx, handle, "tab");
    assert_eq!(*changes.borrow(), vec![false]);
    assert_eq!(
        focused(cx, handle),
        Some(h.panel.clone()),
        "an open controlled menu must keep focus; it must not hand off while visible"
    );
}

#[gpui::test]
fn a_menu_opened_without_a_saved_focus_does_not_keep_the_trap_after_close(cx: &mut TestAppContext) {
    // Opening without a focused trigger leaves `previous` empty. The close can
    // only leave focus where it was; the old shared-handle wrapper kept that
    // panel handle mounted and the registered trap stayed active. The next Tab
    // must land on a real control, not be wrapped back into the closed menu.
    let (window, h, changes) = open_view(cx, true, true);
    let handle: AnyWindowHandle = window.into();
    set_open(cx, window, true);
    assert_eq!(focused(cx, handle), Some(h.panel.clone()));

    press(cx, handle, "tab");
    assert_eq!(*changes.borrow(), vec![false], "the controlled close ran");
    assert_ne!(
        focused(cx, handle),
        Some(h.panel.clone()),
        "closing without a saved handle must move focus off the closed panel"
    );
    press(cx, handle, "tab");
    assert_ne!(
        focused(cx, handle),
        Some(h.panel.clone()),
        "the closed panel's old handle must not keep the focus trap active"
    );
}

#[gpui::test]
fn a_refused_tab_does_not_leak_into_a_later_controlled_close(cx: &mut TestAppContext) {
    let (handle, h, changes) = open_menu(cx, true, false);
    press(cx, handle, "tab");
    assert_eq!(focused(cx, handle), Some(h.panel.clone()));

    // The caller now accepts a close, but not because of that Tab. The old
    // direction must not be consumed: focus returns to the trigger, and the
    // next real Tab starts traversal from there.
    cx.update_window(handle, |view, _, cx| {
        view.downcast::<HandoffView>()
            .unwrap()
            .update(cx, |view, cx| {
                view.accept_close = true;
                cx.notify();
            });
    })
    .unwrap();
    cx.run_until_parked();
    // Re-open state is unchanged; close it explicitly through the controlled
    // value.
    cx.update_window(handle, |view, _, cx| {
        view.downcast::<HandoffView>()
            .unwrap()
            .update(cx, |view, cx| {
                view.open = false;
                cx.notify();
            });
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);

    assert_eq!(
        focused(cx, handle),
        Some(h.trigger.clone()),
        "a later close must restore the trigger, not consume the refused Tab direction"
    );
    assert_eq!(
        *changes.borrow(),
        vec![false],
        "still only the refused request"
    );

    press(cx, handle, "tab");
    assert_eq!(
        focused(cx, handle),
        Some(h.after.clone()),
        "the next Tab starts at the restored trigger"
    );
}
