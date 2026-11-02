//! Focus behaviour of `TextInput`.
//!
//! GPUI does not clear focus when the pointer lands elsewhere, and the text
//! engine underneath does not either, so "clicking away blurs the field" is this
//! component's own responsibility. These tests drive real pointer events at a
//! window rather than calling the handlers directly, because the interesting part
//! is which element the press reaches.

use super::*;
use gpui::{
    AppContext, Bounds, Context, InputEvent as _, Modifiers, MouseButton, MouseDownEvent, Pixels,
    Point, Render, TestAppContext, Window, WindowHandle, canvas, point, size,
};

type Field = std::rc::Rc<std::cell::Cell<Option<Bounds<Pixels>>>>;

/// Centre of a measured bounds, in window coordinates.
fn centre(bounds: Bounds<Pixels>) -> Point<Pixels> {
    point(
        bounds.origin.x + bounds.size.width / 2.,
        bounds.origin.y + bounds.size.height / 2.,
    )
}

struct FocusView {
    state: Entity<InputState>,
    field: Field,
}

impl Render for FocusView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let field = self.field.clone();
        div().size_full().p(px(40.)).child(
            div()
                .relative()
                .w(px(220.))
                .child(
                    canvas(
                        move |bounds, _, _| field.set(Some(bounds)),
                        |_, (), _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
                .child(TextInput::new(&self.state)),
        )
    }
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<FocusView>, Entity<InputState>, Field) {
    // The text engine keeps process-wide state; without it InputState::new
    // panics before any of this component's behaviour is exercised.
    cx.update(fluent_core::gpui_compat::init_base);
    let field: Field = std::rc::Rc::new(std::cell::Cell::new(None));
    let field_for_view = field.clone();
    let state_slot: std::rc::Rc<std::cell::RefCell<Option<Entity<InputState>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let slot = state_slot.clone();
    let window = cx.open_window(size(px(600.), px(400.)), move |window, cx| {
        let state = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        slot.replace(Some(state.clone()));
        FocusView {
            state,
            field: field_for_view,
        }
    });
    cx.run_until_parked();
    let state = state_slot.borrow().clone().expect("view created a state");
    (window, state, field)
}

fn press(cx: &mut TestAppContext, handle: WindowHandle<FocusView>, position: Point<Pixels>) {
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

fn is_focused(
    cx: &mut TestAppContext,
    handle: WindowHandle<FocusView>,
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
fn pressing_inside_the_field_focuses_it(cx: &mut TestAppContext) {
    let (handle, state, field) = open(cx);
    let bounds = field.get().expect("field measured");
    press(cx, handle, centre(bounds));
    assert!(
        is_focused(cx, handle, &state),
        "a press inside the field must focus it"
    );
}

#[gpui::test]
fn pressing_outside_the_field_blurs_it(cx: &mut TestAppContext) {
    let (handle, state, field) = open(cx);
    let bounds = field.get().expect("field measured");
    press(cx, handle, centre(bounds));
    assert!(is_focused(cx, handle, &state), "precondition: focused");

    // Well below the field, inside the window.
    press(cx, handle, point(bounds.left(), bounds.bottom() + px(120.)));
    assert!(
        !is_focused(cx, handle, &state),
        "a press outside the field must clear focus, otherwise the underline \
         keeps drawing as if the field were still active"
    );
}

#[gpui::test]
fn pressing_inside_again_keeps_focus(cx: &mut TestAppContext) {
    let (handle, state, field) = open(cx);
    let bounds = field.get().expect("field measured");
    press(cx, handle, centre(bounds));
    press(cx, handle, centre(bounds));
    assert!(
        is_focused(cx, handle, &state),
        "a second press inside the field must not blur it"
    );
}
