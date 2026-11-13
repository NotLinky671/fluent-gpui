//! Accessibility-tree behaviour of `TextInput`.
//!
//! These tests render the real element in a test window, activate the platform
//! accessibility callbacks, and read the tree that the window actually
//! publishes through `debug_a11y_tree_json`. They do not inspect builder fields
//! or the component's own getters.

use super::*;
use crate::root::FluentRoot;
use gpui::{AppContext, Context, Render, TestAppContext, Window, WindowHandle, size};
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

struct FieldView {
    state: Entity<InputState>,
    label: Option<SharedString>,
}

impl Render for FieldView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut field = TextInput::new(&self.state);
        if let Some(label) = self.label.clone() {
            field = field.accessibility_label(label);
        }
        FluentRoot::new().child(field)
    }
}

fn open(
    cx: &mut TestAppContext,
    preset: Option<InputAccessibility>,
    label: Option<&'static str>,
) -> (WindowHandle<FieldView>, Entity<InputState>) {
    cx.update(fluent_core::gpui_compat::init_base);
    let slot: Rc<RefCell<Option<Entity<InputState>>>> = Rc::new(RefCell::new(None));
    let slot_for_view = slot.clone();
    let window = cx.open_window(size(px(400.), px(200.)), move |window, cx| {
        let state = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            if let Some(preset) = preset {
                state.set_accessibility(preset);
            }
            state
        });
        slot_for_view.borrow_mut().replace(state.clone());
        FieldView {
            state,
            label: label.map(SharedString::from),
        }
    });
    cx.run_until_parked();
    let state = slot.borrow().clone().expect("the view built its state");
    (window, state)
}

fn draw_and_read_tree(cx: &mut TestAppContext, window: WindowHandle<FieldView>) -> Value {
    cx.update_window(window.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
    let json = cx
        .update_window(window.into(), |_, window, _| window.debug_a11y_tree_json())
        .unwrap()
        .expect("the activated window must publish an accessibility tree");
    serde_json::from_str(&json).expect("the debug tree is valid JSON")
}

fn text_input_nodes(tree: &Value) -> Vec<&Value> {
    tree["nodes"]
        .as_object()
        .expect("nodes object")
        .values()
        .filter(|node| node["aria"]["role"].as_str() == Some("TextInput"))
        .collect()
}

#[gpui::test]
fn a_plain_field_still_exposes_the_text_input_role(cx: &mut TestAppContext) {
    let (window, _state) = open(cx, None, None);
    cx.enable_accessibility();
    let tree = draw_and_read_tree(cx, window);

    let nodes = text_input_nodes(&tree);
    assert_eq!(
        nodes.len(),
        1,
        "a plain TextInput must publish exactly one TextInput node, got tree: {tree}"
    );
}

#[gpui::test]
fn explicit_state_accessibility_survives_rendering_without_a_builder(cx: &mut TestAppContext) {
    let preset = InputAccessibility::new(None)
        .with_label(Some(SharedString::from("Caller name")))
        .with_placeholder(Some(SharedString::from("Search notes")))
        .with_value_exposed(false);
    let (window, _state) = open(cx, Some(preset), None);
    cx.enable_accessibility();
    let tree = draw_and_read_tree(cx, window);

    let nodes = text_input_nodes(&tree);
    assert_eq!(nodes.len(), 1, "tree: {tree}");
    let node = &nodes[0]["aria"];
    assert_eq!(
        node["label"].as_str(),
        Some("Caller name"),
        "the state's explicit name must survive a render without a builder"
    );
    assert_eq!(
        node["placeholder"].as_str(),
        Some("Search notes"),
        "the state's placeholder must survive"
    );
    assert!(
        node["value"].is_null(),
        "value_exposed=false must keep the value out of the tree"
    );

    // A second frame must not replace the projection with a default one.
    let again = draw_and_read_tree(cx, window);
    let nodes = text_input_nodes(&again);
    assert_eq!(nodes.len(), 1, "tree: {again}");
    assert_eq!(nodes[0]["aria"]["label"].as_str(), Some("Caller name"));
    assert_eq!(
        nodes[0]["aria"]["placeholder"].as_str(),
        Some("Search notes")
    );
    assert!(nodes[0]["aria"]["value"].is_null());
}

#[gpui::test]
fn builder_name_overrides_only_the_name(cx: &mut TestAppContext) {
    let preset = InputAccessibility::new(Some(Role::TextInput))
        .with_label(Some(SharedString::from("Caller name")))
        .with_placeholder(Some(SharedString::from("Search notes")))
        .with_value_exposed(false);
    let (window, _state) = open(cx, Some(preset), Some("Builder name"));
    cx.enable_accessibility();
    let tree = draw_and_read_tree(cx, window);

    let nodes = text_input_nodes(&tree);
    assert_eq!(nodes.len(), 1, "tree: {tree}");
    let node = &nodes[0]["aria"];
    assert_eq!(
        node["label"].as_str(),
        Some("Builder name"),
        "a builder name defines the accessible name"
    );
    assert_eq!(
        node["placeholder"].as_str(),
        Some("Search notes"),
        "the builder must not replace the caller's placeholder"
    );
    assert!(
        node["value"].is_null(),
        "the builder must not replace value_exposed"
    );
}
