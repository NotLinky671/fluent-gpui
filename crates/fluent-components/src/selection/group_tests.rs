use super::*;
use crate::FluentRoot;
use gpui::{
    AppContext, Context, Entity, FocusHandle, InputEvent as _, KeyBinding, KeyDownEvent, Keystroke,
    Modifiers, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput, Render, TestAppContext,
    WindowHandle, point, size,
};
use std::{cell::RefCell, rc::Rc};

struct View {
    controlled: bool,
    default: &'static str,
    requests: Rc<RefCell<Vec<ElementId>>>,
    before: FocusHandle,
    after: FocusHandle,
}
impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let requests = self.requests.clone();
        let mut group = RadioGroup::new("group")
            .accessibility_label("Choice")
            .default_value(self.default)
            .radio(Radio::new("a").label("Alpha").w(px(160.)).h(px(28.)))
            .radio(
                Radio::new("b")
                    .label("Beta")
                    .disabled(true)
                    .w(px(160.))
                    .h(px(28.)),
            )
            .radio(Radio::new("c").label("Gamma").w(px(160.)).h(px(28.)))
            .on_change(move |value, _, _| requests.borrow_mut().push(value.clone()));
        if self.controlled {
            group = group.value(Some("a".into()));
        }
        FluentRoot::new().child(
            div()
                .flex()
                .flex_col()
                .p(px(20.))
                .child(div().id("before").track_focus(&self.before).h(px(20.)))
                .child(group)
                .child(div().id("after").track_focus(&self.after).h(px(20.))),
        )
    }
}
fn open(cx: &mut TestAppContext, controlled: bool) -> (WindowHandle<View>, Entity<View>) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("tab", crate::FocusNext, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", crate::FocusPrev, Some(crate::ROOT_KEY_CONTEXT)),
        ])
    });
    let h = cx.open_window(size(px(400.), px(300.)), |_, cx| View {
        controlled,
        default: "a",
        requests: Default::default(),
        before: cx.focus_handle().tab_stop(true),
        after: cx.focus_handle().tab_stop(true),
    });
    cx.enable_accessibility();
    cx.run_until_parked();
    let view = cx
        .update_window(h.into(), |root, _, _| root.downcast::<View>().unwrap())
        .unwrap();
    (h, view)
}
fn draw(cx: &mut TestAppContext, h: WindowHandle<View>) -> serde_json::Value {
    let text = cx
        .update_window(h.into(), |_, w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
            w.debug_a11y_tree_json().unwrap()
        })
        .unwrap();
    serde_json::from_str(&text).unwrap()
}
fn selected(tree: &serde_json::Value) -> Vec<String> {
    tree["nodes"]
        .as_object()
        .unwrap()
        .values()
        .filter(|n| n["aria"]["role"] == "RadioButton" && n["aria"]["selected"] == true)
        .map(|n| n["aria"]["label"].as_str().unwrap().to_string())
        .collect()
}
fn click_gamma(cx: &mut TestAppContext, h: WindowHandle<View>) {
    let position = point(px(30.), px(126.)); // p20 + before20 + 2*(radio28+gap8) + half14
    cx.update_window(h.into(), |_, w, cx| {
        w.dispatch_event(
            MouseMoveEvent {
                position,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
            cx,
        );
        w.dispatch_event(
            MouseDownEvent {
                position,
                button: gpui::MouseButton::Left,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        w.dispatch_event(
            MouseUpEvent {
                position,
                button: gpui::MouseButton::Left,
                modifiers: Modifiers::default(),
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
}
fn key(cx: &mut TestAppContext, h: WindowHandle<View>, key: &'static str) {
    cx.update_window(h.into(), |_, w, cx| {
        w.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse(key).unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
}
#[gpui::test]
fn pointer_selection_is_exclusive_and_survives_a_new_default(cx: &mut TestAppContext) {
    let (h, view) = open(cx, false);
    assert_eq!(selected(&draw(cx, h)), vec!["Alpha"]);
    click_gamma(cx, h);
    assert_eq!(selected(&draw(cx, h)), vec!["Gamma"]);
    view.update(cx, |v, cx| {
        v.default = "b";
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(selected(&draw(cx, h)), vec!["Gamma"]);
    assert_eq!(
        view.read_with(cx, |v, _| v.requests.borrow().clone()),
        vec![ElementId::from("c")]
    );
}
#[gpui::test]
fn ignored_controlled_request_does_not_take_over_selection(cx: &mut TestAppContext) {
    let (h, view) = open(cx, true);
    click_gamma(cx, h);
    assert_eq!(selected(&draw(cx, h)), vec!["Alpha"]);
    assert_eq!(
        view.read_with(cx, |v, _| v.requests.borrow().clone()),
        vec![ElementId::from("c")]
    );
}
#[gpui::test]
fn arrows_skip_disabled_wrap_and_tab_leaves_the_group(cx: &mut TestAppContext) {
    let (h, view) = open(cx, false);
    cx.update_window(h.into(), |_, w, cx| {
        let focus = view.read(cx).before.clone();
        focus.focus(w, cx);
    })
    .unwrap();
    cx.run_until_parked();
    key(cx, h, "tab");
    key(cx, h, "down");
    assert_eq!(selected(&draw(cx, h)), vec!["Gamma"]);
    key(cx, h, "down");
    assert_eq!(selected(&draw(cx, h)), vec!["Alpha"]);
    key(cx, h, "tab");
    assert!(
        cx.update_window(h.into(), |_, w, cx| view.read(cx).after.is_focused(w))
            .unwrap()
    );
    let tree = draw(cx, h);
    assert!(
        tree["nodes"]
            .as_object()
            .unwrap()
            .values()
            .any(|n| n["aria"]["role"] == "RadioGroup" && n["aria"]["label"] == "Choice")
    );
}
