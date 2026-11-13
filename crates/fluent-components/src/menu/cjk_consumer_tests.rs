//! End-to-end CJK input through the real menu container and platform input
//! handler. Composition never moves the menu; the committed Chinese text goes
//! through the shared typeahead path, and Enter activates the matched row only
//! after the commit.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, PlatformInput, Render,
    TestAppContext, Window, WindowHandle, size,
};
use std::cell::RefCell;
use std::rc::Rc;

type Log = Rc<RefCell<Vec<&'static str>>>;

struct CjkMenuView {
    focus: FocusHandle,
    log: Log,
}

impl Render for CjkMenuView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let delete_log = self.log.clone();
        let copy_log = self.log.clone();
        let menu = Menu::new("cjk-menu-panel")
            .accessibility_label("Actions")
            .item(
                MenuItem::new("cjk-delete", "删除")
                    .on_click(move |_, _| delete_log.borrow_mut().push("delete")),
            )
            .item(
                MenuItem::new("cjk-copy", "复制")
                    .on_click(move |_, _| copy_log.borrow_mut().push("copy")),
            );
        FluentRoot::new().child(
            div().size_full().p(px(24.)).child(
                MenuDropdown::new(
                    "cjk-dropdown",
                    div()
                        .id("cjk-trigger")
                        .w(px(120.))
                        .h(px(28.))
                        .child("Actions"),
                    menu,
                )
                .default_open(true)
                .track_focus(&self.focus),
            ),
        )
    }
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<CjkMenuView>, Log) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("down", crate::MenuNext, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("enter", crate::MenuActivate, Some(crate::MENU_KEY_CONTEXT)),
        ]);
    });
    let log: Log = Default::default();
    let log_for_view = log.clone();
    let window = cx.open_window(size(px(600.), px(500.)), move |_, cx| CjkMenuView {
        focus: cx.focus_handle().tab_stop(true),
        log: log_for_view,
    });
    cx.run_until_parked();
    cx.enable_accessibility();
    focus_and_draw(cx, window);
    (window, log)
}

fn focus_and_draw(cx: &mut TestAppContext, handle: WindowHandle<CjkMenuView>) {
    cx.update_window(handle.into(), |view, window, cx| {
        let focus = view
            .downcast::<CjkMenuView>()
            .unwrap()
            .read(cx)
            .focus
            .clone();
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

fn press_enter(cx: &mut TestAppContext, handle: WindowHandle<CjkMenuView>) {
    let keystroke = Keystroke::parse("enter").unwrap();
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

fn tree(cx: &mut TestAppContext, handle: WindowHandle<CjkMenuView>) -> serde_json::Value {
    let json = cx
        .update_window(handle.into(), |_, window, _| window.debug_a11y_tree_json())
        .unwrap()
        .expect("accessibility tree exists");
    serde_json::from_str(&json).expect("valid JSON")
}

fn key_of(tree: &serde_json::Value, label: &str) -> String {
    tree["nodes"]
        .as_object()
        .expect("nodes object")
        .iter()
        .find(|(_, node)| node["aria"]["label"].as_str() == Some(label))
        .map(|(key, _)| key.clone())
        .unwrap_or_else(|| panic!("no node labelled {label:?}: {tree}"))
}

#[gpui::test]
fn composition_does_not_activate_and_committed_chinese_matches_the_row(cx: &mut TestAppContext) {
    let (window, log) = open(cx);

    // Platform IME path: marked pinyin reaches the real EntityInputHandler sunk
    // into the focused menu container.
    cx.replace_and_mark_text(window.into(), "shan");
    press_enter(cx, window);
    assert!(
        log.borrow().is_empty(),
        "Enter during marked composition must not activate a row: {:?}",
        log.borrow()
    );
    let composing = tree(cx, window);
    assert!(
        composing["active_descendant_focus"].is_null(),
        "marked composition must not move the menu's current row: {composing}"
    );

    // Committing the Chinese character goes through the same typeahead path as
    // keyboard input and selects 删除.
    cx.commit_text(window.into(), "删");
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
    let committed = tree(cx, window);
    let delete = key_of(&committed, "删除");
    assert_eq!(
        committed["active_descendant_focus"].as_str(),
        Some(delete.as_str()),
        "committed 删 must make the 删除 row current: {committed}"
    );
    assert_eq!(
        committed["nodes"][&delete]["aria"]["selected"].as_bool(),
        Some(true),
        "the matched row must be selected: {committed}"
    );

    press_enter(cx, window);
    assert_eq!(
        log.borrow().as_slice(),
        ["delete"],
        "a later Enter activates the matched 删除 row exactly once"
    );
}
