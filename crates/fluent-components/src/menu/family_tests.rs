//! The rest of Fluent's menu family: a trigger button, grouped rows, two-line
//! rows and the checkable row types.
//!
//! These are end-to-end tests. A trigger that paints but never opens, a group
//! whose heading steals a keyboard stop, or a two-line row that keeps the
//! single-line height all look plausible in the source and wrong on screen, so
//! each test drives the real path and measures the painted result.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, InputEvent as _, KeyBinding, KeyDownEvent, Keystroke,
    Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, PlatformInput, Render, TestAppContext,
    Window, WindowHandle, point, px, size,
};

/// What the test window shows.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Three single-line rows.
    Single,
    /// Three rows that each carry a secondary line.
    TwoLine,
    /// A labelled group of two rows.
    Group,
    /// One checkable row.
    Checkbox,
}

struct FamilyView {
    kind: Kind,
    focus: FocusHandle,
    /// Whether the application hands the trigger a menu focus handle. A trigger
    /// without one still has to be usable from the mouse.
    tracks_focus: bool,
    log: std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>,
}

impl FamilyView {
    fn row(&self, id: &'static str, label: &'static str) -> MenuItem {
        let log = self.log.clone();
        MenuItem::new(id, label).on_click(move |_, _| log.borrow_mut().push(label))
    }

    fn menu(&self) -> Menu {
        match self.kind {
            Kind::Single => Menu::new("family-menu")
                .item(self.row("one", "One"))
                .item(self.row("two", "Two"))
                .item(self.row("three", "Three")),
            Kind::TwoLine => Menu::new("family-menu")
                .item(self.row("one", "One").secondary("First"))
                .item(self.row("two", "Two").secondary("Second"))
                .item(self.row("three", "Three").secondary("Third")),
            Kind::Group => Menu::new("family-menu").group(
                MenuGroup::new("Actions")
                    .item(self.row("one", "One"))
                    .item(self.row("two", "Two")),
            ),
            Kind::Checkbox => {
                let log = self.log.clone();
                Menu::new("family-menu").item(
                    MenuItemCheckbox::new("check", "Bold")
                        .checked(true)
                        .on_click(move |_, _| log.borrow_mut().push("check")),
                )
            }
        }
    }
}

impl Render for FamilyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let trigger = MenuTrigger::new("family-trigger", "Open", self.menu());
        let trigger = if self.tracks_focus {
            trigger.track_focus(&self.focus)
        } else {
            trigger
        };
        FluentRoot::new().child(div().size_full().p(px(24.)).child(trigger))
    }
}

/// The bindings an application installs: arrow keys and Enter inside a menu, and
/// Escape for both the menu and the page underneath it.
fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("down", crate::MenuNext, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("enter", crate::MenuActivate, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("escape", crate::MenuEscape, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("escape", crate::Dismiss, Some(crate::OVERLAY_KEY_CONTEXT)),
        ]);
    });
}

fn open(cx: &mut TestAppContext, kind: Kind) -> WindowHandle<FamilyView> {
    open_with(cx, kind, true)
}

fn open_with(cx: &mut TestAppContext, kind: Kind, tracks_focus: bool) -> WindowHandle<FamilyView> {
    bind_keys(cx);
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let handle = cx.open_window(size(px(600.), px(360.)), |_, cx| FamilyView {
        kind,
        focus: cx.focus_handle().tab_stop(true),
        tracks_focus,
        log,
    });
    cx.run_until_parked();
    settle(cx, handle);
    handle
}

/// Draws twice so a state change from the previous frame is measured.
fn draw(cx: &mut TestAppContext, handle: WindowHandle<FamilyView>) {
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

/// Draws with the reveal already finished, so the panel is at its final size.
fn settle(cx: &mut TestAppContext, handle: WindowHandle<FamilyView>) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    draw(cx, handle);
}

/// The painted menu surface in logical pixels, when there is one.
fn panel(cx: &mut TestAppContext, handle: WindowHandle<FamilyView>) -> Option<Bounds<Pixels>> {
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

/// Clicks the trigger button, which sits at the window's 24px inset.
fn click_trigger(cx: &mut TestAppContext, handle: WindowHandle<FamilyView>) {
    // The trigger sits at the window's 24px inset and is a little wider than the
    // word on it, so a press near its top-left corner is inside the button.
    let position = point(px(40.), px(40.));
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
    draw(cx, handle);
}

fn press(cx: &mut TestAppContext, handle: WindowHandle<FamilyView>, key: &'static str) {
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
        // A button reports its activation when the key comes back up, so a real
        // press has to release too.
        window.dispatch_event(PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }), cx);
    })
    .unwrap();
    cx.run_until_parked();
    draw(cx, handle);
}

#[gpui::test]
fn clicking_the_trigger_opens_its_menu(cx: &mut TestAppContext) {
    let handle = open(cx, Kind::Single);
    assert!(
        panel(cx, handle).is_none(),
        "the menu must stay closed until the trigger is used"
    );

    click_trigger(cx, handle);
    settle(cx, handle);
    assert!(
        panel(cx, handle).is_some(),
        "a click on the trigger must open its menu"
    );
}

#[gpui::test]
fn the_trigger_closes_on_escape_and_reopens_from_the_keyboard(cx: &mut TestAppContext) {
    let handle = open(cx, Kind::Single);
    click_trigger(cx, handle);
    settle(cx, handle);
    assert!(panel(cx, handle).is_some(), "the trigger must open a menu");

    press(cx, handle, "escape");
    assert!(
        panel(cx, handle).is_none(),
        "Escape must close the menu the trigger opened"
    );

    // The menu took the keyboard while it was open; after closing, the trigger
    // is the control the user is on, so Enter opens it again.
    press(cx, handle, "enter");
    settle(cx, handle);
    assert!(
        panel(cx, handle).is_some(),
        "Enter on the focused trigger must reopen its menu"
    );
}

#[gpui::test]
fn a_two_line_row_makes_the_menu_taller(cx: &mut TestAppContext) {
    let single = open(cx, Kind::Single);
    click_trigger(cx, single);
    settle(cx, single);
    let single_panel = panel(cx, single).expect("the single-line menu must paint");

    let two_line = open(cx, Kind::TwoLine);
    click_trigger(cx, two_line);
    settle(cx, two_line);
    let two_line_panel = panel(cx, two_line).expect("the two-line menu must paint");

    // Three rows, each one 16px taller: the secondary line is a second line, not
    // text squeezed into the single-line row.
    let extra = two_line_panel.size.height - single_panel.size.height;
    assert!(
        (extra - px(48.)).abs() <= px(1.5),
        "three two-line rows must add 48px, got {extra} ({} vs {})",
        single_panel.size.height,
        two_line_panel.size.height
    );
}

#[gpui::test]
fn the_keyboard_walks_a_group_from_its_first_row(cx: &mut TestAppContext) {
    let handle = open(cx, Kind::Group);
    click_trigger(cx, handle);
    settle(cx, handle);

    // The heading is not a row: the first Down lands on the group's first item,
    // not on the heading and not on a row below it.
    press(cx, handle, "down");
    press(cx, handle, "enter");

    cx.update_window(handle.into(), |view, _, cx| {
        let view = view.downcast::<FamilyView>().unwrap();
        assert_eq!(
            view.read(cx).log.borrow().as_slice(),
            ["One"],
            "the first keyboard stop in a group must be its first row"
        );
    })
    .unwrap();
}

#[gpui::test]
fn a_checkable_row_runs_its_handler_from_the_keyboard(cx: &mut TestAppContext) {
    let handle = open(cx, Kind::Checkbox);
    click_trigger(cx, handle);
    settle(cx, handle);

    press(cx, handle, "down");
    press(cx, handle, "enter");

    cx.update_window(handle.into(), |view, _, cx| {
        let view = view.downcast::<FamilyView>().unwrap();
        assert_eq!(
            view.read(cx).log.borrow().as_slice(),
            ["check"],
            "a checkable row must activate like any other row"
        );
    })
    .unwrap();
}

#[gpui::test]
fn a_click_hands_the_keyboard_to_the_menu(cx: &mut TestAppContext) {
    let handle = open(cx, Kind::Single);
    click_trigger(cx, handle);
    settle(cx, handle);

    // A press leaves the focus on the button unless the trigger moves it. The
    // menu's arrows only work once the menu holds the keyboard, so a click that
    // never handed it over leaves this Down dead and the log empty.
    press(cx, handle, "down");
    press(cx, handle, "enter");

    cx.update_window(handle.into(), |view, _, cx| {
        let view = view.downcast::<FamilyView>().unwrap();
        assert_eq!(
            view.read(cx).log.borrow().as_slice(),
            ["One"],
            "a click must leave the keyboard on the menu it opened"
        );
    })
    .unwrap();
}

#[gpui::test]
fn a_trigger_without_a_menu_focus_still_takes_the_keyboard(cx: &mut TestAppContext) {
    // Most callers just drop a trigger on the page. Nothing hands the menu a
    // focus handle, so the only thing that can give it the keyboard after a
    // click is the trigger itself.
    let handle = open_with(cx, Kind::Single, false);
    click_trigger(cx, handle);
    settle(cx, handle);

    press(cx, handle, "down");
    press(cx, handle, "enter");

    cx.update_window(handle.into(), |view, _, cx| {
        let view = view.downcast::<FamilyView>().unwrap();
        assert_eq!(
            view.read(cx).log.borrow().as_slice(),
            ["One"],
            "a trigger with no menu focus handle must still leave the keyboard on its menu"
        );
    })
    .unwrap();
}
