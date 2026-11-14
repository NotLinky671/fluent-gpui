//! `Textarea::disabled` must disable the text engine, not only the chrome.
//!
//! Before the fix the builder only picked the disabled colours: a press still
//! focused the field, Tab still stopped on it and typing still edited it.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Bounds, Context, InputEvent as _, KeyBinding, KeyDownEvent, Keystroke, Modifiers,
    MouseButton, MouseDownEvent, Pixels, PlatformInput, Point, Render, TestAppContext, Window,
    WindowHandle, canvas, point, size,
};

type Slot = std::rc::Rc<std::cell::Cell<Option<Bounds<Pixels>>>>;

struct DisabledView {
    first: Entity<TextareaState>,
    middle: Entity<TextareaState>,
    last: Entity<TextareaState>,
    /// The builder value given to the middle field; `None` omits the builder.
    middle_disabled: Option<bool>,
    middle_bounds: Slot,
}

impl Render for DisabledView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = self.middle_bounds.clone();
        let mut middle = Textarea::new(&self.middle);
        if let Some(disabled) = self.middle_disabled {
            middle = middle.disabled(disabled);
        }
        FluentRoot::new().child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .p(px(20.))
                .w(px(260.))
                .child(Textarea::new(&self.first).tab_behavior(TextareaTabBehavior::Navigate))
                .child(
                    div()
                        .relative()
                        .child(
                            canvas(move |b, _, _| bounds.set(Some(b)), |_, (), _, _| {})
                                .absolute()
                                .size_full(),
                        )
                        .child(middle),
                )
                .child(Textarea::new(&self.last)),
        )
    }
}

struct Opened {
    window: WindowHandle<DisabledView>,
    first: Entity<TextareaState>,
    middle: Entity<TextareaState>,
    last: Entity<TextareaState>,
    middle_bounds: Slot,
}

fn open(cx: &mut TestAppContext, middle_disabled: Option<bool>, preset: bool) -> Opened {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new(
                "tab",
                crate::FocusNext,
                Some(crate::TEXT_INPUT_NAVIGATE_BINDING_CONTEXT),
            ),
            KeyBinding::new("tab", crate::FocusNext, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", crate::FocusPrev, Some(crate::ROOT_KEY_CONTEXT)),
        ]);
    });
    type States = (
        Entity<TextareaState>,
        Entity<TextareaState>,
        Entity<TextareaState>,
    );
    let slot: std::rc::Rc<std::cell::RefCell<Option<States>>> = Default::default();
    let for_view = slot.clone();
    let middle_bounds: Slot = Default::default();
    let bounds_for_view = middle_bounds.clone();
    let window = cx.open_window(size(px(500.), px(400.)), move |window, cx| {
        let first = cx.new(|cx| TextareaState::new(window, cx));
        let middle = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx);
            if preset {
                state.set_disabled(true, cx);
            }
            state
        });
        let last = cx.new(|cx| TextareaState::new(window, cx));
        for_view.replace(Some((first.clone(), middle.clone(), last.clone())));
        DisabledView {
            first,
            middle,
            last,
            middle_disabled,
            middle_bounds: bounds_for_view,
        }
    });
    cx.run_until_parked();
    let (first, middle, last) = slot.borrow().clone().expect("view built its states");
    Opened {
        window,
        first,
        middle,
        last,
        middle_bounds,
    }
}

fn centre(bounds: Bounds<Pixels>) -> Point<Pixels> {
    point(
        bounds.origin.x + bounds.size.width / 2.,
        bounds.origin.y + bounds.size.height / 2.,
    )
}

fn press_at(cx: &mut TestAppContext, handle: WindowHandle<DisabledView>, position: Point<Pixels>) {
    cx.update_window(handle.into(), |_, window, cx| {
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
    })
    .unwrap();
    cx.run_until_parked();
}

fn key(cx: &mut TestAppContext, handle: WindowHandle<DisabledView>, keystroke: &'static str) {
    let keystroke = Keystroke::parse(keystroke).unwrap();
    cx.update_window(handle.into(), move |_, window, cx| {
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
}

fn focus(
    cx: &mut TestAppContext,
    handle: WindowHandle<DisabledView>,
    state: &Entity<TextareaState>,
) {
    cx.update_window(handle.into(), |_, window, cx| {
        let focus = state.read(cx).presentation().focus_handle().clone();
        window.focus(&focus, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn is_focused(
    cx: &mut TestAppContext,
    handle: WindowHandle<DisabledView>,
    state: &Entity<TextareaState>,
) -> bool {
    cx.update_window(handle.into(), |_, window, cx| {
        state
            .read(cx)
            .presentation()
            .focus_handle()
            .is_focused(window)
    })
    .unwrap()
}

fn state_disabled(cx: &mut TestAppContext, state: &Entity<TextareaState>) -> bool {
    cx.update(|cx| state.read(cx).presentation().is_disabled())
}

#[gpui::test]
fn the_builder_value_reaches_the_text_engine(cx: &mut TestAppContext) {
    let opened = open(cx, Some(true), false);
    assert!(
        state_disabled(cx, &opened.middle),
        "Textarea::disabled(true) must disable the TextareaState, not only the styling"
    );
}

#[gpui::test]
fn pressing_a_disabled_field_does_not_focus_it(cx: &mut TestAppContext) {
    let opened = open(cx, Some(true), false);
    let bounds = opened.middle_bounds.get().expect("middle field measured");
    press_at(cx, opened.window, centre(bounds));
    assert!(
        !is_focused(cx, opened.window, &opened.middle),
        "a press on a disabled field must not focus it"
    );
}

#[gpui::test]
fn typing_into_a_disabled_field_does_not_edit_it(cx: &mut TestAppContext) {
    let opened = open(cx, Some(true), false);
    // Force focus programmatically: even then the engine must reject input.
    focus(cx, opened.window, &opened.middle);
    cx.commit_text(opened.window.into(), "a");
    cx.run_until_parked();
    let value = cx.update(|cx| opened.middle.read(cx).value().to_string());
    assert_eq!(value, "", "a disabled field must reject typed text");
}

#[gpui::test]
fn an_enabled_field_accepts_platform_committed_text(cx: &mut TestAppContext) {
    let opened = open(cx, Some(false), false);
    focus(cx, opened.window, &opened.middle);
    cx.commit_text(opened.window.into(), "a");
    cx.run_until_parked();
    assert_eq!(
        cx.update(|cx| opened.middle.read(cx).value().to_string()),
        "a",
        "precondition: this test must exercise the real platform input handler"
    );
}

#[gpui::test]
fn tab_skips_a_disabled_field(cx: &mut TestAppContext) {
    let opened = open(cx, Some(true), false);
    focus(cx, opened.window, &opened.first);
    assert!(is_focused(cx, opened.window, &opened.first), "precondition");
    key(cx, opened.window, "tab");
    assert!(
        is_focused(cx, opened.window, &opened.last),
        "Tab must skip the disabled field"
    );
}

#[gpui::test]
fn a_state_disabled_by_the_caller_stays_disabled_without_the_builder(cx: &mut TestAppContext) {
    let opened = open(cx, None, true);
    assert!(
        state_disabled(cx, &opened.middle),
        "omitting the builder must not re-enable a state the caller disabled"
    );
    let bounds = opened.middle_bounds.get().expect("middle field measured");
    press_at(cx, opened.window, centre(bounds));
    assert!(!is_focused(cx, opened.window, &opened.middle));
}

#[gpui::test]
fn an_explicit_false_re_enables_the_field(cx: &mut TestAppContext) {
    let opened = open(cx, Some(false), true);
    assert!(
        !state_disabled(cx, &opened.middle),
        "an explicit disabled(false) is the caller's value and wins over the state"
    );
    let bounds = opened.middle_bounds.get().expect("middle field measured");
    press_at(cx, opened.window, centre(bounds));
    assert!(is_focused(cx, opened.window, &opened.middle));
}
