//! Component gallery: the manual review surface for `fluent-ui`.
//!
//! - `FLUENT_GALLERY_SMOKE=1` opens the window and quits, for CI boot checks.
//! - `FLUENT_GALLERY_SCREENSHOT=<path>` (with `--features screenshot`) writes a
//!   PNG of the first frame.

use std::time::Duration;

use fluent_ui::gpui::{
    App, Bounds, ClickEvent, Context, Hsla, Render, StatefulInteractiveElement, Subscription,
    TitlebarOptions, Window, WindowBounds, WindowOptions, div, px, size,
};
use fluent_ui::prelude::*;
use fluent_ui::{
    Appearance, Button, ButtonAppearance, ButtonShape, ControlSize, FluentIcon, FluentRoot,
    FluentTheme, FontMode,
};

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

/// Root view: theme switcher, font mode, and the button matrix.
struct Gallery {
    appearance: Appearance,
    clicks: usize,
    /// Keeps the OS appearance observer alive for the window's lifetime.
    _appearance_sub: Option<Subscription>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // A forced appearance (screenshots, CI baselines) must not be
        // overwritten by the OS observer, which fires immediately.
        let forced = std::env::var_os("FLUENT_GALLERY_APPEARANCE").is_some();
        let sub = if forced {
            None
        } else {
            Some(FluentTheme::follow_system(window))
        };
        Self {
            appearance: FluentTheme::global(cx).appearance(),
            clicks: 0,
            _appearance_sub: sub,
        }
    }

    fn apply(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        self.appearance = appearance;
        FluentTheme::global(cx)
            .with_appearance(appearance)
            .install(cx);
        cx.notify();
    }

    fn toggle_font(&mut self, cx: &mut Context<Self>) {
        let current = FluentTheme::global(cx);
        let next = match current.font_mode() {
            FontMode::SegoeCompat => FontMode::System,
            _ => FontMode::SegoeCompat,
        };
        current.with_font_mode(next).install(cx);
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
        for shape in [
            ButtonShape::Rounded,
            ButtonShape::Circular,
            ButtonShape::Square,
        ] {
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

        let mut icon_row = div().flex().items_center().gap(px(12.)).child(
            div()
                .w(px(96.))
                .text_color(t.color.neutral_foreground_3)
                .child("icon"),
        );
        for (icon, label) in [
            (FluentIcon::Add, "Add"),
            (FluentIcon::Search, "Search"),
            (FluentIcon::Settings, "Settings"),
        ] {
            icon_row = icon_row.child(
                Button::new(("icon", index))
                    .icon(icon)
                    .on_click(handler())
                    .child(label),
            );
            index += 1;
        }
        icon_row = icon_row.child(
            Button::new(("icon", index))
                .appearance(ButtonAppearance::Primary)
                .icon_after(FluentIcon::ChevronDown)
                .on_click(handler())
                .child("Options"),
        );
        index += 1;
        for icon in [
            FluentIcon::Add,
            FluentIcon::MoreHorizontal,
            FluentIcon::Dismiss,
        ] {
            icon_row = icon_row.child(
                Button::new(("icon-only", index))
                    .appearance(ButtonAppearance::Subtle)
                    .icon(icon)
                    .icon_only()
                    .on_click(handler()),
            );
            index += 1;
        }
        matrix = matrix.child(icon_row);

        let header = div()
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
                    .id("theme-hc")
                    .px(px(12.))
                    .py(px(5.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(t.color.neutral_stroke_1)
                    .cursor_pointer()
                    .on_click(
                        cx.listener(|this, _, _, cx| this.apply(Appearance::HighContrast, cx)),
                    )
                    .child("HC"),
            )
            .child(
                div()
                    .id("font-mode")
                    .px(px(12.))
                    .py(px(5.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(t.color.neutral_stroke_1)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_font(cx)))
                    .child("Font"),
            )
            .child(
                div()
                    .ml(px(8.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(format!(
                        "appearance: {:?}   clicks: {}   font: {} ({:?})",
                        theme.appearance(),
                        self.clicks,
                        theme.font_family(),
                        theme.font_mode()
                    )),
            );

        FluentRoot::new().flex().flex_col().child(header).child(
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

        if let Some(value) = std::env::var_os("FLUENT_GALLERY_APPEARANCE") {
            let appearance = match value.to_string_lossy().to_ascii_lowercase().as_str() {
                "dark" => Appearance::Dark,
                "hc" | "high-contrast" => Appearance::HighContrast,
                _ => Appearance::Light,
            };
            FluentTheme::for_appearance(appearance).install(cx);
        }

        let bounds = Bounds::centered(None, size(px(900.), px(640.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Fluent Gallery".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| Gallery::new(window, cx)),
            )
            .expect("failed to open window");

        #[cfg(feature = "screenshot")]
        if let Some(path) = std::env::var_os("FLUENT_GALLERY_SCREENSHOT") {
            let path = std::path::PathBuf::from(path);
            let executor = cx.background_executor().clone();
            cx.spawn(async move |cx| {
                // Give the first frames (layout, text shaping, SVG parse) time.
                executor.timer(Duration::from_millis(1200)).await;
                let captured =
                    cx.update(|cx| window.update(cx, |_, window, _| window.render_to_image()));
                match captured {
                    Ok(Ok(image)) => match image.save(&path) {
                        Ok(()) => eprintln!("screenshot written to {}", path.display()),
                        Err(error) => eprintln!("screenshot save failed: {error}"),
                    },
                    Ok(Err(error)) => eprintln!("render_to_image failed: {error:#}"),
                    Err(error) => eprintln!("window.update failed: {error:#}"),
                }
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
        #[cfg(not(feature = "screenshot"))]
        let _ = &window;

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
