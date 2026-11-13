//! Real accessibility tree for `Textarea`.
//!
//! The component must expose the multiline role even without a name builder,
//! and a builder name plus the live value when it is active.

use super::*;
use crate::root::FluentRoot;
use gpui::{AppContext, Context, Entity, Render, TestAppContext, Window, WindowHandle, size};
use gpui_base::input::TextareaState;

struct AxView {
    state: Entity<TextareaState>,
    label: Option<&'static str>,
}

impl Render for AxView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut field = Textarea::new(&self.state);
        if let Some(label) = self.label {
            field = field.accessibility_label(label);
        }
        FluentRoot::new().child(div().p(px(20.)).child(field))
    }
}

fn open(cx: &mut TestAppContext, label: Option<&'static str>) -> WindowHandle<AxView> {
    cx.update(fluent_core::gpui_compat::init_base);
    let window = cx.open_window(size(px(400.), px(240.)), move |window, cx| {
        let state = cx.new(|cx| TextareaState::new(window, cx).default_value("hello"));
        AxView { state, label }
    });
    cx.enable_accessibility();
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    window
}

fn tree(cx: &mut TestAppContext, window: WindowHandle<AxView>) -> String {
    let mut json = None;
    cx.update_window(window.into(), |_, window, _| {
        json = window.debug_a11y_tree_json();
    })
    .unwrap();
    json.expect("accessibility tree was built")
}

#[gpui::test]
fn the_multiline_role_exists_without_a_label(cx: &mut TestAppContext) {
    let window = open(cx, None);
    let json = tree(cx, window);
    assert!(
        json.contains("MultilineTextInput"),
        "textarea must expose its role without a name builder: {json}"
    );
}

#[gpui::test]
fn the_builder_name_and_value_reach_the_tree(cx: &mut TestAppContext) {
    let window = open(cx, Some("Body"));
    let json = tree(cx, window);
    assert!(json.contains("MultilineTextInput"), "role missing: {json}");
    assert!(json.contains("Body"), "name missing: {json}");
    assert!(json.contains("hello"), "value missing: {json}");
}
