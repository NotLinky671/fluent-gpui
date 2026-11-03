//! Behaviour checks for the feedback components that need a window.
//!
//! These tests only prove the elements render and honour the reduce-motion
//! flag; actual pixels and the real OS reduced-motion setting are not covered
//! here (see docs/validation/spinner-progress-2026-11-03.md).

use crate::feedback::{ProgressBar, Spinner, SpinnerSize};
use gpui::{
    Context, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext, Window, div,
    size,
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
            .child(Spinner::new().size(SpinnerSize::Small))
            .child(Spinner::new().size(SpinnerSize::Medium).label("Loading"))
            .child(Spinner::new().size(SpinnerSize::Large))
            .child(ProgressBar::new(40.).indeterminate())
            .child(ProgressBar::new(40.))
    }
}

fn open_feedback(cx: &mut TestAppContext, reduced: bool) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(move |cx| cx.set_reduce_motion(reduced));
    let _handle = cx.open_window(size(gpui::px(300.), gpui::px(300.)), move |_, _| {
        FeedbackView { reduced }
    });
    cx.run_until_parked();
}

#[gpui::test]
fn spinner_renders_every_size(cx: &mut TestAppContext) {
    open_feedback(cx, false);
}

#[gpui::test]
fn spinner_renders_every_size_under_reduced_motion(cx: &mut TestAppContext) {
    open_feedback(cx, true);
}
