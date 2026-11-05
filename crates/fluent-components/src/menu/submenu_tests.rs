//! Submenus: opening, navigating a level, and closing one level at a time.
//!
//! These assert end-to-end behaviour rather than state: which handler ran, and
//! whether the dropdown reported a close. That covers key dispatch, the tree
//! state and the level routing together.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, Modifiers,
    MouseMoveEvent, PlatformInput, Point, Render, TestAppContext, Window, WindowHandle, point,
    size,
};

type Log = std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>;
type OpenLog = std::rc::Rc<std::cell::RefCell<Vec<bool>>>;

struct SubmenuView {
    focus: FocusHandle,
    log: Log,
    opens: OpenLog,
}

fn child(log: Log, id: &'static str, label: &'static str) -> MenuItem {
    MenuItem::new(id, label).on_click(move |_, _| log.borrow_mut().push(label))
}

impl Render for SubmenuView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let log = self.log.clone();
        let menu = Menu::new("sub-menu")
            .item(
                MenuItem::new("row-more", "More").submenu(
                    Menu::new("sub-more")
                        .item(child(log.clone(), "child-one", "Child One"))
                        .item(child(log.clone(), "child-two", "Child Two")),
                ),
            )
            .item(child(log.clone(), "row-two", "Two"));

        let opens = self.opens.clone();
        FluentRoot::new().child(
            div().size_full().p(px(24.)).child(
                MenuDropdown::new(
                    "sub-dropdown",
                    div().id("sub-trigger").w(px(120.)).h(px(28.)).child("open"),
                    menu,
                )
                .default_open(true)
                .track_focus(&self.focus)
                .on_open_change(move |next, _, _| opens.borrow_mut().push(next)),
            ),
        )
    }
}

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("down", crate::MenuNext, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("up", crate::MenuPrev, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("enter", crate::MenuActivate, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new(
                "right",
                crate::MenuSubmenuOpen,
                Some(crate::MENU_KEY_CONTEXT),
            ),
            KeyBinding::new(
                "left",
                crate::MenuSubmenuClose,
                Some(crate::MENU_KEY_CONTEXT),
            ),
            KeyBinding::new("escape", crate::MenuEscape, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("tab", crate::MenuTabOut, Some(crate::MENU_KEY_CONTEXT)),
        ]);
    });
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<SubmenuView>, Log, OpenLog) {
    bind_keys(cx);
    let log: Log = Default::default();
    let opens: OpenLog = Default::default();
    let log_for_view = log.clone();
    let opens_for_view = opens.clone();
    let window = cx.open_window(size(px(600.), px(500.)), move |_, cx| SubmenuView {
        focus: cx.focus_handle().tab_stop(true),
        log: log_for_view,
        opens: opens_for_view,
    });
    cx.run_until_parked();
    (window, log, opens)
}

fn settle(cx: &mut TestAppContext, handle: WindowHandle<SubmenuView>) {
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

/// Focus the menu, settle, then dispatch `keys` in order.
fn press(cx: &mut TestAppContext, handle: WindowHandle<SubmenuView>, keys: &[&'static str]) {
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<SubmenuView>().unwrap();
        let focus = view.read(cx).focus.clone();
        focus.focus(window, cx);
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
    for key in keys {
        let keystroke = Keystroke::parse(key).unwrap();
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
}

fn closed(opens: &OpenLog) -> bool {
    opens.borrow().last() == Some(&false)
}

#[gpui::test]
fn activating_a_submenu_child_runs_that_child(cx: &mut TestAppContext) {
    let (handle, log, _) = open(cx);
    // Down lands on the row that opens the submenu, Right opens it, Down moves
    // inside it, Enter runs the child.
    press(cx, handle, &["down", "right", "down", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Child Two"],
        "Enter must run the submenu row, not the parent row"
    );
}

#[gpui::test]
fn escape_pops_one_level(cx: &mut TestAppContext) {
    let (handle, _, opens) = open(cx);
    press(cx, handle, &["down", "right", "escape"]);
    assert!(
        !closed(&opens),
        "the first Escape must close the submenu and leave the menu open"
    );
    press(cx, handle, &["escape"]);
    assert!(
        closed(&opens),
        "Escape at the root must close the whole menu"
    );
}

#[gpui::test]
fn tab_closes_the_whole_chain(cx: &mut TestAppContext) {
    let (handle, _, opens) = open(cx);
    press(cx, handle, &["down", "right", "tab"]);
    assert!(
        closed(&opens),
        "Tab must close every level, not just the submenu"
    );
}

#[gpui::test]
fn left_returns_to_the_parent_row(cx: &mut TestAppContext) {
    let (handle, log, _) = open(cx);
    // Left pops the level and leaves the parent row current, so the next Down
    // moves to the row below it rather than inside the closed submenu.
    press(cx, handle, &["down", "right", "left", "down", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Two"],
        "Left must hand the keyboard back to the parent row"
    );
}

#[test]
fn the_submenu_sits_right_of_the_row_that_opens_it() {
    // A row 200 wide, 32 tall, starting 12 in from the viewport origin and 40 down.
    let row = Bounds {
        origin: point(px(12.), px(40.)),
        size: size(px(200.), px(32.)),
    };
    let origin = submenu_origin(row, size(px(150.), px(120.)), size(px(800.), px(600.)));
    assert_eq!(origin.x, px(212.), "flush with the row right edge");
    assert_eq!(origin.y, px(40.), "level with the row top");
}

#[test]
fn the_submenu_placement_is_absolute_not_panel_relative() {
    // The row is already in window coordinates, so the same row relative to a
    // second panel that starts at another origin must not change the result.
    let row = Bounds {
        origin: point(px(112.), px(90.)),
        size: size(px(200.), px(32.)),
    };
    let origin = submenu_origin(row, size(px(150.), px(120.)), size(px(800.), px(600.)));
    assert_eq!(origin.x, px(312.));
    assert_eq!(origin.y, px(90.));
}

/// A plain menu (no dropdown) at a fixed position, so row coordinates are known.
struct HoverView {
    focus: FocusHandle,
    log: Log,
}

impl Render for HoverView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let log = self.log.clone();
        let menu = Menu::new("hover-menu")
            .item(
                MenuItem::new("hover-more", "More").submenu(
                    Menu::new("hover-sub")
                        .item(child(log.clone(), "hover-child-one", "Beta One"))
                        .item(child(log.clone(), "hover-child-two", "Beta Two")),
                ),
            )
            .item(child(log.clone(), "hover-two", "Two"));
        FluentRoot::new().child(
            div().size_full().p(px(24.)).child(
                div()
                    .w(px(240.))
                    .h(px(300.))
                    .child(menu.track_focus(&self.focus)),
            ),
        )
    }
}

fn open_hover(cx: &mut TestAppContext) -> (WindowHandle<HoverView>, Log) {
    bind_keys(cx);
    let log: Log = Default::default();
    let log_for_view = log.clone();
    let window = cx.open_window(size(px(600.), px(500.)), move |_, cx| HoverView {
        focus: cx.focus_handle().tab_stop(true),
        log: log_for_view,
    });
    cx.run_until_parked();
    let handle = window;
    for _ in 0..2 {
        cx.update_window(handle.into(), |view, window, cx| {
            let view = view.downcast::<HoverView>().unwrap();
            let focus = view.read(cx).focus.clone();
            focus.focus(window, cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    (handle, log)
}

/// Centre of the row at `index`, from the menu spec rather than a magic number.
fn row_centre(cx: &mut TestAppContext, index: usize) -> Point<gpui::Pixels> {
    let mut centre = point(px(0.), px(0.));
    cx.update(|cx| {
        let theme = FluentTheme::global(cx);
        let surface = menu::surface(&theme);
        let row = menu::item(
            &theme,
            &MenuItemProps {
                danger: false,
                disabled: false,
            },
        );
        let y = px(24.) + surface.padding + (row.height + px(2.)) * index as f32 + row.height / 2.;
        centre = point(px(24.) + px(40.), y);
    });
    centre
}

/// Focus the hover view and dispatch keys, mirroring `press` for the other view.
fn press_hover(cx: &mut TestAppContext, handle: WindowHandle<HoverView>, keys: &[&'static str]) {
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<HoverView>().unwrap();
        let focus = view.read(cx).focus.clone();
        focus.focus(window, cx);
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
    for key in keys {
        let keystroke = Keystroke::parse(key).unwrap();
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
        // The menu resolves its current level during render, so every key has
        // to be followed by a frame before the next one is dispatched.
        for _ in 0..2 {
            cx.update_window(handle.into(), |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .unwrap();
            cx.run_until_parked();
        }
    }
}

fn hover_row(cx: &mut TestAppContext, handle: WindowHandle<HoverView>, index: usize) {
    let position = row_centre(cx, index);
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
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

fn advance(cx: &mut TestAppContext, handle: WindowHandle<HoverView>, millis: u64) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(millis));
    cx.run_until_parked();
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
fn hovering_a_submenu_row_opens_it_after_the_delay(cx: &mut TestAppContext) {
    let (handle, log) = open_hover(cx);
    hover_row(cx, handle, 0);
    advance(cx, handle, 400);
    // The submenu opened on its own and its first row is current.
    press_hover(cx, handle, &["enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Beta One"],
        "a hovered submenu must open by itself and take the keyboard"
    );
}

#[gpui::test]
fn hovering_another_row_cancels_the_pending_open(cx: &mut TestAppContext) {
    let (handle, log) = open_hover(cx);
    hover_row(cx, handle, 0);
    advance(cx, handle, 150);
    // Leaving the row before the delay elapses drops the request.
    hover_row(cx, handle, 1);
    advance(cx, handle, 400);
    press_hover(cx, handle, &["enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Two"],
        "a cancelled hover must not open the submenu later"
    );
}

#[gpui::test]
fn typing_restarts_when_the_keyboard_enters_a_submenu(cx: &mut TestAppContext) {
    let (handle, log) = open_hover(cx);
    // "t" at the root selects "Two"; Up returns to the submenu row and Right
    // opens it. Typing "t" again must match from scratch, not extend "t" to "tt".
    press_hover(cx, handle, &["t"]);
    press_hover(cx, handle, &["up", "right"]);
    press_hover(cx, handle, &["b", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Beta Two"],
        "the prefix must restart after changing level, so the second child matches"
    );
}

#[test]
fn the_submenu_stays_right_when_it_fits() {
    let row = Bounds {
        origin: point(px(100.), px(40.)),
        size: size(px(200.), px(32.)),
    };
    let origin = submenu_origin(row, size(px(150.), px(120.)), size(px(800.), px(600.)));
    assert_eq!(origin.x, px(300.), "flush with the row right edge");
    assert_eq!(origin.y, px(40.));
}

#[test]
fn the_submenu_flips_left_when_it_would_leave_the_window() {
    let row = Bounds {
        origin: point(px(300.), px(40.)),
        size: size(px(150.), px(32.)),
    };
    // 300 + 150 + 150 = 600 > 500, so the panel goes to the row's left edge.
    let origin = submenu_origin(row, size(px(150.), px(120.)), size(px(500.), px(600.)));
    assert_eq!(origin.x, px(150.), "flipped to the row left edge");
}

#[test]
fn the_submenu_is_clamped_into_the_viewport_on_both_axes() {
    let row = Bounds {
        origin: point(px(10.), px(400.)),
        size: size(px(200.), px(32.)),
    };
    let origin = submenu_origin(row, size(px(150.), px(200.)), size(px(800.), px(500.)));
    assert_eq!(
        origin.y,
        px(300.),
        "pushed up so the panel ends at the viewport edge"
    );

    let row = Bounds {
        origin: point(px(4.), px(20.)),
        size: size(px(100.), px(32.)),
    };
    let origin = submenu_origin(row, size(px(300.), px(100.)), size(px(200.), px(500.)));
    assert_eq!(
        origin.x,
        px(0.),
        "a flipped panel that is wider than the space on the left is clamped"
    );
}
