//! Tab contract for `Textarea`.
//!
//! gpui-base binds Tab/Shift+Tab in its deeper `Input` context to
//! IndentInline/OutdentInline. A Fluent control that wants navigation opts in
//! with a marker context; the Fluent binding names both contexts, ties at the
//! `Input` depth and wins because it is installed later.

use super::*;
use crate::root::FluentRoot;
use crate::{FocusNext, FocusPrev, TEXT_INPUT_NAVIGATE_BINDING_CONTEXT, TextareaTabBehavior};
use gpui::{
    AnyWindowHandle, AppContext, Context, Entity, FocusHandle, KeyBinding, KeyDownEvent, Keystroke,
    PlatformInput, Render, TestAppContext, Window, WindowHandle, size,
};
use gpui_base::input::TextareaState;

struct TabView {
    state: Entity<TextareaState>,
    before: FocusHandle,
    after: FocusHandle,
    behavior: TextareaTabBehavior,
}

impl Render for TabView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .p(px(20.))
                .child(control("textarea-before", &self.before))
                .child(
                    div()
                        .w(px(240.))
                        .child(Textarea::new(&self.state).tab_behavior(self.behavior)),
                )
                .child(control("textarea-after", &self.after)),
        )
    }
}

fn control(id: &'static str, focus: &FocusHandle) -> impl IntoElement {
    div().id(id).track_focus(focus).h(px(28.)).child(id)
}

#[derive(Clone)]
struct Handles {
    state: Entity<TextareaState>,
    before: FocusHandle,
    after: FocusHandle,
}

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("tab", FocusNext, Some(TEXT_INPUT_NAVIGATE_BINDING_CONTEXT)),
            KeyBinding::new(
                "shift-tab",
                FocusPrev,
                Some(TEXT_INPUT_NAVIGATE_BINDING_CONTEXT),
            ),
            KeyBinding::new("tab", FocusNext, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", FocusPrev, Some(crate::ROOT_KEY_CONTEXT)),
        ]);
    });
}

fn open_view(
    cx: &mut TestAppContext,
    behavior: TextareaTabBehavior,
) -> (WindowHandle<TabView>, Handles) {
    bind_keys(cx);
    let slot: std::rc::Rc<std::cell::RefCell<Option<Handles>>> = Default::default();
    let for_view = slot.clone();
    let window = cx.open_window(size(px(600.), px(500.)), move |window, cx| {
        let state = cx.new(|cx| TextareaState::new(window, cx));
        let handles = Handles {
            state: state.clone(),
            before: cx.focus_handle().tab_stop(true),
            after: cx.focus_handle().tab_stop(true),
        };
        for_view.replace(Some(handles.clone()));
        TabView {
            state,
            before: handles.before.clone(),
            after: handles.after.clone(),
            behavior,
        }
    });
    cx.run_until_parked();
    let handles = slot.borrow().clone().expect("view built");
    (window, handles)
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

fn focus_textarea(cx: &mut TestAppContext, handle: AnyWindowHandle, h: &Handles) {
    for _ in 0..2 {
        cx.update_window(handle, |_, window, cx| {
            h.state
                .read(cx)
                .presentation()
                .focus_handle()
                .focus(window, cx);
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

fn focused(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Option<FocusHandle> {
    let mut out = None;
    cx.update_window(handle, |_, window, cx| out = window.focused(cx))
        .unwrap();
    out
}

fn value(cx: &mut TestAppContext, h: &Handles) -> String {
    cx.update(|cx| h.state.read(cx).value().to_string())
}

#[gpui::test]
fn navigate_moves_forward_without_editing(cx: &mut TestAppContext) {
    let (window, h) = open_view(cx, TextareaTabBehavior::Navigate);
    let handle: AnyWindowHandle = window.into();
    focus_textarea(cx, handle, &h);
    cx.update_window(handle, |_, window, cx| {
        h.state
            .update(cx, |state, cx| state.set_value("body", window, cx));
    })
    .unwrap();
    settle(cx, handle);
    let before = value(cx, &h);

    press(cx, handle, "tab");
    assert_eq!(
        focused(cx, handle),
        Some(h.after.clone()),
        "Navigate must move to the next control"
    );
    assert_eq!(value(cx, &h), before, "Tab must not edit the body");
}

#[gpui::test]
fn navigate_moves_backward_without_editing(cx: &mut TestAppContext) {
    let (window, h) = open_view(cx, TextareaTabBehavior::Navigate);
    let handle: AnyWindowHandle = window.into();
    focus_textarea(cx, handle, &h);
    cx.update_window(handle, |_, window, cx| {
        h.state
            .update(cx, |state, cx| state.set_value("body", window, cx));
    })
    .unwrap();
    settle(cx, handle);
    let before = value(cx, &h);

    press(cx, handle, "shift-tab");
    assert_eq!(
        focused(cx, handle),
        Some(h.before.clone()),
        "Shift+Tab must move to the previous control"
    );
    assert_eq!(value(cx, &h), before, "Shift+Tab must not edit the body");
}

#[gpui::test]
fn the_default_indent_behavior_still_consumes_tab(cx: &mut TestAppContext) {
    let (window, h) = open_view(cx, TextareaTabBehavior::Indent);
    let handle: AnyWindowHandle = window.into();
    focus_textarea(cx, handle, &h);

    let textarea_focus = cx.update(|cx| h.state.read(cx).presentation().focus_handle().clone());
    press(cx, handle, "tab");
    assert_eq!(
        focused(cx, handle),
        Some(textarea_focus),
        "the default must keep gpui-base's indent behavior and focus"
    );
}
