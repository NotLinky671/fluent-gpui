//! Component gallery: the manual review surface for `fluent-ui`.
//!
//! Set `FLUENT_GALLERY_SMOKE=1` to open the window and quit automatically;
//! this is what CI uses to prove the application boots.

use std::time::Duration;

use fluent_ui::gpui::{
    div, px, size, App, Bounds, ClickEvent, Context, Hsla, Render, StatefulInteractiveElement, TitlebarOptions, Window,
    WindowBounds, WindowOptions,
};
use fluent_ui::prelude::*;
use fluent_ui::{Appearance, Button, ButtonAppearance, ButtonShape, ControlSize, FluentTheme};

const APPEARANCES: [ButtonAppearance; 5] = [
    ButtonAppearance::Secondary,
    ButtonAppearance::Outline,
    ButtonAppearance::Primary,
    ButtonAppearance::Subtle,
    ButtonAppearance::Transparent,
];

const SIZES: [ControlSize; 3] = [ControlSize::Small, ControlSize::Medium, ControlSize::Large];

fn appearance_name(appearance: ButtonAppearance) -> &'static str {
    match appearance {
        ButtonAppearance::Secondary => "secondary",
        ButtonAppearance::Outline => "outline",
        ButtonAppearance::Primary => "primary",
        ButtonAppearance::Subtle => "subtle",
        ButtonAppearance::Transparent => "transparent",
    }
}

fn size_name(size: ControlSize) -> &'static str {
    match size {
        ControlSize::Small => "small",
        ControlSize::Medium => "medium",
        ControlSize::Large => "large",
    }
}

/// Root view: theme switcher plus the button matrix.
struct Gallery {
    appearance: Appearance,
    clicks: usize,
}

impl Gallery {
    fn new() -> Self {
        Self {
            appearance: Appearance::Light,
            clicks: 0,
        }
    }

    fn apply(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        self.appearance = appearance;
        FluentTheme::for_appearance(appearance).install(cx);
        cx.notify();
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();

        let swatch = |c: Hsla| div().w(px(44.)).h(px(28.)).rounded(px(4.)).bg(c);

        let weak = cx.entity().downgrade();
        let handler = {
            let weak = weak.clone();
            move || {
                let weak = weak.clone();
                move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                    weak.update(cx, |this, cx| {
                        this.clicks += 1;
                        cx.notify();
                    })
                    .ok();
                }
            }
        };

        let mut matrix = div().flex().flex_col().gap(px(12.)).p(px(16.));
        let mut index = 0usize;

        for appearance in APPEARANCES {
            let mut row = div().flex().items_center().gap(px(12.)).child(
                div()
                    .w(px(96.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(appearance_name(appearance)),
            );
            for size in SIZES {
                row = row.child(
                    Button::new(("btn", index))
                        .appearance(appearance)
                        .size(size)
                        .on_click(handler())
                        .child(size_name(size)),
                );
                index += 1;
            }
            matrix = matrix.child(row);
        }

        // Disabled + shapes + icon-only, so the matrix covers every variant.
        let mut extra = div().flex().items_center().gap(px(12.)).child(
            div()
                .w(px(96.))
                .text_color(t.color.neutral_foreground_3)
                .child("disabled"),
        );
        for appearance in APPEARANCES {
            extra = extra.child(
                Button::new(("disabled", index))
                    .appearance(appearance)
                    .disabled(true)
                    .on_click(handler())
                    .child("disabled"),
            );
            index += 1;
        }
        matrix = matrix.child(extra);

        let mut shapes = div().flex().items_center().gap(px(12.)).child(
            div()
                .w(px(96.))
                .text_color(t.color.neutral_foreground_3)
                .child("shape"),
        );
        for shape in [ButtonShape::Rounded, ButtonShape::Circular, ButtonShape::Square] {
            shapes = shapes.child(
                Button::new(("shape", index))
                    .shape(shape)
                    .on_click(handler())
                    .child(match shape {
                        ButtonShape::Rounded => "rounded",
                        ButtonShape::Circular => "circular",
                        ButtonShape::Square => "square",
                    }),
            );
            index += 1;
        }
        matrix = matrix.child(shapes);

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
                    .child(
                        div()
                            .id("theme-light")
                            .px(px(12.))
                            .py(px(5.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(t.color.neutral_stroke_1)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| this.apply(Appearance::Light, cx)))
                            .child("Light"),
                    )
                    .child(
                        div()
                            .id("theme-dark")
                            .px(px(12.))
                            .py(px(5.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(t.color.neutral_stroke_1)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| this.apply(Appearance::Dark, cx)))
                            .child("Dark"),
                    )
                    .child(
                        div()
                            .ml(px(8.))
                            .text_color(t.color.neutral_foreground_3)
                            .child(format!(
                                "appearance: {:?}   clicks: {}",
                                theme.appearance(),
                                self.clicks
                            )),
                    ),
            )
            .child(
                div()
                    .id("matrix-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(matrix)
                    .child(
                        div()
                            .p(px(16.))
                            .flex()
                            .gap(px(8.))
                            .child(swatch(t.color.brand_background))
                            .child(swatch(t.color.neutral_background_1_hover))
                            .child(swatch(t.color.neutral_stroke_accessible))
                            .child(swatch(t.color.stroke_focus_2)),
                    ),
            )
    }
}

fn main() {
    fluent_ui::application().run(|cx: &mut App| {
        fluent_ui::init(cx);

        let bounds = Bounds::centered(None, size(px(900.), px(640.)), cx);
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