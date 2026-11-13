//! C5: a real pressed-left drag across the field selects text and Cmd+C copies
//! the selected range. The Fluent wrapper adds focus/blur and hover chrome
//! around gpui-base's input; this drives the pointer path through that wrapper.

use crate::root::FluentRoot;
use crate::{TextInput, Textarea};
use fluent_core::gpui_compat::Hsla;
use gpui::{
    AnyWindowHandle, AppContext, Context, Entity, InputEvent as _, IntoElement, KeyDownEvent,
    Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement as _, PlatformInput, Render, Styled as _, TestAppContext, Window, WindowHandle,
    div, point, px, size,
};
use gpui_base::input::{InputState, TextareaState};

struct SelectionView {
    state: Entity<InputState>,
}

impl Render for SelectionView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(
            div()
                .p(px(20.))
                .child(div().w(px(240.)).child(TextInput::new(&self.state))),
        )
    }
}

struct TextareaSelectionView {
    state: Entity<TextareaState>,
}

impl Render for TextareaSelectionView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(
            div()
                .p(px(20.))
                .child(div().w(px(240.)).child(Textarea::new(&self.state))),
        )
    }
}

fn open_textarea(
    cx: &mut TestAppContext,
) -> (WindowHandle<TextareaSelectionView>, Entity<TextareaState>) {
    cx.update(|cx| {
        fluent_core::gpui_compat::init_base(cx);
        fluent_theme::FluentTheme::web_light().install(cx);
    });
    let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<TextareaState>>>> = Default::default();
    let for_view = slot.clone();
    let window = cx.open_window(size(px(400.), px(220.)), move |window, cx| {
        let state = cx.new(|cx| TextareaState::new(window, cx).default_value("hello world"));
        for_view.replace(Some(state.clone()));
        TextareaSelectionView { state }
    });
    cx.run_until_parked();
    let state = slot.borrow().clone().expect("textarea state built");
    (window, state)
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<SelectionView>, Entity<InputState>) {
    cx.update(|cx| {
        fluent_core::gpui_compat::init_base(cx);
        fluent_theme::FluentTheme::web_light().install(cx);
    });
    let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<InputState>>>> = Default::default();
    let for_view = slot.clone();
    let window = cx.open_window(size(px(400.), px(200.)), move |window, cx| {
        let state = cx.new(|cx| InputState::new(window, cx).default_value("hello world"));
        for_view.replace(Some(state.clone()));
        SelectionView { state }
    });
    cx.run_until_parked();
    let state = slot.borrow().clone().expect("state built");
    (window, state)
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

fn assert_visible_selection_path(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    expected: Hsla,
    min_width: f32,
) {
    let paths = cx
        .update_window(handle, |_, window, _| window.painted_paths())
        .unwrap();
    let found = paths.iter().any(|path| {
        let Some(color) = path.color.as_solid() else {
            return false;
        };
        // The highlight must come from the Fluent-designated selection token,
        // not the much lighter accent token. The comparison is exact enough to
        // fail when gpui-base substitutes accent, and theme installation keeps
        // the token current for light/dark/high-contrast.
        (color.h - expected.h).abs() < 0.01
            && (color.s - expected.s).abs() < 0.02
            && (color.l - expected.l).abs() < 0.02
            && (color.a - expected.a).abs() < 0.02
            && path.bounds.size.width.0 >= min_width
    });
    assert!(
        found,
        "selection path must use the Fluent selection token {expected:?}; paths: {:?}",
        paths
            .iter()
            .map(|path| (path.color, path.bounds, path.vertices.len()))
            .collect::<Vec<_>>()
    );
}

fn copy(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.update_window(handle, |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse("cmd-c").unwrap(),
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
fn dragging_across_the_field_selects_and_copies_text(cx: &mut TestAppContext) {
    let (window, state) = open(cx);
    let handle: AnyWindowHandle = window.into();
    cx.update_window(handle, |_, window, _| window.activate_window())
        .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
    assert!(
        cx.update_window(handle, |_, window, _| window.is_window_active())
            .unwrap(),
        "selection paint is gated on an active window"
    );

    // Press at the first glyph, drag past the right edge of the field, release.
    let start = point(px(33.), px(36.));
    let end = point(px(250.), px(36.));
    cx.update_window(handle, move |_, window, cx| {
        window.dispatch_event(
            MouseDownEvent {
                button: MouseButton::Left,
                position: start,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            MouseMoveEvent {
                position: end,
                modifiers: Modifiers::default(),
                pressed_button: Some(MouseButton::Left),
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button: MouseButton::Left,
                position: end,
                modifiers: Modifiers::default(),
                click_count: 1,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);

    let selected = cx.update(|cx| state.read(cx).selected_text().to_string());
    assert_eq!(
        selected, "hello world",
        "a left-button drag from the first glyph must select the whole text"
    );

    // The drag must be visible, not merely stored in the editing state. The
    // selection is painted as a path under the glyphs; assert a wide, blue
    // path exists instead of a rasterizer-dependent pixel sample.
    let expected = cx.update(|cx| {
        fluent_theme::FluentTheme::global(cx)
            .tokens()
            .color
            .brand_background
            .alpha(0.3)
    });
    assert_visible_selection_path(cx, handle, expected, 60.);

    copy(cx, handle);
    let copied = cx
        .update(|cx| cx.read_from_clipboard())
        .and_then(|item| item.text());
    assert_eq!(
        copied.as_deref(),
        Some(selected.as_str()),
        "Cmd+C must copy the drag-selected range"
    );
}

#[gpui::test]
fn dragging_across_the_textarea_selects_and_copies(cx: &mut TestAppContext) {
    let (window, state) = open_textarea(cx);
    let handle: AnyWindowHandle = window.into();
    cx.update_window(handle, |_, window, _| window.activate_window())
        .unwrap();
    cx.run_until_parked();
    settle(cx, handle);

    let start = point(px(33.), px(32.));
    let end = point(px(250.), px(32.));
    cx.update_window(handle, move |_, window, cx| {
        window.dispatch_event(
            MouseDownEvent {
                button: MouseButton::Left,
                position: start,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            MouseMoveEvent {
                position: end,
                modifiers: Modifiers::default(),
                pressed_button: Some(MouseButton::Left),
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button: MouseButton::Left,
                position: end,
                modifiers: Modifiers::default(),
                click_count: 1,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);

    let selected = cx.update(|cx| state.read(cx).selected_text().to_string());
    assert_eq!(
        selected, "hello world",
        "a left-button drag from the first glyph must select the whole text"
    );
    let expected = cx.update(|cx| {
        fluent_theme::FluentTheme::global(cx)
            .tokens()
            .color
            .brand_background
            .alpha(0.3)
    });
    assert_visible_selection_path(cx, handle, expected, 60.);

    copy(cx, handle);
    let copied = cx
        .update(|cx| cx.read_from_clipboard())
        .and_then(|item| item.text());
    assert_eq!(
        copied.as_deref(),
        Some(selected.as_str()),
        "Cmd+C must copy the textarea drag-selected range"
    );
}
