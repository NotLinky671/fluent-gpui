//! Pixel baselines: render every fixture and compare it against a stored PNG.
//!
//! Run with the `screenshot` feature, which pulls in gpui's test-support:
//!
//! ```text
//! cargo test -p fluent-gallery --features screenshot --test baselines
//! ```
//!
//! `FLUENT_BASELINE_UPDATE=1` rewrites the baselines instead of comparing, so an
//! intentional visual change shows up as a PNG diff for review.
//!
//! Baseline layout is per platform: font rasterisation and anti-aliasing differ
//! between Windows, macOS and Linux, so a baseline captured on one is not
//! meaningful on another.
#![cfg(feature = "screenshot")]

use std::path::{Path, PathBuf};
use std::process::Command;

use image::{Rgba, RgbaImage};

/// How much a single channel may drift before the pixel counts as different.
/// Text anti-aliasing moves edge pixels by a level or two on every run.
const CHANNEL_TOLERANCE: i32 = 3;

/// Fraction of pixels allowed to differ. Kept tiny: this absorbs font
/// rasterisation noise, not layout changes.
const MAX_DIFF_RATIO: f64 = 0.001;

fn gallery() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fluent-gallery"))
}

fn baseline_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("baselines")
        .join(std::env::consts::OS)
}

fn fixture_names() -> Vec<String> {
    let output = gallery()
        .env("FLUENT_GALLERY_LIST_FIXTURES", "1")
        .output()
        .expect("failed to run the gallery to list fixtures");
    assert!(output.status.success(), "listing fixtures failed");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split('\t').next())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

fn render(name: &str, appearance: &str, out: &Path) {
    let status = gallery()
        .env("FLUENT_GALLERY_FIXTURE", name)
        .env("FLUENT_GALLERY_APPEARANCE", appearance)
        .env("FLUENT_GALLERY_SCREENSHOT", out)
        .status()
        .expect("failed to run the gallery");
    assert!(status.success(), "rendering {name} failed");
    assert!(out.exists(), "rendering {name} produced no screenshot");
}

fn load(path: &Path) -> RgbaImage {
    image::open(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
        .to_rgba8()
}

/// Counts differing pixels and paints them red in a copy of the actual image.
fn diff(actual: &RgbaImage, baseline: &RgbaImage) -> (usize, RgbaImage) {
    let mut marked = actual.clone();
    let mut count = 0;
    for (x, y, pixel) in actual.enumerate_pixels() {
        let expected = baseline.get_pixel(x, y);
        let differs = pixel
            .0
            .iter()
            .zip(expected.0.iter())
            .any(|(a, b)| (*a as i32 - *b as i32).abs() > CHANNEL_TOLERANCE);
        if differs {
            count += 1;
            marked.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        }
    }
    (count, marked)
}

#[test]
fn fixtures_match_baselines() {
    let name_appearance = "light";
    let update = std::env::var_os("FLUENT_BASELINE_UPDATE").is_some();
    let dir = baseline_dir();
    let scratch = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("baselines");
    std::fs::create_dir_all(&scratch).expect("cannot create target/baselines");
    if update {
        std::fs::create_dir_all(&dir).expect("cannot create the baseline directory");
    }

    let mut failures = Vec::new();
    let names = fixture_names();
    assert!(!names.is_empty(), "no fixtures were reported");

    for name in names {
        let baseline_path = dir.join(format!("{name_appearance}-{name}.png"));
        let actual_path = scratch.join(format!("{name}.actual.png"));
        render(&name, name_appearance, &actual_path);
        let actual = load(&actual_path);

        if update {
            std::fs::copy(&actual_path, &baseline_path).expect("cannot write baseline");
            println!("updated {}", baseline_path.display());
            continue;
        }

        if !baseline_path.exists() {
            failures.push(format!(
                "{name}: missing baseline {} (run with FLUENT_BASELINE_UPDATE=1)",
                baseline_path.display()
            ));
            continue;
        }

        let baseline = load(&baseline_path);
        if baseline.dimensions() != actual.dimensions() {
            failures.push(format!(
                "{name}: size changed {:?} -> {:?}",
                baseline.dimensions(),
                actual.dimensions()
            ));
            continue;
        }

        let (count, marked) = diff(&actual, &baseline);
        let total = (actual.width() * actual.height()) as f64;
        let ratio = count as f64 / total;
        if ratio > MAX_DIFF_RATIO {
            let diff_path = scratch.join(format!("{name}.diff.png"));
            marked
                .save(&diff_path)
                .expect("cannot write the diff image");
            failures.push(format!(
                "{name}: {count} px differ ({:.3}%)  actual={}  diff={}",
                ratio * 100.0,
                actual_path.display(),
                diff_path.display()
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} fixture(s) drifted from their baseline:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}
