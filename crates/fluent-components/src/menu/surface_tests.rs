//! A menu inside a popover must paint one surface, not two.
//!
//! `Popover` draws a panel card (padding, radius, background, shadow) for content
//! that has none. `Menu` already draws its own surface, so leaving the panel card
//! on stacks a second card and insets every row by the panel padding.

use super::*;
use crate::root::FluentRoot;
use gpui::{AppContext, Context, Render, TestAppContext, Window, px, size};

struct View;

impl Render for View {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = Menu::new("surface-panel")
            .item(MenuItem::new("rename", "Rename").icon(FluentIcon::Settings))
            .item(MenuItem::new("copy", "Copy"))
            .separator()
            .item(MenuItem::new("delete", "Delete").danger(true));

        FluentRoot::new().child(
            div().size_full().p(px(40.)).child(
                MenuDropdown::new(
                    "surface-trigger",
                    div().id("surface-btn").w(px(160.)).h(px(32.)).child("Open"),
                    menu,
                )
                .default_open(true)
                .track_focus(&cx.focus_handle()),
            ),
        )
    }
}

/// Card-sized painted surfaces, ignoring the window background.
fn cards(cx: &mut TestAppContext, handle: gpui::WindowHandle<View>) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        let scale = window.scale_factor();
        out = window
            .painted_quads()
            .into_iter()
            .filter(|quad| quad.background.as_solid().is_some_and(|c| c.a > 0.9))
            .map(|quad| {
                (
                    quad.bounds.size.width.0 / scale,
                    quad.bounds.size.height.0 / scale,
                )
            })
            // The window background spans the whole viewport.
            .filter(|(w, h)| *w < 400. && *h > 40.)
            .collect();
    })
    .unwrap();
    out
}

#[gpui::test]
fn the_dropdown_paints_a_single_surface(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    let handle = cx.open_window(size(px(500.), px(400.)), |_, _| View);
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    // Let the open reveal finish: the panel starts at zero height.
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(200));
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    let cards = cards(cx, handle);
    assert_eq!(
        cards.len(),
        1,
        "expected one menu surface, got {cards:?} — a panel card around the menu stacks a second surface"
    );
}
