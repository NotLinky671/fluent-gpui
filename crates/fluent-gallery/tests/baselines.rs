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
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

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
    let log = scratch_dir().join("fixtures.stdout.log");
    let mut command = gallery();
    command.env("FLUENT_GALLERY_LIST_FIXTURES", "1");
    let status = run(&mut command, &log, Duration::from_secs(30)).expect("list fixtures");
    assert!(
        status.success(),
        "listing fixtures failed; see {}",
        log.display()
    );
    std::fs::read_to_string(log)
        .expect("read fixture list")
        .lines()
        .filter_map(|line| line.split('\t').next())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

fn render(name: &str, appearance: &str, out: &Path) {
    remove_previous(out).expect("remove previous screenshot before launching gallery");
    let log = out.with_extension("stdout.log");
    let mut command = gallery();
    command
        .env("FLUENT_GALLERY_FIXTURE", name)
        .env("FLUENT_GALLERY_APPEARANCE", appearance)
        .env("FLUENT_GALLERY_SCREENSHOT", out);
    let status = run(&mut command, &log, Duration::from_secs(30))
        .unwrap_or_else(|error| panic!("rendering {name}: {error}; see {}", log.display()));
    assert!(status.success(), "rendering {name} failed");
    assert!(out.exists(), "rendering {name} produced no screenshot");
}

fn scratch_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("baselines")
}

fn remove_previous(out: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(out) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Reap the child even if waiting panics or times out. File-backed logs avoid
/// blocking on full stdout/stderr pipes while a child is being waited on.
struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run(command: &mut Command, log: &Path, timeout: Duration) -> std::io::Result<ExitStatus> {
    command
        .stdout(Stdio::from(std::fs::File::create(log)?))
        .stderr(Stdio::from(std::fs::File::create(
            log.with_extension("stderr.log"),
        )?));
    let mut child = ChildGuard(command.spawn()?);
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.0.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "gallery process exceeded its deadline",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

// This test doubles as the deliberately hanging child in the lifecycle test.
#[test]
fn lifecycle_child() {
    if std::env::var_os("FLUENT_LIFECYCLE_HANG").is_some() {
        loop {
            std::thread::park();
        }
    }
}

#[test]
fn a_hung_child_times_out_and_a_previous_actual_is_removed() {
    let scratch = scratch_dir().join("lifecycle");
    std::fs::create_dir_all(&scratch).unwrap();
    let actual = scratch.join("previous.actual.png");
    std::fs::write(&actual, b"old run").unwrap();
    remove_previous(&actual).unwrap();
    assert!(
        !actual.exists(),
        "an old actual must not count as a fresh capture"
    );
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .arg("--exact")
        .arg("lifecycle_child")
        .env("FLUENT_LIFECYCLE_HANG", "1");
    let start = Instant::now();
    let error = run(
        &mut child,
        &scratch.join("hung.stdout.log"),
        Duration::from_millis(100),
    )
    .expect_err("the child cannot finish on its own");
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "timeout must include bounded child cleanup"
    );
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
    let scratch = scratch_dir();
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
