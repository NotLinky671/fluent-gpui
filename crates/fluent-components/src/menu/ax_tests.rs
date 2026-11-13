//! Menu accessibility: the focused container is named, and the current row is
//! the active descendant rather than a separately focused element.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, PlatformInput, Render,
    TestAppContext, Window, WindowHandle, size,
};

struct MenuAxView {
    focus: FocusHandle,
}

impl Render for MenuAxView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let menu = Menu::new("menu-ax-panel")
            .track_focus(&self.focus)
            .accessibility_label("Actions")
            .item(MenuItem::new("menu-ax-del", "Delete"))
            .item(MenuItem::new("menu-ax-copy", "Copy"));
        FluentRoot::new().child(div().p(px(20.)).child(menu))
    }
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<MenuAxView>, FocusHandle) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([KeyBinding::new(
            "down",
            crate::MenuNext,
            Some(crate::MENU_KEY_CONTEXT),
        )]);
    });
    let slot: std::rc::Rc<std::cell::RefCell<Option<FocusHandle>>> = Default::default();
    let for_view = slot.clone();
    let window = cx.open_window(size(px(400.), px(300.)), move |_, cx| {
        let focus = cx.focus_handle().tab_stop(true);
        for_view.replace(Some(focus.clone()));
        MenuAxView { focus }
    });
    cx.enable_accessibility();
    cx.run_until_parked();
    let focus = slot.borrow().clone().expect("focus built");
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            focus.focus(window, cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    (window, focus)
}

fn json(cx: &mut TestAppContext, window: WindowHandle<MenuAxView>) -> String {
    let mut out = None;
    cx.update_window(window.into(), |_, window, _| {
        out = window.debug_a11y_tree_json();
    })
    .unwrap();
    out.expect("accessibility tree exists")
}

fn parse(cx: &mut TestAppContext, window: WindowHandle<MenuAxView>) -> serde_json::Value {
    let raw = json(cx, window);
    serde_json::from_str(&raw).expect("debug tree is valid JSON")
}

fn ephemeral_key(tree: &serde_json::Value, role: &str, label: &str) -> String {
    let nodes = tree["nodes"].as_object().expect("nodes object");
    nodes
        .iter()
        .find(|(_, node)| {
            node["aria"]["role"].as_str() == Some(role)
                && node["aria"]["label"].as_str() == Some(label)
        })
        .map(|(key, _)| key.clone())
        .unwrap_or_else(|| panic!("no {role} node labelled {label:?}: {tree}"))
}

fn press_down(cx: &mut TestAppContext, window: WindowHandle<MenuAxView>) {
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse("down").unwrap(),
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

#[gpui::test]
fn the_menu_is_named_and_the_current_row_is_the_active_descendant(cx: &mut TestAppContext) {
    let (window, _focus) = open(cx);
    press_down(cx, window);
    let tree = parse(cx, window);

    let menu_key = ephemeral_key(&tree, "Menu", "Actions");
    let delete_key = ephemeral_key(&tree, "MenuItem", "Delete");
    assert_eq!(
        tree["gpui_focus"].as_str(),
        Some(menu_key.as_str()),
        "focus must stay on the named Menu container: {tree}"
    );
    assert_eq!(
        tree["active_descendant_focus"].as_str(),
        Some(delete_key.as_str()),
        "the current Delete row must be the active descendant: {tree}"
    );
    assert_eq!(
        tree["nodes"][&delete_key]["aria"]["selected"].as_bool(),
        Some(true),
        "the active row must be selected: {tree}"
    );

    press_down(cx, window);
    let tree = parse(cx, window);
    let menu_key = ephemeral_key(&tree, "Menu", "Actions");
    let delete_key = ephemeral_key(&tree, "MenuItem", "Delete");
    let copy_key = ephemeral_key(&tree, "MenuItem", "Copy");
    assert_eq!(
        tree["gpui_focus"].as_str(),
        Some(menu_key.as_str()),
        "focus still stays on the container after the next Down: {tree}"
    );
    assert_eq!(
        tree["active_descendant_focus"].as_str(),
        Some(copy_key.as_str()),
        "the next Down must move the active descendant to Copy: {tree}"
    );
    assert_eq!(
        tree["nodes"][&delete_key]["aria"]["selected"].as_bool(),
        Some(false),
        "the previous row must no longer be selected: {tree}"
    );
    assert_eq!(
        tree["nodes"][&copy_key]["aria"]["selected"].as_bool(),
        Some(true),
        "the new row must be selected: {tree}"
    );
}
