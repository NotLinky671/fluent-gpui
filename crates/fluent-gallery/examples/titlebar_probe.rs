//! Native titlebar acceptance fixture. `-- veto` rejects the first close request
//! and accepts the next; `-- two` checks that one window can close independently.
use fluent_ui::gpui::{
    App, AppContext, Context, ImageFormat, IntoElement, Render, Window, WindowOptions, div, px,
};
use fluent_ui::prelude::*;
use fluent_ui::{Button, Dialog, FluentRoot, TitleBar};
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

struct Probe {
    dialog: bool,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        FluentRoot::new()
            .flex()
            .flex_col()
            .child(
                TitleBar::new("probe-titlebar", "Fluent Titlebar Probe")
                    .logo_bytes(ImageFormat::Png, logo_png()),
            )
            .child(
                div().p(px(24.)).child(
                    Button::new("open-dialog")
                        .accessibility_label("Open dialog")
                        .child("Open dialog")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dialog = true;
                            println!("dialog-open");
                            cx.notify();
                        })),
                ),
            )
            .child(
                Dialog::new("probe-dialog")
                    .open(self.dialog)
                    .title("Caption buttons remain usable")
                    .child("Use the caption controls above this scrim.")
                    .on_open_change(move |open, _, _, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.dialog = open;
                            cx.notify();
                        });
                    }),
            )
    }
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    fluent_ui::application().run(move |cx: &mut App| {
        fluent_ui::init(cx);
        for index in 0..if mode == "two" { 2 } else { 1 } {
            let allow = Rc::new(Cell::new(mode != "veto"));
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
                    cx.new(|_| Probe { dialog: false })
                },
            )
            .expect("open native titlebar probe");
        }
        cx.activate(true);
    });
}
