//! Native titlebar acceptance fixture.
//!
//! Modes:
//! - no argument: one window, closable.
//! - `veto`: rejects the first close request and accepts the next.
//! - `two`: checks that one window can close independently.
//! - `backdrop`: asks for Mica, revokes it, asks again, and reports what DWM
//!   answered at each step. The page paints the layer the spec resolves, so what
//!   is on screen is the material behind that layer, or a solid colour if the
//!   request was refused.
//! - `backdrop-fail`: the same page with an injected refusal, to prove the solid
//!   fallback renders opaque instead of leaving a transparent hole.
use fluent_ui::backdrop::{BackdropOutcome, BackdropPolicy, WindowBackdrop, apply, reported};
use fluent_ui::gpui::{
    App, AppContext, Context, Hsla, ImageFormat, IntoElement, Render, Window, WindowOptions, div,
    px,
};
use fluent_ui::prelude::*;
use fluent_ui::{Button, Dialog, FluentRoot, FluentTheme, TitleBar};
use std::{cell::Cell, rc::Rc};

/// A 32×32 blue tile with a white bar, standing in for an application logo, so
/// the fixture exercises the bitmap path without shipping an asset.
fn logo_png() -> Vec<u8> {
    let mut pixels = image::RgbaImage::new(32, 32);
    for (x, y, pixel) in pixels.enumerate_pixels_mut() {
        let edge = 3;
        let tile = x >= edge && y >= edge && x < 32 - edge && y < 32 - edge;
        let bar = (14..18).contains(&x) && (8..24).contains(&y);
        *pixel = if tile && bar {
            image::Rgba([255, 255, 255, 255])
        } else if tile {
            image::Rgba([0, 90, 158, 255])
        } else {
            image::Rgba([0, 0, 0, 0])
        };
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(pixels)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("encode the fixture logo");
    bytes.into_inner()
}

/// Runs the request → revoke → request sequence and returns the policy the page
/// should paint. Every step prints, because the point of this fixture is the
/// answer from DWM, not the picture alone.
fn probe_backdrop(
    window: &Window,
    cx: &App,
    steps: &[WindowBackdrop],
    request: WindowBackdrop,
    inject_failure: bool,
) -> BackdropPolicy {
    println!("backdrop:platform={}", std::env::consts::OS);
    println!("backdrop:scale={}", window.scale_factor());

    let mut last = (request, BackdropOutcome::Unsupported("nothing was asked"));
    for step in steps {
        let outcome = apply(window, *step);
        let reported_now = reported(window);
        // The DWM contract itself, written out here rather than read back from
        // the adapter: were the adapter to map a request to the wrong value, it
        // would compare that value with itself and call the result a success.
        let expected = match *step {
            WindowBackdrop::Solid => 1,
            WindowBackdrop::Mica => 2,
            WindowBackdrop::MicaAlt => 4,
        };
        let verdict = if reported_now == Some(expected) {
            "ok"
        } else {
            "WRONG"
        };
        println!(
            "backdrop:request={step:?} result={outcome:?} expected={expected} reported={reported_now:?} {verdict}"
        );
        last = (*step, outcome);
    }

    // The last step's own verdict, not a second guess: `apply` already compares
    // what was asked with what the compositor reports back.
    let outcome = if inject_failure {
        println!("backdrop:injected=failed");
        BackdropOutcome::Failed(-2147467259)
    } else {
        last.1
    };

    let theme = FluentTheme::global(cx);
    let mut policy = fluent_ui::backdrop::resolve(request, outcome, false, &theme);
    // `FLUENT_PROBE_LAYER_ALPHA` overrides the layer so the material can be
    // observed on its own: at 0 the window shows nothing but what DWM puts
    // behind it, which is the only way to see whether Mica is really there.
    if let Ok(alpha) = std::env::var("FLUENT_PROBE_LAYER_ALPHA")
        .unwrap_or_default()
        .parse::<f32>()
    {
        policy.layer.a = alpha;
        println!("backdrop:layer_override={alpha}");
    }
    println!(
        "backdrop:policy=material:{} alpha:{} window:{:?}",
        policy.material, policy.layer.a, policy.window
    );
    policy
}

struct Probe {
    dialog: bool,
    /// Set in the backdrop modes: the layer the page paints. `None` uses the
    /// ordinary opaque root.
    layer: Option<Hsla>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let theme = FluentTheme::global(cx);
        let tokens = theme.tokens();
        let titlebar = TitleBar::new("probe-titlebar", "Fluent Titlebar Probe")
            .logo_bytes(ImageFormat::Png, logo_png());
        let body = div().p(px(24.)).child(
            Button::new("open-dialog")
                .accessibility_label("Open dialog")
                .child("Open dialog")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dialog = true;
                    println!("dialog-open");
                    cx.notify();
                })),
        );

        let dialog = Dialog::new("probe-dialog")
            .open(self.dialog)
            .title("Caption buttons remain usable")
            .child("Use the caption controls above this scrim.")
            .on_open_change(move |open, _, _, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.dialog = open;
                    cx.notify();
                });
            });

        match self.layer {
            // The backdrop page deliberately skips `FluentRoot`: its opaque
            // background is exactly what would hide the material.
            Some(layer) => div()
                .size_full()
                .bg(layer)
                .text_color(tokens.color.neutral_foreground_1)
                .font_family(theme.font_family().clone())
                .flex()
                .flex_col()
                .child(titlebar)
                .child(body)
                .child(dialog)
                .into_any_element(),
            None => FluentRoot::new()
                .flex()
                .flex_col()
                .child(titlebar)
                .child(body)
                .child(dialog)
                .into_any_element(),
        }
    }
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let backdrop_mode = matches!(
        mode.as_str(),
        "backdrop" | "backdrop-solid" | "backdrop-fail"
    );
    fluent_ui::application().run(move |cx: &mut App| {
        fluent_ui::init(cx);
        for index in 0..if mode == "two" { 2 } else { 1 } {
            let allow = Rc::new(Cell::new(mode != "veto"));
            let mode = mode.clone();
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(fluent_ui::gpui::WindowBounds::centered(
                        fluent_ui::gpui::size(px(600.), px(320.)),
                        cx,
                    )),
                    ..TitleBar::window_options(format!("Fluent Titlebar Probe {index}"))
                },
                |window, cx| {
                    window.on_window_should_close(cx, move |_, _| {
                        let accepted = allow.replace(true);
                        println!("should-close:{index}:{accepted}");
                        accepted
                    });
                    let layer = backdrop_mode.then(|| {
                        let (steps, request, inject): (&[WindowBackdrop], WindowBackdrop, bool) =
                            match mode.as_str() {
                                // Ask, revoke, ask again: one enable and one revoke,
                                // with DWM's answer printed at each step.
                                "backdrop" => (
                                    &[
                                        WindowBackdrop::Mica,
                                        WindowBackdrop::Solid,
                                        WindowBackdrop::Mica,
                                    ],
                                    WindowBackdrop::Mica,
                                    false,
                                ),
                                "backdrop-solid" => {
                                    (&[WindowBackdrop::Solid], WindowBackdrop::Solid, false)
                                }
                                // A refusal with the window already solid: the page
                                // must stay opaque instead of turning translucent.
                                _ => (&[WindowBackdrop::Solid], WindowBackdrop::Mica, true),
                            };
                        probe_backdrop(window, cx, steps, request, inject).layer
                    });
                    cx.new(|_| Probe {
                        dialog: false,
                        layer,
                    })
                },
            )
            .expect("open native titlebar probe");
        }
        cx.activate(true);
    });
}
