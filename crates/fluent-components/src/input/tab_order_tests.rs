//! Tab-order behaviour across Fluent fields.
//!
//! `gpui_base::root::init` binds `tab` and `shift-tab`, but scoped to its own
//! `Root` key context. These tests check whether that binding actually reaches
//! controls mounted under [`FluentRoot`], which is what an application uses.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, KeyBinding, KeyDownEvent, Keystroke, PlatformInput, Render,
    TestAppContext, Window, WindowHandle, size,
};

type States = std::rc::Rc<std::cell::RefCell<Option<(Entity<InputState>, Entity<InputState>)>>>;

struct TwoFields {
    first: Entity<InputState>,
    second: Entity<InputState>,
}

impl Render for TwoFields {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .p(px(20.))
                .child(TextInput::new(&self.first))
                .child(TextInput::new(&self.second)),
        )
    }
}

fn open(
    cx: &mut TestAppContext,
) -> (
    WindowHandle<TwoFields>,
    Entity<InputState>,
    Entity<InputState>,
) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("tab", crate::FocusNext, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", crate::FocusPrev, Some(crate::ROOT_KEY_CONTEXT)),
        ]);
    });
    let slot: States = std::rc::Rc::new(std::cell::RefCell::new(None));
    let slot_for_view = slot.clone();
    let window = cx.open_window(size(px(500.), px(400.)), move |window, cx| {
        let first = cx.new(|cx| InputState::new(window, cx).placeholder("First"));
        let second = cx.new(|cx| InputState::new(window, cx).placeholder("Second"));
        slot_for_view.replace(Some((first.clone(), second.clone())));
        TwoFields { first, second }
    });
    cx.run_until_parked();
    let (first, second) = slot.borrow().clone().expect("view built both states");
    (window, first, second)
}

fn press_tab(cx: &mut TestAppContext, handle: WindowHandle<TwoFields>, shift: bool) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse(if shift { "shift-tab" } else { "tab" }).unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
}

fn focus_first(
    cx: &mut TestAppContext,
    handle: WindowHandle<TwoFields>,
    state: &Entity<InputState>,
) {
    cx.update_window(handle.into(), |_, window, cx| {
        let focus = state.read(cx).presentation().focus_handle().clone();
        window.focus(&focus, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn focused_is(
    cx: &mut TestAppContext,
    handle: WindowHandle<TwoFields>,
    state: &Entity<InputState>,
) -> bool {
    let mut focused = false;
    cx.update_window(handle.into(), |_, window, cx| {
        focused = state
            .read(cx)
            .presentation()
            .focus_handle()
            .is_focused(window);
    })
    .unwrap();
    focused
}

#[gpui::test]
fn tab_moves_focus_to_the_next_field(cx: &mut TestAppContext) {
    let (handle, first, second) = open(cx);
    focus_first(cx, handle, &first);
    assert!(
        focused_is(cx, handle, &first),
        "precondition: first focused"
    );

    press_tab(cx, handle, false);
    assert!(
        focused_is(cx, handle, &second),
        "Tab must move focus from the first field to the second; without a Root \
         key context the gpui-base binding never fires"
    );
}

#[gpui::test]
fn shift_tab_moves_focus_backwards(cx: &mut TestAppContext) {
    let (handle, first, second) = open(cx);
    focus_first(cx, handle, &second);
    assert!(
        focused_is(cx, handle, &second),
        "precondition: second focused"
    );

    press_tab(cx, handle, true);
    assert!(
        focused_is(cx, handle, &first),
        "Shift+Tab must move focus backwards"
    );
}
