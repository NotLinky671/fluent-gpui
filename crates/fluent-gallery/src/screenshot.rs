//! Capture static fixtures after three consecutive rendered images agree.
//! This observes rendered output, with a deadline; it does not prove interaction
//! correctness or that external resources will never change later.
use fluent_ui::gpui::Window;
use image::RgbaImage;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub(super) fn capture(path: PathBuf, window: &mut Window) {
    sample(path, None, 0, 0, Instant::now(), window);
}

fn sample(
    path: PathBuf,
    previous: Option<RgbaImage>,
    equal: usize,
    frames: usize,
    started: Instant,
    window: &mut Window,
) {
    window.on_next_frame(move |window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        let image = match window.render_to_image() {
            Ok(image) => image,
            Err(error) => {
                eprintln!("render_to_image failed: {error:#}");
                cx.quit();
                return;
            }
        };
        let equal = if previous.as_ref() == Some(&image) {
            equal + 1
        } else {
            0
        };
        if equal >= 2 {
            match image.save(&path) {
                Ok(()) => eprintln!(
                    "screenshot written to {} after {} frames",
                    path.display(),
                    frames + 1
                ),
                Err(error) => eprintln!("screenshot save failed: {error}"),
            }
            cx.quit();
        } else if frames >= 119 || started.elapsed() >= Duration::from_secs(20) {
            eprintln!("screenshot did not stabilize within the frame/time budget");
            cx.quit();
        } else {
            sample(path, Some(image), equal, frames + 1, started, window);
        }
    });
}
