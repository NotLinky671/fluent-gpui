//! Overlay acceptance fixture: one window, one gesture, one printed answer.
//!
//! `scripts/verify-overlays-windows.py` drives this with real pointer and
//! keyboard input. The page is deliberately plain: a dark backdrop so a panel or
//! a tooltip is detectable by pixel, and fixed positions so the script knows
//! where each target is; the coordinates are printed as `anchor` lines at start.
//!
//! Modes:
//! - `popover`: a controlled Popover with a button inside its panel.
//! - `tooltip`: two buttons with tooltips, one at each side of the window.
//! - `menu`: a MenuDropdown whose fourth row opens a submenu.
//! - `scroll`: a Popover whose trigger sits in a scroll container, so the panel
//!   has to follow the trigger when the wheel moves it.
use fluent_ui::gpui::{
    App, AppContext, Context, IntoElement, Render, ScrollHandle, Window, WindowOptions, div, px,
    rgb,
};
use fluent_ui::prelude::*;
use fluent_ui::{Button, FluentRoot, FluentTheme, Menu, MenuDropdown, MenuItem, Popover};

/// Window size in logical pixels; the script multiplies by the window scale.
const WIDTH: f32 = 900.;
const HEIGHT: f32 = 620.;

struct Probe {
    mode: String,
    scroll: ScrollHandle,
}

impl Probe {
    /// The script reads these lines; they are the only channel between the probe
    /// and the checks, so every gesture prints its own outcome.
    fn note(&self, line: String) {
        println!("{line}");
    }
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let theme = FluentTheme::global(cx);
        // Every overlay in this theme is a light surface with a border, so the
        // page stays dark in every mode and the script can find a surface by
        // brightness alone instead of trusting what the app says it drew.
        let backdrop = rgb(0x1b2430);

        let mut page = div()
            .relative()
            .size_full()
            .bg(backdrop)
            .text_color(theme.tokens().color.neutral_foreground_1);

        match self.mode.as_str() {
            "popover" => {
                let toggle = {
                    let view = view.clone();
                    move |open: bool, _: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |probe, cx| {
                            probe.note(format!("state popover open={open}"));
                            cx.notify();
                        });
                    }
                };
                let clicked = {
                    let view = view.clone();
                    move |_: &_, _: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |probe, _| {
                            probe.note("event child=clicked".to_string());
                        });
                        cx.refresh_windows();
                    }
                };
                page = page.child(
                    div()
                        .absolute()
                        .left(px(560.))
                        .top(px(120.))
                        .w(px(140.))
                        .h(px(36.))
                        .child(
                            Popover::new("probe-popover")
                                .trigger(
                                    Button::new("probe-popover-trigger")
                                        .accessibility_label("Open the probe popover")
                                        .child("Open panel"),
                                )
                                .content(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(8.))
                                        .child("Panel content")
                                        .child(
                                            Button::new("probe-popover-child")
                                                .accessibility_label("A control inside the panel")
                                                .child("Inside")
                                                .on_click(clicked),
                                        ),
                                )
                                // Uncontrolled on purpose: a raw Popover has no
                                // built-in toggle when `open` is supplied, and this
                                // probe is about hit testing, not caller code.
                                .default_open(false)
                                .on_open_change(toggle),
                        ),
                );
            }
            "tooltip" => {
                page = page
                    .child(
                        div()
                            .absolute()
                            .left(px(80.))
                            .top(px(120.))
                            .w(px(140.))
                            .h(px(36.))
                            .child(
                                Button::new("probe-tooltip-left")
                                    .accessibility_label("Left tooltip target")
                                    .child("Hover left")
                                    .tooltip("Tooltip on the left target"),
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(700.))
                            .top(px(120.))
                            .w(px(140.))
                            .h(px(36.))
                            .child(
                                Button::new("probe-tooltip-right")
                                    .accessibility_label("Right tooltip target")
                                    .child("Hover right")
                                    .tooltip("Tooltip on the right target"),
                            ),
                    );
            }
            "menu" => {
                let toggle = {
                    let view = view.clone();
                    move |open: bool, _: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |probe, cx| {
                            probe.note(format!("state menu open={open}"));
                            cx.notify();
                        });
                    }
                };
                let submenu = Menu::new("probe-submenu")
                    .item(MenuItem::new("probe-sub-1", "Sub one"))
                    .item(MenuItem::new("probe-sub-2", "Sub two"))
                    .item(MenuItem::new("probe-sub-3", "Sub three"));
                let menu = Menu::new("probe-menu")
                    .item(MenuItem::new("probe-row-1", "First"))
                    .item(MenuItem::new("probe-row-2", "Second"))
                    .item(MenuItem::new("probe-row-3", "Third"))
                    .item(MenuItem::new("probe-row-4", "More").submenu(submenu))
                    .item(MenuItem::new("probe-row-5", "Fifth"))
                    .item(MenuItem::new("probe-row-6", "Sixth"));
                page = page.child(
                    div()
                        .absolute()
                        .left(px(300.))
                        .top(px(120.))
                        .w(px(180.))
                        .h(px(36.))
                        .child(
                            MenuDropdown::new(
                                "probe-menu-dropdown",
                                Button::new("probe-menu-trigger")
                                    .accessibility_label("Open the probe menu")
                                    .child("Open menu"),
                                menu,
                            )
                            // Uncontrolled for the same reason as the popover:
                            // a supplied `open` makes the caller the only toggler.
                            .default_open(false)
                            .on_open_change(toggle),
                        ),
                );
            }
            "scroll" => {
                let toggle = {
                    let view = view.clone();
                    move |open: bool, _: &mut Window, cx: &mut App| {
                        let _ = view.update(cx, |probe, cx| {
                            probe.note(format!("state popover open={open}"));
                            cx.notify();
                        });
                    }
                };
                let mut column = div().flex().flex_col().gap(px(12.));
                for index in 0..12 {
                    let label = format!("Filler {index}");
                    if index == 2 {
                        column = column.child(
                            div().w(px(240.)).h(px(36.)).child(
                                Popover::new("probe-scroll-popover")
                                    .trigger(
                                        Button::new("probe-scroll-trigger")
                                            .accessibility_label("Scrolled popover trigger")
                                            .child("Scrolled trigger"),
                                    )
                                    .content(div().child("Panel over the scroller"))
                                    .default_open(false)
                                    .on_open_change(toggle.clone()),
                            ),
                        );
                    } else {
                        column = column.child(div().h(px(36.)).child(label));
                    }
                }
                page = page.child(
                    div()
                        .id("probe-scroll")
                        .absolute()
                        .left(px(80.))
                        .top(px(100.))
                        .w(px(320.))
                        .h(px(240.))
                        .overflow_y_scroll()
                        .track_scroll(&self.scroll)
                        .child(column),
                );
            }
            _ => {}
        }

        FluentRoot::new().child(page)
    }
}

/// The centre of each target in logical coordinates, printed for the script.
fn anchors(mode: &str) {
    match mode {
        "popover" => println!("anchor trigger=630,138"),
        "tooltip" => {
            println!("anchor left=150,138");
            println!("anchor right=770,138");
        }
        "menu" => {
            println!("anchor trigger=390,138");
            // Rows start under the trigger with the surface padding and the
            // 2px gap between rows; the fourth row sits at 3 × 34 + 16.
            println!("anchor row4=390,262");
        }
        // The trigger is the third child of the scrolling column: two 36px fillers
        // plus their 12px gaps put it at y 196..232 inside a column at (80, 100).
        "scroll" => println!("anchor trigger=200,214"),
        _ => println!("anchor none=0,0"),
    }
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "popover".into());
    println!("mode={mode}");
    anchors(&mode);
    println!("window={WIDTH},{HEIGHT}");
    match mode.as_str() {
        "tooltip" => println!("backdrop=243,243,243"),
        _ => println!("backdrop=27,36,48"),
    }

    fluent_ui::application().run(move |cx: &mut App| {
        fluent_ui::init(cx);
        let mode = mode.clone();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(fluent_ui::gpui::WindowBounds::Windowed(
                    fluent_ui::gpui::Bounds::new(
                        fluent_ui::gpui::point(px(120.), px(120.)),
                        fluent_ui::gpui::size(px(WIDTH), px(HEIGHT)),
                    ),
                )),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| Probe {
                    mode,
                    scroll: ScrollHandle::new(),
                })
            },
        )
        .expect("open the overlay probe window");
    });
}
