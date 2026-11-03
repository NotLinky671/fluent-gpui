//! Typed-prefix navigation and closing the menu with Tab.
//!
//! Both drive real key events at a window: typeahead is a key listener on the
//! panel (arbitrary characters cannot be bindings), and Tab resolves through the
//! menu's own key context.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AnyWindowHandle, AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke,
    PlatformInput, Render, TestAppContext, Window, size,
};

type Log = std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>;
type Changes = std::rc::Rc<std::cell::RefCell<Vec<bool>>>;
type FocusSlot = std::rc::Rc<std::cell::RefCell<Option<FocusHandle>>>;

const ROWS: [&str; 3] = ["One", "Two", "Three"];

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("down", crate::MenuNext, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("enter", crate::MenuActivate, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("tab", crate::MenuTabOut, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new(
                "shift-tab",
                crate::MenuTabOut,
                Some(crate::MENU_KEY_CONTEXT),
            ),
        ]);
    });
}

fn settle(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    for _ in 0..2 {
        cx.update_window(handle, |_, window, cx| {
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

// ---------------------------------------------------------------- bare menu

struct BareView {
    focus: FocusHandle,
    log: Log,
}

impl Render for BareView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut menu = Menu::new("typeahead-menu").track_focus(&self.focus);
        for label in ROWS {
            let log = self.log.clone();
            menu = menu.item(
                MenuItem::new(label, label.to_string())
                    .on_click(move |_, _| log.borrow_mut().push(label)),
            );
        }
        FluentRoot::new().child(div().size_full().child(menu))
    }
}

fn open_bare(cx: &mut TestAppContext) -> (AnyWindowHandle, Log) {
    bind_keys(cx);
    let log: Log = Default::default();
    let log_for_view = log.clone();
    let slot: FocusSlot = Default::default();
    let slot_for_view = slot.clone();
    let handle = cx.open_window(size(px(600.), px(500.)), move |_, cx| {
        let focus = cx.focus_handle().tab_stop(true);
        slot_for_view.replace(Some(focus.clone()));
        BareView {
            focus,
            log: log_for_view,
        }
    });
    cx.run_until_parked();
    let focus = slot.borrow().clone().expect("view built");
    let any: AnyWindowHandle = handle.into();
    focus_and_settle(cx, any, &focus);
    (any, log)
}

#[gpui::test]
fn a_letter_jumps_to_the_next_row_starting_with_it(cx: &mut TestAppContext) {
    let (handle, log) = open_bare(cx);
    press(cx, handle, "t");
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["Two"],
        "typing t must select the first row starting with t"
    );
}

#[gpui::test]
fn repeating_a_letter_cycles_through_matches(cx: &mut TestAppContext) {
    let (handle, log) = open_bare(cx);
    press(cx, handle, "t");
    press(cx, handle, "t");
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["Three"],
        "a second t must move to the next row starting with t, not build \"tt\""
    );
}

#[gpui::test]
fn a_multi_character_prefix_narrows_the_match(cx: &mut TestAppContext) {
    let (handle, log) = open_bare(cx);
    press(cx, handle, "t");
    press(cx, handle, "h");
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["Three"],
        "th must match Three even though t alone matched Two"
    );
}

#[gpui::test]
fn an_unmatched_letter_leaves_no_current_row(cx: &mut TestAppContext) {
    let (handle, log) = open_bare(cx);
    press(cx, handle, "z");
    press(cx, handle, "enter");
    assert!(
        log.borrow().is_empty(),
        "a prefix matching nothing must not activate anything, got {:?}",
        *log.borrow()
    );
}

#[gpui::test]
fn typeahead_after_arrows_continues_from_the_current_row(cx: &mut TestAppContext) {
    let (handle, log) = open_bare(cx);
    press(cx, handle, "down");
    press(cx, handle, "t");
    press(cx, handle, "enter");
    assert_eq!(*log.borrow(), vec!["Two"]);
}

// -------------------------------------------------------------- dropdown

struct DropdownView {
    focus: FocusHandle,
    changes: Changes,
}

impl Render for DropdownView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let changes = self.changes.clone();
        let mut menu = Menu::new("tab-menu-panel");
        for label in ROWS {
            menu = menu.item(MenuItem::new(label, label.to_string()));
        }
        FluentRoot::new().child(
            div().size_full().child(
                MenuDropdown::new(
                    "tab-menu",
                    div().id("tab-trigger").w(px(120.)).h(px(28.)).child("open"),
                    menu,
                )
                .default_open(true)
                .track_focus(&self.focus)
                .on_open_change(move |next, _, _| changes.borrow_mut().push(next)),
            ),
        )
    }
}

fn open_dropdown(cx: &mut TestAppContext) -> (AnyWindowHandle, Changes) {
    bind_keys(cx);
    let changes: Changes = Default::default();
    let for_view = changes.clone();
    let slot: FocusSlot = Default::default();
    let slot_for_view = slot.clone();
    let handle = cx.open_window(size(px(600.), px(500.)), move |_, cx| {
        let focus = cx.focus_handle().tab_stop(true);
        slot_for_view.replace(Some(focus.clone()));
        DropdownView {
            focus,
            changes: for_view,
        }
    });
    cx.run_until_parked();
    let focus = slot.borrow().clone().expect("view built");
    let any: AnyWindowHandle = handle.into();
    // The popover moves focus into the panel on open; focusing again is harmless
    // and makes the precondition explicit.
    focus_and_settle(cx, any, &focus);
    (any, changes)
}

fn row_fills(cx: &mut TestAppContext, handle: AnyWindowHandle) -> usize {
    let mut count = 0;
    cx.update_window(handle, |_, window, cx| {
        let scale = window.scale_factor();
        let theme = FluentTheme::global(cx);
        let spec = menu::item(&theme, &MenuItemProps::default());
        count = window
            .painted_quads()
            .into_iter()
            .filter(|quad| {
                quad.bounds.size.height.0 == f32::from(spec.height) * scale
                    && quad.bounds.size.width.0 >= 100. * scale
            })
            .count();
    })
    .unwrap();
    count
}

#[gpui::test]
fn tab_closes_the_menu(cx: &mut TestAppContext) {
    let (handle, changes) = open_dropdown(cx);
    press(cx, handle, "down");
    assert_eq!(row_fills(cx, handle), 1, "precondition: a row is current");

    press(cx, handle, "tab");
    assert_eq!(
        *changes.borrow(),
        vec![false],
        "Tab must ask the host to close the menu"
    );
    assert_eq!(
        row_fills(cx, handle),
        0,
        "the panel must be gone once Tab closed the menu"
    );
}

#[gpui::test]
fn shift_tab_also_closes_the_menu(cx: &mut TestAppContext) {
    let (handle, changes) = open_dropdown(cx);
    press(cx, handle, "shift-tab");
    assert_eq!(*changes.borrow(), vec![false]);
    assert_eq!(row_fills(cx, handle), 0);
}
