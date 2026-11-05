//! Tabs: keyboard navigation, disabled handling, and the sliding indicator.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, PlatformInput, Render,
    TestAppContext, Window, WindowHandle, px, size,
};

type Selected = std::rc::Rc<std::cell::RefCell<Vec<usize>>>;

struct View {
    focus: FocusHandle,
    changes: Selected,
}

impl Render for View {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let changes = self.changes.clone();
        FluentRoot::new().child(
            div().size_full().p(px(20.)).child(
                TabList::new("probe-tabs")
                    .track_focus(&self.focus)
                    .default_selected(0)
                    .on_change(move |index, _, _| changes.borrow_mut().push(*index))
                    .tab(TabItem::new("tab-a", "Alpha"))
                    .tab(TabItem::new("tab-b", "Beta"))
                    .tab(TabItem::new("tab-c", "Gamma").disabled(true))
                    .tab(TabItem::new("tab-d", "Delta")),
            ),
        )
    }
}

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("right", crate::TabsNext, Some(crate::TABS_KEY_CONTEXT)),
            KeyBinding::new("down", crate::TabsNext, Some(crate::TABS_KEY_CONTEXT)),
            KeyBinding::new("left", crate::TabsPrev, Some(crate::TABS_KEY_CONTEXT)),
            KeyBinding::new("up", crate::TabsPrev, Some(crate::TABS_KEY_CONTEXT)),
            KeyBinding::new("home", crate::TabsFirst, Some(crate::TABS_KEY_CONTEXT)),
            KeyBinding::new("end", crate::TabsLast, Some(crate::TABS_KEY_CONTEXT)),
        ]);
    });
}

fn settle(cx: &mut TestAppContext, handle: WindowHandle<View>) {
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<View>, Selected) {
    bind_keys(cx);
    let changes: Selected = Default::default();
    let for_view = changes.clone();
    let handle = cx.open_window(size(px(600.), px(400.)), move |_, cx| View {
        focus: cx.focus_handle().tab_stop(true),
        changes: for_view,
    });
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(handle.into(), |view, window, cx| {
            let view = view.downcast::<View>().unwrap();
            let focus = view.read(cx).focus.clone();
            focus.focus(window, cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    (handle, changes)
}

fn press(cx: &mut TestAppContext, handle: WindowHandle<View>, key: &'static str) {
    let keystroke = Keystroke::parse(key).unwrap();
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
    settle(cx, handle);
}

/// Bounds of the indicator: the only painted 2px bar inside the strip.
fn indicator(cx: &mut TestAppContext, handle: WindowHandle<View>) -> Option<(f32, f32)> {
    let mut found = None;
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        let scale = window.scale_factor();
        found = window
            .painted_quads()
            .into_iter()
            .filter(|quad| (quad.bounds.size.height.0 / scale - 2.0).abs() < 0.6)
            .map(|quad| {
                (
                    quad.bounds.origin.x.0 / scale,
                    quad.bounds.size.width.0 / scale,
                )
            })
            .next();
    })
    .unwrap();
    cx.run_until_parked();
    found
}

#[gpui::test]
fn arrows_move_the_selection_and_skip_disabled_tabs(cx: &mut TestAppContext) {
    let (handle, changes) = open(cx);
    press(cx, handle, "right");
    press(cx, handle, "right");
    assert_eq!(
        *changes.borrow(),
        vec![1, 3],
        "right must step to Beta and then skip the disabled Gamma for Delta"
    );
}

#[gpui::test]
fn home_and_end_jump_to_the_selectable_edges(cx: &mut TestAppContext) {
    let (handle, changes) = open(cx);
    press(cx, handle, "end");
    press(cx, handle, "home");
    assert_eq!(*changes.borrow(), vec![3, 0]);
}

#[gpui::test]
fn vertical_strips_use_the_same_arrows(cx: &mut TestAppContext) {
    // The binding maps both axes to the same actions, so Down works for a
    // horizontal strip and Right works for a vertical one.
    let (handle, changes) = open(cx);
    press(cx, handle, "down");
    assert_eq!(*changes.borrow(), vec![1]);
}

#[gpui::test]
fn the_indicator_follows_the_selected_tab(cx: &mut TestAppContext) {
    let (handle, _) = open(cx);
    settle(cx, handle);
    let first = indicator(cx, handle).expect("the indicator must be painted");
    assert!(first.1 > 0.0, "the indicator must have the tab's width");

    press(cx, handle, "right");
    // Let the transition finish.
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    cx.run_until_parked();
    settle(cx, handle);
    let second = indicator(cx, handle).expect("the indicator must still be painted");

    assert!(
        second.0 > first.0,
        "the indicator must move right with the selection: {first:?} then {second:?}"
    );
}
/// A controlled strip whose caller hands it an out-of-range index.
struct ClampView {
    focus: FocusHandle,
    selected: usize,
}

impl Render for ClampView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(
            div().size_full().p(px(20.)).child(
                TabList::new("clamp-tabs")
                    .track_focus(&self.focus)
                    .selected(self.selected)
                    .tab(TabItem::new("c-a", "Alpha"))
                    .tab(TabItem::new("c-b", "Beta"))
                    .tab(TabItem::new("c-c", "Gamma")),
            ),
        )
    }
}

fn clamp_view(cx: &mut TestAppContext, selected: usize) -> WindowHandle<ClampView> {
    bind_keys(cx);
    let handle = cx.open_window(size(px(500.), px(300.)), move |_, cx| ClampView {
        focus: cx.focus_handle().tab_stop(true),
        selected,
    });
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(handle.into(), |view, window, cx| {
            let view = view.downcast::<ClampView>().unwrap();
            let focus = view.read(cx).focus.clone();
            focus.focus(window, cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    handle
}

fn clamp_indicator_x(cx: &mut TestAppContext, handle: WindowHandle<ClampView>) -> f32 {
    let mut x = -1.0;
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        let scale = window.scale_factor();
        x = window
            .painted_quads()
            .into_iter()
            .filter(|quad| (quad.bounds.size.height.0 / scale - 2.0).abs() < 0.6)
            .map(|quad| quad.bounds.origin.x.0 / scale)
            .next()
            .unwrap_or(-1.0);
    })
    .unwrap();
    cx.run_until_parked();
    x
}

#[gpui::test]
fn an_out_of_range_selection_clamps_to_the_last_tab(cx: &mut TestAppContext) {
    // Two windows in the same app context: keyed state is per window, so each
    // strip resolves its own selection.
    let last = {
        let handle = clamp_view(cx, 2);
        clamp_indicator_x(cx, handle)
    };
    let out_of_range = {
        let handle = clamp_view(cx, 9);
        clamp_indicator_x(cx, handle)
    };
    assert!(last > 0.0, "the last tab must paint an indicator");
    assert_eq!(
        out_of_range, last,
        "an index past the end must clamp to the last tab, not jump back to the first"
    );
}
/// A strip whose two tabs differ a lot in width, so a width snap is measurable.
struct WidthView {
    focus: FocusHandle,
}

impl Render for WidthView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(
            div().size_full().p(px(20.)).child(
                TabList::new("width-tabs")
                    .track_focus(&self.focus)
                    .default_selected(1)
                    .tab(TabItem::new("w-a", "A"))
                    .tab(TabItem::new("w-b", "A much wider label")),
            ),
        )
    }
}

fn width_view(cx: &mut TestAppContext) -> WindowHandle<WidthView> {
    bind_keys(cx);
    let handle = cx.open_window(size(px(600.), px(300.)), move |_, cx| WidthView {
        focus: cx.focus_handle().tab_stop(true),
    });
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(handle.into(), |view, window, cx| {
            let view = view.downcast::<WidthView>().unwrap();
            let focus = view.read(cx).focus.clone();
            focus.focus(window, cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    handle
}

fn indicator_width(cx: &mut TestAppContext, handle: WindowHandle<WidthView>) -> f32 {
    let mut width = -1.0;
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        let scale = window.scale_factor();
        width = window
            .painted_quads()
            .into_iter()
            .filter(|quad| (quad.bounds.size.height.0 / scale - 2.0).abs() < 0.6)
            .map(|quad| quad.bounds.size.width.0 / scale)
            .next()
            .unwrap_or(-1.0);
    })
    .unwrap();
    cx.run_until_parked();
    width
}

#[gpui::test]
fn the_indicator_width_animates_with_its_position(cx: &mut TestAppContext) {
    let handle = width_view(cx);
    let wide = indicator_width(cx, handle);
    assert!(wide > 0.0, "the wide tab must paint an indicator");

    // Move to the narrow tab and sample partway through the transition.
    let keystroke = Keystroke::parse("left").unwrap();
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
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(60));
    cx.run_until_parked();
    let mid = indicator_width(cx, handle);

    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    cx.run_until_parked();
    let narrow = indicator_width(cx, handle);

    assert!(
        narrow < wide,
        "precondition: the second tab must be wider than the first ({narrow} vs {wide})"
    );
    assert!(
        mid > narrow && mid < wide,
        "the width must interpolate, not snap: wide {wide}, mid {mid}, narrow {narrow}"
    );
}
