//! `fluent-notes`: an in-memory notes integration app for `fluent-ui`.
//!
//! The gallery shows each component in isolation; this app exercises the flows
//! that only appear when the components are combined: a draft that must survive
//! redraws and theme changes, menu keyboard navigation that updates real state,
//! popover focus handover, and explicit confirmation before losing work.
//!
//! Data never leaves memory. Closing the window loses every note.

mod app;
mod model;

use std::time::Duration;

use app::NotesApp;
use fluent_ui::gpui::{
    App, AppContext as _, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};
use fluent_ui::{Appearance, FluentTheme};

fn main() {
    fluent_ui::application().run(|cx: &mut App| {
        fluent_ui::init(cx);

        if let Some(value) = std::env::var_os("FLUENT_NOTES_APPEARANCE") {
            let appearance = match value.to_string_lossy().to_ascii_lowercase().as_str() {
                "dark" => Appearance::Dark,
                "hc" | "high-contrast" => Appearance::HighContrast,
                _ => Appearance::Light,
            };
            FluentTheme::for_appearance(appearance).install(cx);
        }

        // Fit the default window to the display instead of assuming 1180x760 is
        // always available: at high DPI or on a small laptop that size can exceed
        // the work area before the window is even shown.
        let desired = size(px(1180.), px(760.));
        let (width, height) = match cx
            .primary_display()
            .map(|display| display.visible_bounds().size)
        {
            Some(available) => (
                desired.width.min(available.width - px(64.)).max(px(720.)),
                desired.height.min(available.height - px(96.)).max(px(480.)),
            ),
            None => (desired.width, desired.height),
        };
        let bounds = Bounds::centered(None, size(width, height), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Fluent Notes — 内存便签".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| NotesApp::new(window, cx)),
            )
            .expect("failed to open the notes window");

        // CI boot smoke: opening the window and drawing the first tree catches
        // panics that unit tests and behaviour tests cannot. It is not an
        // interaction check.
        if std::env::var_os("FLUENT_NOTES_SMOKE").is_some() {
            let executor = cx.background_executor().clone();
            cx.spawn(async move |cx| {
                executor.timer(Duration::from_millis(1500)).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
        let _ = window;

        cx.activate(true);
    });
}
