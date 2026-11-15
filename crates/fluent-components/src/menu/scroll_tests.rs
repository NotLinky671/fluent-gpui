//! A menu taller than the window.
//!
//! Fluent keeps a menu on screen: the panel stops at the window edge and the
//! rows scroll inside it. These tests measure the painted result, because a
//! panel that overflows the viewport and one that is clamped there paint
//! differently, and only the second one leaves its last row reachable.

use super::*;
use crate::root::FluentRoot;
use fluent_core::spec::menu as menu_spec;
use gpui::{
    AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, PlatformInput, Render,
    TestAppContext, Window, WindowHandle, point, size,
};

/// Rows in the long menu. Tall enough that the content cannot fit the test
/// window whatever the row metrics are.
const ROWS: usize = 24;

struct ScrollView {
    focus: FocusHandle,
}

impl Render for ScrollView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut menu = Menu::new("scroll-menu");
        for index in 0..ROWS {
            menu = menu.item(MenuItem::new(
                ElementId::from(SharedString::from(format!("row-{index}"))),
                SharedString::from(format!("Row {index}")),
            ));
        }

        FluentRoot::new().child(
            div().size_full().p(px(24.)).child(
                MenuDropdown::new(
                    "scroll-dropdown",
                    div()
                        .id("scroll-trigger")
                        .w(px(120.))
                        .h(px(28.))
                        .child("open"),
                    menu,
                )
                .default_open(true)
                .track_focus(&self.focus),
            ),
        )
    }
}

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("home", crate::MenuFirst, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("end", crate::MenuLast, Some(crate::MENU_KEY_CONTEXT)),
        ]);
    });
}

fn open(cx: &mut TestAppContext) -> WindowHandle<ScrollView> {
    bind_keys(cx);
    let handle = cx.open_window(size(px(600.), px(400.)), |_, cx| ScrollView {
        focus: cx.focus_handle().tab_stop(true),
    });
    cx.run_until_parked();
    draw(cx, handle);
    handle
}

fn draw(cx: &mut TestAppContext, handle: WindowHandle<ScrollView>) {
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

/// Dispatches `keys` after focusing the menu, redrawing after each one.
fn press(cx: &mut TestAppContext, handle: WindowHandle<ScrollView>, keys: &[&'static str]) {
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<ScrollView>().unwrap();
        let focus = view.read(cx).focus.clone();
        focus.focus(window, cx);
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
    draw(cx, handle);

    for key in keys {
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
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

/// The menu surface, and the current row when there is one, in logical pixels.
///
/// The block colour tells them apart: only the current row paints the hover
/// fill, and the panel is the only opaque surface taller than a row.
fn painted(
    cx: &mut TestAppContext,
    handle: WindowHandle<ScrollView>,
) -> (Bounds<Pixels>, Option<Bounds<Pixels>>) {
    let mut panel = None;
    let mut row = None;
    cx.update_window(handle.into(), |_, window, cx| {
        let theme = FluentTheme::global(cx);
        let hover = menu_spec::item(&theme, &MenuItemProps::default()).hover_background;
        let scale = window.scale_factor();
        let to_px = |value: gpui::ScaledPixels| px(value.0 / scale);
        for quad in window.painted_quads() {
            let Some(colour) = quad.background.as_solid() else {
                continue;
            };
            let bounds = Bounds {
                origin: Point {
                    x: to_px(quad.bounds.origin.x),
                    y: to_px(quad.bounds.origin.y),
                },
                size: Size {
                    width: to_px(quad.bounds.size.width),
                    height: to_px(quad.bounds.size.height),
                },
            };
            if colour.a > 0.9 && bounds.size.width < px(400.) && bounds.size.height > px(40.) {
                panel = Some(bounds);
            }
            if colour == hover && (bounds.size.height - px(32.)).abs() < px(1.5) {
                row = Some(bounds);
            }
        }
    })
    .unwrap();
    (panel.expect("the menu panel must be painted"), row)
}

#[gpui::test]
fn a_menu_taller_than_the_window_stops_at_the_window(cx: &mut TestAppContext) {
    let handle = open(cx);
    draw(cx, handle);
    // Let the reveal finish so the panel is at its final size.
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    draw(cx, handle);

    let (panel, _) = painted(cx, handle);
    // 400px window, 8px margin at each edge.
    assert!(
        (panel.size.height - px(384.)).abs() <= px(1.5),
        "the panel must stop at the window edge, got {} for a {} row menu",
        panel.size.height,
        ROWS
    );
    assert!(
        panel.bottom() <= px(400.),
        "the panel must not paint past the window: {:?}",
        panel
    );
}

#[gpui::test]
fn the_keyboard_scrolls_the_current_row_into_view(cx: &mut TestAppContext) {
    let handle = open(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    draw(cx, handle);

    // End moves to the last row, which without scrolling sits below the window.
    press(cx, handle, &["end"]);
    let (panel, row) = painted(cx, handle);
    let row = row.expect("the current row must be painted with the hover fill");
    assert!(
        row.bottom() <= px(400.) && row.top() >= panel.top(),
        "the last row must be scrolled into view, got {row:?} in a panel at {panel:?}"
    );

    // Home has to bring the first row back the same way.
    press(cx, handle, &["home"]);
    let (panel, row) = painted(cx, handle);
    let row = row.expect("the current row must be painted with the hover fill");
    assert!(
        row.top() - panel.top() <= px(12.),
        "the first row must be scrolled back to the top, got {row:?} in a panel at {panel:?}"
    );
}

/// One wheel step over the middle of the panel.
///
/// A positive delta scrolls towards the top of the rows, which is also the
/// direction the native listener applies.
fn wheel(cx: &mut TestAppContext, handle: WindowHandle<ScrollView>, delta_y: f32) {
    let (panel, _) = painted(cx, handle);
    let position = panel.center();
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            PlatformInput::Scroll(gpui::ScrollEvent {
                position,
                delta: gpui::ScrollDelta::Pixels(point(px(0.), px(delta_y))),
                ..Default::default()
            }),
            cx,
        );
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
    draw(cx, handle);
}

/// Whether the painted current row sits inside the painted panel.
fn inside(panel: Bounds<Pixels>, row: Option<Bounds<Pixels>>) -> bool {
    row.is_some_and(|row| {
        row.top() >= panel.top() - px(1.) && row.bottom() <= panel.bottom() + px(1.)
    })
}

#[gpui::test]
fn the_keyboard_brings_the_current_row_back_after_the_wheel(cx: &mut TestAppContext) {
    let handle = open(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    draw(cx, handle);

    press(cx, handle, &["end"]);
    let (panel, row) = painted(cx, handle);
    assert!(
        inside(panel, row),
        "precondition: End must scroll the last row into view"
    );

    // The wheel scrolls the row it just revealed out of sight. A panel that
    // ignored the wheel would fail here, which is why this is measured.
    wheel(cx, handle, 2000.);
    let (panel, row) = painted(cx, handle);
    assert!(
        !inside(panel, row),
        "the wheel must be able to hide the current row, got {row:?} in a panel at {panel:?}"
    );

    // A plain repaint must leave the panel where the user scrolled it.
    draw(cx, handle);
    let (panel, row) = painted(cx, handle);
    assert!(
        !inside(panel, row),
        "a repaint must not fight the wheel, got {row:?} in a panel at {panel:?}"
    );

    // The same End has to bring the row back even though the current row did not
    // change: the explicit keyboard request is the only signal left.
    press(cx, handle, &["end"]);
    let (panel, row) = painted(cx, handle);
    assert!(
        inside(panel, row),
        "End must restore the current row, got {row:?} in a panel at {panel:?}"
    );
}

#[gpui::test]
fn home_brings_the_first_row_back_after_the_wheel(cx: &mut TestAppContext) {
    let handle = open(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    draw(cx, handle);

    press(cx, handle, &["home"]);
    let (panel, row) = painted(cx, handle);
    assert!(
        inside(panel, row),
        "precondition: Home must leave the first row at the top"
    );

    // Negative delta scrolls towards the end of the rows.
    wheel(cx, handle, -2000.);
    let (panel, row) = painted(cx, handle);
    assert!(
        !inside(panel, row),
        "the wheel must be able to hide the first row, got {row:?} in a panel at {panel:?}"
    );

    press(cx, handle, &["home"]);
    let (panel, row) = painted(cx, handle);
    assert!(
        inside(panel, row),
        "Home must restore the first row, got {row:?} in a panel at {panel:?}"
    );
}
