//! A context menu opens where the pointer is, not where a trigger happens to
//! sit, and it hands the keyboard back when it closes.
//!
//! `Popover` already places a panel against a trigger rectangle. A context menu
//! has no such rectangle: its anchor is the pointer position at the moment of
//! the press, so the panel can only be positioned once the press has happened.
//! These tests pin what that new anchor brings — the panel origin follows the
//! press, near an edge it flips instead of leaving the window — and the sum of
//! region plus popover: a press outside the panel dismisses it, Escape closes
//! it and returns the keyboard, and a second right click moves the panel rather
//! than stacking a second one.

use super::*;
use crate::menu::{Menu, MenuItem};
use crate::root::FluentRoot;
use gpui::{
    AppContext, Bounds, Context, FocusHandle, InputEvent as _, KeyBinding, Keystroke, Modifiers,
    MouseMoveEvent, Pixels, PlatformInput, Point, Render, TestAppContext, Window, WindowHandle,
    point, px, size,
};
use std::cell::RefCell;
use std::time::Duration;

/// Rows in the probe menu. Six rows of 32 px plus the surface padding make a
/// panel around 160 by 210, which is easy to tell apart from the window.
const ROWS: usize = 6;

struct CtxView {
    focus: FocusHandle,
    log: Rc<RefCell<Vec<bool>>>,
    /// When set, the menu is driven by the page the way the gallery drives it:
    /// the request comes back through `on_open_change` and only the page can
    /// make it open.
    controlled: bool,
    open: bool,
    /// The gallery attaches the menu to a fixed-size card inside a column,
    /// rather than to the column itself.
    boxed: bool,
}

impl Render for CtxView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut menu = Menu::new("context-menu");
        for index in 0..ROWS {
            menu = menu.item(MenuItem::new(
                ElementId::from(SharedString::from(format!("context-row-{index}"))),
                SharedString::from(format!("Row {index}")),
            ));
        }

        let log = self.log.clone();
        let entity = cx.entity();
        let on_change = move |next: bool, _: &mut Window, cx: &mut App| {
            log.borrow_mut().push(next);
            entity.update(cx, |view, cx| {
                view.open = next;
                cx.notify();
            });
        };
        let controlled = self.controlled;
        let open = self.open;
        let context = ContextMenu::new("context-probe", menu)
            .region(div().size_full())
            .when(controlled, |el| el.open(open))
            .on_open_change(on_change)
            .into_any_element();
        let host = if self.boxed {
            div()
                .relative()
                .w(px(280.))
                .h(px(160.))
                .rounded(px(6.))
                .child(div().size_full().child("Right click"))
                .child(context)
                .into_any_element()
        } else {
            div()
                .relative()
                .flex_1()
                .w_full()
                .child(context)
                .into_any_element()
        };

        FluentRoot::new().child(
            div()
                .size_full()
                .p(px(20.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .id("context-before")
                        .track_focus(&self.focus)
                        .w(px(120.))
                        .h(px(28.))
                        .child("before"),
                )
                .child(host),
        )
    }
}

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("escape", crate::MenuEscape, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("escape", crate::Dismiss, Some(crate::OVERLAY_KEY_CONTEXT)),
        ]);
    });
}

fn settle(cx: &mut TestAppContext, handle: WindowHandle<CtxView>) {
    // The menu fades in from zero opacity, so a frame taken at time zero is a
    // frame with no visible surface at all.
    cx.executor().advance_clock(Duration::from_millis(400));
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<CtxView>, FocusHandle, Rc<RefCell<Vec<bool>>>) {
    open_with(cx, false)
}

fn open_with(
    cx: &mut TestAppContext,
    controlled: bool,
) -> (WindowHandle<CtxView>, FocusHandle, Rc<RefCell<Vec<bool>>>) {
    open_with_flags(cx, controlled, false)
}

fn open_with_flags(
    cx: &mut TestAppContext,
    controlled: bool,
    boxed: bool,
) -> (WindowHandle<CtxView>, FocusHandle, Rc<RefCell<Vec<bool>>>) {
    bind_keys(cx);
    let log: Rc<RefCell<Vec<bool>>> = Rc::new(RefCell::new(Vec::new()));
    let window_log = log.clone();
    let handle = cx.open_window(size(px(600.), px(400.)), move |_, cx| CtxView {
        focus: cx.focus_handle().tab_stop(true),
        log: window_log,
        controlled,
        open: false,
        boxed,
    });
    cx.run_until_parked();
    settle(cx, handle);
    let before = cx
        .update_window(handle.into(), |view, _, cx| {
            view.downcast::<CtxView>().unwrap().read(cx).focus.clone()
        })
        .unwrap();
    (handle, before, log)
}

/// The panel the menu paints, in logical pixels.
///
/// The menu surface and the window background share a colour, so the panel is
/// found by its size: an opaque quad that is narrower than the window but tall
/// enough to hold six rows.
fn panel(cx: &mut TestAppContext, handle: WindowHandle<CtxView>) -> Option<Bounds<Pixels>> {
    cx.update_window(handle.into(), |_, window, _| {
        let scale = window.scale_factor();
        let mut found = None;
        for quad in window.painted_quads() {
            let Some(colour) = quad.background.as_solid() else {
                continue;
            };
            let width = quad.bounds.size.width.0 / scale;
            let height = quad.bounds.size.height.0 / scale;
            if colour.a > 0.9 && (100.0..400.0).contains(&width) && height > 100.0 {
                found = Some(Bounds {
                    origin: point(
                        px(quad.bounds.origin.x.0 / scale),
                        px(quad.bounds.origin.y.0 / scale),
                    ),
                    size: size(px(width), px(height)),
                });
            }
        }
        found
    })
    .unwrap()
}

fn move_to(cx: &mut TestAppContext, handle: WindowHandle<CtxView>, position: Point<Pixels>) {
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }),
            cx,
        );
    })
    .unwrap();
}

/// A press and release at `position`, as the platform would deliver it.
fn press(
    cx: &mut TestAppContext,
    handle: WindowHandle<CtxView>,
    position: Point<Pixels>,
    button: MouseButton,
) {
    move_to(cx, handle, position);
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            MouseDownEvent {
                button,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(gpui::MouseUpEvent {
                button,
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

fn press_escape(cx: &mut TestAppContext, handle: WindowHandle<CtxView>) {
    cx.update_window(handle.into(), |_, window, cx| {
        let keystroke = Keystroke::parse("escape").expect("escape is a keystroke");
        window.dispatch_event(
            PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.dispatch_event(PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }), cx);
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
}

fn focused(cx: &mut TestAppContext, handle: WindowHandle<CtxView>) -> Option<FocusHandle> {
    cx.update_window(handle.into(), |_, window, cx| window.focused(cx))
        .unwrap()
}

#[gpui::test]
fn a_right_click_inside_a_fixed_card_still_opens_the_menu(cx: &mut TestAppContext) {
    let (handle, _, log) = open_with_flags(cx, true, true);
    assert!(panel(cx, handle).is_none(), "the menu starts closed");

    // The card spans 20..300 by 36..196 inside the padded column.
    press(cx, handle, point(px(160.), px(100.)), MouseButton::Right);

    assert!(
        panel(cx, handle).is_some(),
        "a menu attached to a fixed card must answer a right press"
    );
    assert_eq!(*log.borrow(), vec![true], "opening must be reported once");
}

#[gpui::test]
fn a_controlled_right_click_still_opens_the_menu(cx: &mut TestAppContext) {
    let (handle, _, log) = open_with(cx, true);
    assert!(
        panel(cx, handle).is_none(),
        "a controlled menu starts closed when the page says so"
    );

    press(cx, handle, point(px(220.), px(140.)), MouseButton::Right);

    let bounds = panel(cx, handle).expect("a controlled menu must open when the page accepts");
    assert!(
        (bounds.origin.x - px(220.)).abs() <= px(2.)
            && (bounds.origin.y - px(140.)).abs() <= px(2.),
        "the panel must start at the press, got {bounds:?}"
    );
    assert_eq!(*log.borrow(), vec![true], "opening must be reported once");
}

#[gpui::test]
fn a_right_click_opens_the_menu_at_the_pointer(cx: &mut TestAppContext) {
    let (handle, _, log) = open(cx);
    assert!(
        panel(cx, handle).is_none(),
        "the menu must not be on screen before the press"
    );

    press(cx, handle, point(px(220.), px(140.)), MouseButton::Right);

    let bounds = panel(cx, handle).expect("a right click must open the menu");
    assert!(
        (bounds.origin.x - px(220.)).abs() <= px(2.)
            && (bounds.origin.y - px(140.)).abs() <= px(2.),
        "the panel must start at the press, got {bounds:?} for a press at 220,140"
    );
    assert_eq!(*log.borrow(), vec![true], "opening must be reported once");
}

#[gpui::test]
fn a_right_click_near_the_window_edge_keeps_the_menu_on_screen(cx: &mut TestAppContext) {
    let (handle, _, _) = open(cx);

    press(cx, handle, point(px(560.), px(370.)), MouseButton::Right);

    let bounds = panel(cx, handle).expect("a press near the edge must still open the menu");
    assert!(
        bounds.origin.x >= px(8.) && bounds.origin.y >= px(8.),
        "the panel must stay inside the window margins, got {bounds:?}"
    );
    assert!(
        bounds.right() <= px(592.) && bounds.bottom() <= px(392.),
        "the panel must not overhang the window, got {bounds:?}"
    );
    assert!(
        (bounds.right() - px(560.)).abs() <= px(2.),
        "a panel that would overhang flips to the left of the press, got right {:?}",
        bounds.right()
    );
    assert!(
        (bounds.bottom() - px(370.)).abs() <= px(2.),
        "a panel that would overhang flips above the press, got bottom {:?}",
        bounds.bottom()
    );
}

#[gpui::test]
fn escape_closes_the_menu(cx: &mut TestAppContext) {
    let (handle, _, log) = open(cx);
    press(cx, handle, point(px(220.), px(140.)), MouseButton::Right);
    assert!(panel(cx, handle).is_some(), "the press must open the menu");

    press_escape(cx, handle);

    assert!(
        panel(cx, handle).is_none(),
        "Escape must close the context menu"
    );
    assert_eq!(
        *log.borrow(),
        vec![true, false],
        "closing must be reported once"
    );
}

#[gpui::test]
fn escape_returns_the_keyboard_to_the_control_that_had_it(cx: &mut TestAppContext) {
    let (handle, before, _) = open(cx);
    cx.update_window(handle.into(), |view, window, cx| {
        let focus = view.downcast::<CtxView>().unwrap().read(cx).focus.clone();
        window.focus(&focus, cx);
    })
    .unwrap();
    settle(cx, handle);

    press(cx, handle, point(px(220.), px(140.)), MouseButton::Right);
    assert!(panel(cx, handle).is_some(), "the press must open the menu");
    assert_ne!(
        focused(cx, handle).as_ref(),
        Some(&before),
        "the open menu must take the keyboard while it is up"
    );

    press_escape(cx, handle);

    assert_eq!(
        focused(cx, handle).as_ref(),
        Some(&before),
        "closing must give the keyboard back to where it was"
    );
}

#[gpui::test]
fn a_press_outside_the_panel_closes_the_menu(cx: &mut TestAppContext) {
    let (handle, _, log) = open(cx);
    press(cx, handle, point(px(220.), px(140.)), MouseButton::Right);
    let bounds = panel(cx, handle).expect("the press must open the menu");

    press(cx, handle, point(px(60.), px(300.)), MouseButton::Left);

    assert!(
        !bounds.contains(&point(px(60.), px(300.))),
        "the probe press must land outside the panel, which sits at {bounds:?}"
    );
    assert!(
        panel(cx, handle).is_none(),
        "a press elsewhere in the region must dismiss the menu"
    );
    assert_eq!(
        *log.borrow(),
        vec![true, false],
        "the dismissal must be reported once"
    );
}

#[gpui::test]
fn a_second_right_click_moves_the_menu(cx: &mut TestAppContext) {
    let (handle, _, _) = open(cx);
    press(cx, handle, point(px(120.), px(100.)), MouseButton::Right);
    let first = panel(cx, handle).expect("the first press must open the menu");

    // Far enough right to leave the first panel, and high enough that the
    // panel fits below the press instead of flipping.
    press(cx, handle, point(px(320.), px(160.)), MouseButton::Right);

    let second = panel(cx, handle).expect("the menu must still be on screen");
    assert!(
        (second.origin.x - px(320.)).abs() <= px(2.)
            && (second.origin.y - px(160.)).abs() <= px(2.),
        "the panel must move to the new press, got {second:?} after {first:?}"
    );
}
