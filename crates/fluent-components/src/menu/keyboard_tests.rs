//! Keyboard navigation of a menu.
//!
//! The active row is visual state, so these tests assert the end-to-end result
//! instead: which row's handler ran. That covers the whole path — key dispatch,
//! the menu's key context, the active index and activation — and would not pass
//! if any link were broken.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, PlatformInput, Render,
    TestAppContext, Window, WindowHandle, size,
};

/// Shared list of the rows that were activated, in order.
type Log = std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>;

struct MenuView {
    focus: FocusHandle,
    log: Log,
    /// Disabled row index, or `None` to make every row enabled.
    disabled: Option<usize>,
}

impl MenuView {
    fn item(&self, id: &'static str, label: &'static str, index: usize) -> MenuItem {
        let log = self.log.clone();
        let item = MenuItem::new(id, label).on_click(move |_, _| log.borrow_mut().push(label));
        if self.disabled == Some(index) {
            item.disabled(true)
        } else {
            item
        }
    }
}

impl Render for MenuView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let menu = Menu::new("kbd-menu")
            .item(self.item("row-one", "One", 0))
            .item(self.item("row-two", "Two", 1))
            .item(self.item("row-three", "Three", 2));

        FluentRoot::new().child(
            div().size_full().p(px(24.)).child(
                MenuDropdown::new(
                    "kbd-dropdown",
                    div().id("kbd-trigger").w(px(120.)).h(px(28.)).child("open"),
                    menu,
                )
                .default_open(true)
                .track_focus(&self.focus),
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
            KeyBinding::new("home", crate::MenuFirst, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("end", crate::MenuLast, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("enter", crate::MenuActivate, Some(crate::MENU_KEY_CONTEXT)),
        ]);
    });
}

fn open(cx: &mut TestAppContext, disabled: Option<usize>) -> (WindowHandle<MenuView>, Log) {
    bind_keys(cx);
    let log: Log = Default::default();
    let log_for_view = log.clone();
    let window = cx.open_window(size(px(600.), px(500.)), move |_, cx| MenuView {
        focus: cx.focus_handle().tab_stop(true),
        log: log_for_view,
        disabled,
    });
    cx.run_until_parked();
    (window, log)
}

/// Focus the menu and settle tab stops, then dispatch `keys` in order.
fn press(cx: &mut TestAppContext, handle: WindowHandle<MenuView>, keys: &[&'static str]) {
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<MenuView>().unwrap();
        let focus = view.read(cx).focus.clone();
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

    for key in keys {
        // Parsed outside the closure: the listener must be 'static.
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
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

#[gpui::test]
fn down_then_enter_activates_the_first_row(cx: &mut TestAppContext) {
    let (handle, log) = open(cx, None);
    press(cx, handle, &["down", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["One"],
        "the first Down selects the first row and Enter activates it"
    );
}

#[gpui::test]
fn repeated_down_moves_through_the_rows(cx: &mut TestAppContext) {
    let (handle, log) = open(cx, None);
    press(cx, handle, &["down", "down", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Two"],
        "the second Down reaches row two"
    );
}

#[gpui::test]
fn down_past_the_last_row_wraps_to_the_first(cx: &mut TestAppContext) {
    let (handle, log) = open(cx, None);
    // Nothing active, so the first three Downs land on rows one, two and three;
    // the fourth is the one that has to wrap.
    press(cx, handle, &["down", "down", "down", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Three"],
        "three Downs reach the last row"
    );

    let (handle, log) = open(cx, None);
    press(cx, handle, &["down", "down", "down", "down", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["One"],
        "Down on the last row wraps back to the first"
    );
}

#[gpui::test]
fn up_from_the_first_row_wraps_to_the_last(cx: &mut TestAppContext) {
    let (handle, log) = open(cx, None);
    press(cx, handle, &["up", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Three"],
        "Up with nothing active lands on the last row"
    );
}

#[gpui::test]
fn end_and_home_jump_to_the_edges(cx: &mut TestAppContext) {
    let (handle, log) = open(cx, None);
    press(cx, handle, &["end", "enter"]);
    assert_eq!(*log.borrow(), vec!["Three"], "End selects the last row");

    let (handle, log) = open(cx, None);
    press(cx, handle, &["end", "home", "enter"]);
    assert_eq!(*log.borrow(), vec!["One"], "Home returns to the first row");
}

#[gpui::test]
fn disabled_rows_are_skipped(cx: &mut TestAppContext) {
    // Row two is disabled, so two Downs must reach row three, not row two.
    let (handle, log) = open(cx, Some(1));
    press(cx, handle, &["down", "down", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Three"],
        "a disabled row is never selected by the keyboard"
    );
}
