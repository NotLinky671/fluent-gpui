//! Component gallery: the manual review surface for `fluent-ui`.
//!
//! Set `FLUENT_GALLERY_SMOKE=1` to open the window and quit automatically;
//! this is what CI uses to prove the application boots.

use std::time::Duration;

use fluent_ui::gpui::{
    div, px, size, App, Bounds, Context, Hsla, Render, TitlebarOptions, Window, WindowBounds,
    WindowOptions,
};
use fluent_ui::prelude::*;
use fluent_ui::{Appearance, FluentTheme};

/// Root view. Real gallery pages arrive with the components.
struct Gallery {
    appearance: Appearance,
}

impl Gallery {
    fn new() -> Self {
        Self {
            appearance: Appearance::Light,
        }
    }

    fn apply(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        self.appearance = appearance;
        let theme = FluentTheme::for_appearance(appearance);
        theme.install(cx);
        cx.notify();
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();

        let swatch = |c: Hsla| div().w(px(44.)).h(px(28.)).rounded(px(4.)).bg(c);

        let light_button = div()
            .id("theme-light")
            .px(px(12.))
            .py(px(5.))
            .rounded(px(4.))
            .border_1()
            .border_color(t.color.neutral_stroke_1)
            .cursor_pointer()
            .hover(move |s| s.bg(t.color.neutral_background_1_hover))
            .on_click(cx.listener(|this, _, _, cx| this.apply(Appearance::Light, cx)))
            .child("Light");

        let dark_button = div()
            .id("theme-dark")
            .px(px(12.))
            .py(px(5.))
            .rounded(px(4.))
            .border_1()
            .border_color(t.color.neutral_stroke_1)
            .cursor_pointer()
            .hover(move |s| s.bg(t.color.neutral_background_1_hover))
            .on_click(cx.listener(|this, _, _, cx| this.apply(Appearance::Dark, cx)))
            .child("Dark");

        div()
            .size_full()
            .bg(t.color.neutral_background_1)
            .text_color(t.color.neutral_foreground_1)
            .text_size(px(14.))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .p(px(16.))
                    .border_b_1()
                    .border_color(t.color.neutral_stroke_1)
                    .child(light_button)
                    .child(dark_button)
                    .child(
                        div()
                            .ml(px(8.))
                            .text_color(t.color.neutral_foreground_3)
                            .child(format!("appearance: {:?}", theme.appearance())),
                    ),
            )
            .child(
                div()
                    .p(px(16.))
                    .flex()
                    .gap(px(8.))
                    .child(swatch(t.color.brand_background))
                    .child(swatch(t.color.neutral_background_1_hover))
                    .child(swatch(t.color.neutral_stroke_accessible))
                    .child(swatch(t.color.stroke_focus_2)),
            )
    }
}

fn main() {
    fluent_ui::application().run(|cx: &mut App| {
        fluent_ui::init(cx);

        let bounds = Bounds::centered(None, size(px(800.), px(600.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Fluent Gallery".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_window, cx| cx.new(|_| Gallery::new()),
        )
        .expect("failed to open window");

        if std::env::var_os("FLUENT_GALLERY_SMOKE").is_some() {
            let executor = cx.background_executor().clone();
            cx.spawn(async move |cx| {
                executor.timer(Duration::from_millis(1500)).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }

        cx.activate(true);
    });
}