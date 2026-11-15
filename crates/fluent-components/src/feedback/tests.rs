//! Behaviour checks for the feedback components that need a window.
//!
//! The spinner and the indeterminate bar both move. These tests read the
//! painted result after advancing the test clock, because "the animation exists"
//! is not the same claim as "the pixels change": a phase that never advances
//! paints one frame forever, and only the second assertion catches that.

use std::time::Duration;

use crate::feedback::{ProgressBar, Spinner, SpinnerSize};
use gpui::{
    AppContext as _, Context, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext,
    Window, WindowHandle, div, size,
};

struct FeedbackView {
    reduced: bool,
}

impl Render for FeedbackView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.reduced {
            cx.set_reduce_motion(true);
        }
        div()
            .flex()
            .flex_col()
            .gap(gpui::px(8.))
            .child(Spinner::new().size(SpinnerSize::Tiny))
            .child(Spinner::new().size(SpinnerSize::ExtraSmall))
            .child(Spinner::new().size(SpinnerSize::Small).label("Loading"))
            .child(Spinner::new().size(SpinnerSize::Medium))
            .child(
                Spinner::new()
                    .size(SpinnerSize::Medium)
                    .label("Uploading files"),
            )
            .child(
                Spinner::new()
                    .size(SpinnerSize::Medium)
                    .label("Downloading files"),
            )
            .child(ProgressBar::new(40.).indeterminate())
            .child(ProgressBar::new(40.))
    }
}

fn open_feedback(cx: &mut TestAppContext, reduced: bool) -> WindowHandle<FeedbackView> {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(move |cx| cx.set_reduce_motion(reduced));
    let handle = cx.open_window(size(gpui::px(300.), gpui::px(300.)), move |_, _| {
        FeedbackView { reduced }
    });
    cx.run_until_parked();
    draw(cx, handle);
    handle
}

fn draw(cx: &mut TestAppContext, handle: WindowHandle<FeedbackView>) {
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

/// Every painted path vertex, in paint order.
///
/// The spinner is drawn as two stroked paths (track and indicator); a frame that
/// advanced its phase moves the indicator's vertices.
fn path_vertices(cx: &mut TestAppContext, handle: WindowHandle<FeedbackView>) -> Vec<(f32, f32)> {
    cx.update_window(handle.into(), |_, window, _| {
        window
            .painted_paths()
            .iter()
            .flat_map(|path| {
                path.vertices
                    .iter()
                    .map(|vertex| (vertex.xy_position.x.0, vertex.xy_position.y.0))
            })
            .collect()
    })
    .unwrap()
}

/// The left edge of every opaque quad taller than 2px, which is where the
/// indeterminate segment sits on its track.
fn bar_edges(cx: &mut TestAppContext, handle: WindowHandle<FeedbackView>) -> Vec<f32> {
    cx.update_window(handle.into(), |_, window, _| {
        window
            .painted_quads()
            .into_iter()
            .filter(|quad| {
                quad.background.as_solid().is_some()
                    && (quad.bounds.size.height.0 / window.scale_factor()) > 2.0
                    && (quad.bounds.size.height.0 / window.scale_factor()) < 6.0
            })
            .map(|quad| quad.bounds.origin.x.0 / window.scale_factor())
            .collect()
    })
    .unwrap()
}

fn advance(cx: &mut TestAppContext, millis: u64, handle: WindowHandle<FeedbackView>) {
    cx.executor().advance_clock(Duration::from_millis(millis));
    draw(cx, handle);
}

#[gpui::test]
fn spinner_renders_every_size(cx: &mut TestAppContext) {
    open_feedback(cx, false);
}

#[gpui::test]
fn spinner_renders_every_size_under_reduced_motion(cx: &mut TestAppContext) {
    open_feedback(cx, true);
}

#[gpui::test]
fn the_spinner_arc_moves_as_the_clock_advances(cx: &mut TestAppContext) {
    let handle = open_feedback(cx, false);
    let before = path_vertices(cx, handle);
    assert!(!before.is_empty(), "the spinner must paint a ring");

    advance(cx, 250, handle);
    let after = path_vertices(cx, handle);

    assert_ne!(
        before, after,
        "a quarter of the 1.5s period must move the indicator, but the painted paths did not change"
    );
}

#[gpui::test]
fn the_indeterminate_bar_moves_as_the_clock_advances(cx: &mut TestAppContext) {
    let handle = open_feedback(cx, false);
    let before = bar_edges(cx, handle);
    assert!(!before.is_empty(), "the bar must paint a track");

    advance(cx, 500, handle);
    let after = bar_edges(cx, handle);

    assert_ne!(
        before, after,
        "a quarter of the 2s period must move the segment, but the painted bars did not change"
    );
}

#[gpui::test]
fn the_reduced_motion_spinner_stays_still(cx: &mut TestAppContext) {
    let handle = open_feedback(cx, true);
    let before = path_vertices(cx, handle);

    advance(cx, 500, handle);
    let after = path_vertices(cx, handle);

    assert_eq!(
        before, after,
        "reduced motion must not move the ring when the clock advances"
    );
}

#[gpui::test]
fn the_reduced_motion_bar_stays_still(cx: &mut TestAppContext) {
    let handle = open_feedback(cx, true);
    let before = bar_edges(cx, handle);
    assert!(
        !before.is_empty(),
        "the parked segment must still be painted"
    );

    advance(cx, 500, handle);
    let after = bar_edges(cx, handle);

    assert_eq!(
        before, after,
        "reduced motion must not sweep the segment when the clock advances"
    );
}
