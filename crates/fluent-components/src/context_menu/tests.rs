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
use gpui::StatefulInteractiveElement as _;
use gpui::{
    AppContext, Bounds, Context, FocusHandle, InputEvent as _, KeyBinding, Keystroke, Modifiers,
    MouseMoveEvent, Pixels, PlatformInput, Point, Render, ScrollDelta, ScrollEvent, ScrollHandle,
    TestAppContext, Window, WindowHandle, point, px, size,
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
    /// A page that records the request and deliberately does not accept it, to
    /// prove the next press asks again instead of trusting its own mirror.
    refuse: bool,
    /// How the host is clipped, if at all, by an ancestor.
    clip: Clip,
    scroll: ScrollHandle,
    /// A sibling that blocks presses over part of the attached region.
    occluded: bool,
}

/// The host is 160 px tall. `Hidden` shows the top 80 px of it; `Scrolled` puts
/// the same host inside an 80 px scrolling list so the visible part can move.
#[derive(Clone, Copy, PartialEq)]
enum Clip {
    None,
    Hidden,
    Scrolled,
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
                if !(view.refuse && next) {
                    view.open = next;
                }
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
        } else if self.clip == Clip::None {
            div()
                .relative()
                .flex_1()
                .w_full()
                .child(context)
                .into_any_element()
        } else {
            // Inside the clipping frame the column's flex height is gone, so
            // the host states its own size instead of filling the column.
            div()
                .relative()
                .w(px(300.))
                .h(px(160.))
                .child(context)
                .into_any_element()
        };

        // The host sits at 20..320 by 56..216; when clipped, only the top
        // 80 px of that (down to y = 136) can receive a press.
        let frame = match self.clip {
            Clip::None => host,
            Clip::Hidden => div()
                .id("context-clip")
                .w(px(300.))
                .h(px(80.))
                .overflow_hidden()
                .child(div().w(px(300.)).h(px(300.)).child(host))
                .into_any_element(),
            // Scrolling lives on `Stateful`, so `.id()` comes first here —
            // the opposite of `on_children_prepainted`, which only exists on
            // a plain `Div`.
            Clip::Scrolled => div()
                .w(px(300.))
                .h(px(80.))
                .id("context-clip")
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(div().w(px(300.)).h(px(300.)).child(host))
                .into_any_element(),
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
                .child(frame)
                .when(self.occluded, |el| {
                    el.child(
                        div()
                            .absolute()
                            .left(px(250.))
                            .top(px(170.))
                            .w(px(100.))
                            .h(px(100.))
                            .occlude(),
                    )
                }),
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
    open_with_all(cx, controlled, boxed, false, Clip::None)
}

/// A menu attached to a host that an ancestor clips, and — with `scrolled` — a
/// list whose visible window moves while the menu is open.
fn open_clipped(
    cx: &mut TestAppContext,
    controlled: bool,
    scrolled: bool,
) -> (WindowHandle<CtxView>, FocusHandle, Rc<RefCell<Vec<bool>>>) {
    let clip = if scrolled {
        Clip::Scrolled
    } else {
        Clip::Hidden
    };
    open_with_all(cx, controlled, false, false, clip)
}

/// A controlled page that refuses every open request it receives.
fn open_refusing(
    cx: &mut TestAppContext,
) -> (WindowHandle<CtxView>, FocusHandle, Rc<RefCell<Vec<bool>>>) {
    open_with_all(cx, true, false, true, Clip::None)
}

fn open_with_all(
    cx: &mut TestAppContext,
    controlled: bool,
    boxed: bool,
    refuse: bool,
    clip: Clip,
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
        refuse,
        clip,
        scroll: ScrollHandle::new(),
        occluded: false,
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

/// Let the page change the state it renders, the way a form would after an
/// accepted request turns into a rejection.
fn set_open(cx: &mut TestAppContext, handle: WindowHandle<CtxView>, open: bool) {
    cx.update_window(handle.into(), move |view, window, cx| {
        view.downcast::<CtxView>().unwrap().update(cx, |view, cx| {
            view.open = open;
            cx.notify();
        });
        window.refresh();
        window.draw(cx).clear(cx);
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

#[gpui::test]
fn a_rejected_request_is_asked_again_on_the_next_press(cx: &mut TestAppContext) {
    let (handle, _, log) = open_refusing(cx);

    press(cx, handle, point(px(220.), px(140.)), MouseButton::Right);
    assert!(
        panel(cx, handle).is_none(),
        "a page that refuses the request must keep the menu closed"
    );
    press(cx, handle, point(px(220.), px(140.)), MouseButton::Right);

    assert!(
        panel(cx, handle).is_none(),
        "the second press must not open a menu the page still refuses"
    );
    assert_eq!(
        *log.borrow(),
        vec![true, true],
        "every press must ask again, not trust the mirror of a rejected request"
    );
}

#[gpui::test]
fn a_menu_the_page_closed_can_be_asked_for_again(cx: &mut TestAppContext) {
    let (handle, _, log) = open_with(cx, true);
    press(cx, handle, point(px(220.), px(140.)), MouseButton::Right);
    assert!(
        panel(cx, handle).is_some(),
        "the accepted request must open the menu"
    );

    set_open(cx, handle, false);
    assert!(
        panel(cx, handle).is_none(),
        "the page must be able to close the menu it opened"
    );

    press(cx, handle, point(px(220.), px(140.)), MouseButton::Right);

    assert!(
        panel(cx, handle).is_some(),
        "a press after the page closed the menu must open it again"
    );
    assert_eq!(
        *log.borrow(),
        vec![true, true],
        "the reopened menu must ask once, without reporting a close the user did not make"
    );
}

#[gpui::test]
fn a_second_press_moves_the_menu_without_reporting_a_close(cx: &mut TestAppContext) {
    let (handle, _, log) = open(cx);
    press(cx, handle, point(px(120.), px(100.)), MouseButton::Right);
    let first = panel(cx, handle).expect("the first press must open the menu");

    // Inside the attached area, clear of the first panel so the press is a move.
    let target = point(px(420.), px(160.));
    assert!(
        !first.contains(&target),
        "the probe press must land outside the first panel, which sits at {first:?}"
    );
    press(cx, handle, target, MouseButton::Right);

    let second = panel(cx, handle).expect("the move must keep the menu on screen");
    assert!(
        (second.origin.x - px(420.)).abs() <= px(2.)
            && (second.origin.y - px(160.)).abs() <= px(2.),
        "the panel must move to the press, got {second:?}"
    );
    assert_eq!(
        *log.borrow(),
        vec![true],
        "a move inside the attached area must not report a close and a reopen"
    );
}

#[gpui::test]
fn a_controlled_move_reports_only_the_opening(cx: &mut TestAppContext) {
    let (handle, _, log) = open_with(cx, true);
    press(cx, handle, point(px(120.), px(100.)), MouseButton::Right);
    assert!(panel(cx, handle).is_some(), "the page accepted the opening");

    press(cx, handle, point(px(420.), px(160.)), MouseButton::Right);

    let moved = panel(cx, handle).expect("a controlled menu must move too");
    assert!(
        (moved.origin.x - px(420.)).abs() <= px(2.),
        "the controlled panel must follow the second press, got {moved:?}"
    );
    assert_eq!(
        *log.borrow(),
        vec![true],
        "the controlled page must not be told the menu closed in between"
    );
}

#[gpui::test]
fn a_left_press_outside_the_attached_area_still_closes(cx: &mut TestAppContext) {
    let (handle, _, log) = open(cx);
    press(cx, handle, point(px(220.), px(140.)), MouseButton::Right);
    let bounds = panel(cx, handle).expect("the press must open the menu");

    // Above the attached area, which starts below the first control.
    let target = point(px(300.), px(30.));
    assert!(
        !bounds.contains(&target),
        "the probe press must land outside the panel, which sits at {bounds:?}"
    );
    press(cx, handle, target, MouseButton::Left);

    assert!(
        panel(cx, handle).is_none(),
        "a press outside the attached area must dismiss the menu"
    );
    assert_eq!(
        *log.borrow(),
        vec![true, false],
        "the close is reported once"
    );
}

fn set_occluded(cx: &mut TestAppContext, handle: WindowHandle<CtxView>, occluded: bool) {
    handle
        .update(cx, |view, _, cx| {
            view.occluded = occluded;
            cx.notify();
        })
        .unwrap();
    settle(cx, handle);
}

fn assert_occluded_press_closes(cx: &mut TestAppContext, controlled: bool) {
    let (handle, _, log) = open_with(cx, controlled);
    set_occluded(cx, handle, true);
    press(cx, handle, point(px(60.), px(100.)), MouseButton::Right);
    let first = panel(cx, handle).expect("the uncovered part must open the menu");
    let covered = point(px(280.), px(180.));
    assert!(
        !first.contains(&covered),
        "the sibling must be outside the panel"
    );

    press(cx, handle, covered, MouseButton::Right);
    assert!(
        panel(cx, handle).is_none(),
        "a blocked region cannot receive a move, so the press must close"
    );
    assert_eq!(
        *log.borrow(),
        vec![true, false],
        "the dismissal must be reported once"
    );

    // Reopen, then remove the sibling while the menu is up. The current frame's
    // hitbox must replace the blocked one, without a stale dismissal filter.
    press(cx, handle, point(px(60.), px(100.)), MouseButton::Right);
    set_occluded(cx, handle, false);
    press(cx, handle, covered, MouseButton::Right);
    let moved = panel(cx, handle).expect("the uncovered region must receive a move again");
    assert!(
        (moved.origin.x - covered.x).abs() <= px(2.)
            && (moved.origin.y - covered.y).abs() <= px(2.),
        "the menu must move to the uncovered point, got {moved:?}"
    );
    assert_eq!(
        *log.borrow(),
        vec![true, false, true],
        "removing the sibling must allow a move without close/reopen callbacks"
    );
}

#[gpui::test]
fn a_right_press_on_an_occluding_sibling_closes_the_menu(cx: &mut TestAppContext) {
    assert_occluded_press_closes(cx, false);
}

#[gpui::test]
fn a_right_press_on_an_occluding_sibling_closes_a_controlled_menu(cx: &mut TestAppContext) {
    assert_occluded_press_closes(cx, true);
}

/// Scroll the list under the pointer, the way a wheel would.
fn wheel(
    cx: &mut TestAppContext,
    handle: WindowHandle<CtxView>,
    position: Point<Pixels>,
    delta_y: f32,
) {
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            PlatformInput::Scroll(ScrollEvent {
                position,
                delta: ScrollDelta::Pixels(point(px(0.), px(delta_y))),
                ..Default::default()
            }),
            cx,
        );
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
}

#[gpui::test]
fn a_press_in_the_clipped_away_part_of_the_region_closes_the_menu(cx: &mut TestAppContext) {
    let (handle, _, log) = open_clipped(cx, false, false);
    press(cx, handle, point(px(60.), px(100.)), MouseButton::Right);
    assert!(
        panel(cx, handle).is_some(),
        "the visible part of a clipped host must open the menu"
    );

    // Below the clipping window (which ends at y = 136) but still inside the
    // host's layout rectangle, which reaches y = 216.
    let target = point(px(260.), px(180.));
    press(cx, handle, target, MouseButton::Right);

    assert!(
        panel(cx, handle).is_none(),
        "a press the region cannot receive must close the menu, not count as a move"
    );
    assert_eq!(
        *log.borrow(),
        vec![true, false],
        "the clipped-away press must be reported as a close"
    );
}

#[gpui::test]
fn a_press_in_the_visible_part_of_a_clipped_region_moves_the_menu(cx: &mut TestAppContext) {
    let (handle, _, log) = open_clipped(cx, true, false);
    press(cx, handle, point(px(60.), px(100.)), MouseButton::Right);
    assert!(
        panel(cx, handle).is_some(),
        "the visible part opens the menu"
    );

    press(cx, handle, point(px(250.), px(120.)), MouseButton::Right);

    let moved = panel(cx, handle).expect("the menu must still be open after a move");
    assert!(
        (moved.origin.x - px(250.)).abs() <= px(2.) && (moved.origin.y - px(120.)).abs() <= px(2.),
        "the panel must move to the press inside the visible part, got {moved:?}"
    );
    assert_eq!(
        *log.borrow(),
        vec![true],
        "a move inside the visible part must not be reported as a close"
    );
}

#[gpui::test]
fn a_scrolled_clip_drops_the_part_of_the_region_that_scrolled_away(cx: &mut TestAppContext) {
    let (handle, _, log) = open_clipped(cx, false, true);
    press(cx, handle, point(px(60.), px(100.)), MouseButton::Right);
    assert!(panel(cx, handle).is_some(), "the list opens the menu");

    // Scrolling down by 120 px lifts the host by the same amount: it now ends
    // at y = 96, so y = 130 is on screen but no longer on the host.
    wheel(cx, handle, point(px(280.), px(100.)), -120.);

    press(cx, handle, point(px(260.), px(130.)), MouseButton::Right);

    assert!(
        panel(cx, handle).is_none(),
        "the filter must follow the scrolled mask instead of a stale rectangle"
    );
    assert_eq!(
        *log.borrow(),
        vec![true, false],
        "the press closes the menu"
    );
}

#[gpui::test]
fn a_visible_press_after_a_scroll_still_moves_the_menu(cx: &mut TestAppContext) {
    let (handle, _, log) = open_clipped(cx, false, true);
    press(cx, handle, point(px(60.), px(100.)), MouseButton::Right);
    assert!(panel(cx, handle).is_some(), "the list opens the menu");

    wheel(cx, handle, point(px(280.), px(100.)), -120.);
    press(cx, handle, point(px(250.), px(70.)), MouseButton::Right);

    let moved = panel(cx, handle).expect("the menu must still be open after a move");
    assert!(
        (moved.origin.y - px(70.)).abs() <= px(2.),
        "the panel must move to a press that is still on the host, got {moved:?}"
    );
    assert_eq!(*log.borrow(), vec![true], "a move must not report a close");
}
