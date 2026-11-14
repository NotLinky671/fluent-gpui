use super::*;
use gpui::{
    AppContext, Context, PlatformInput, Render, ScrollDelta, ScrollEvent, ScrollHandle,
    TestAppContext, WindowHandle, canvas, size,
};

#[test]
fn panel_keeps_the_trigger_gap_inside_the_viewport() {
    let trigger = Bounds::new(point(px(20.), px(100.)), size(px(100.), px(30.)));
    assert_eq!(
        panel_origin(
            trigger,
            size(px(160.), px(64.)),
            size(px(800.), px(600.)),
            px(8.),
            px(4.)
        ),
        point(px(20.), px(134.)),
    );
}

#[test]
fn panel_clamps_continuously_at_both_window_edges() {
    let panel = size(px(160.), px(64.));
    let viewport = size(px(800.), px(600.));
    // Include the inset band *inside* the viewport, not only true overflow.
    for desired in [
        -1., 0., 4., 7., 8., 9., 527., 528., 529., 535., 536., 537., 631., 632., 633., 639., 640.,
        641.,
    ] {
        let trigger = Bounds::new(
            point(px(desired), px(desired - 34.)),
            size(px(100.), px(30.)),
        );
        let actual = panel_origin(trigger, panel, viewport, px(8.), px(4.));
        assert_eq!(actual.x, px(desired.clamp(8., 632.)));
        assert_eq!(actual.y, px(desired.clamp(8., 528.)));
    }
    // Crossing the old bottom-overflow threshold no longer jumps backwards 8px.
    let positions: Vec<_> = (520..545)
        .map(|y| {
            let trigger = Bounds::new(point(px(20.), px(y as f32 - 34.)), size(px(100.), px(30.)));
            panel_origin(trigger, panel, viewport, px(8.), px(4.)).y
        })
        .collect();
    for pair in positions.windows(2) {
        assert!(pair[1] >= pair[0]);
        assert!(pair[1] - pair[0] <= px(1.));
    }
}

#[test]
fn oversized_panel_aligns_to_the_inset_without_panicking() {
    let trigger = Bounds::new(point(px(100.), px(100.)), size(px(100.), px(30.)));
    for viewport in [size(px(80.), px(40.)), size(px(0.), px(0.))] {
        assert_eq!(
            panel_origin(trigger, size(px(160.), px(64.)), viewport, px(8.), px(4.)),
            point(px(8.), px(8.)),
        );
    }
}

#[derive(Clone, Default)]
struct Measurements {
    trigger: TriggerBounds,
    content: TriggerBounds,
    content_mask: TriggerBounds,
}

impl Measurements {
    fn assert_anchored(&self, window: &Window) {
        let trigger = self
            .trigger
            .get()
            .expect("trigger must be measured this frame");
        let content = self
            .content
            .get()
            .expect("open panel must be measured this frame");
        // The content is inset by the surface's 12px padding on each side.
        let expected = window.pixel_snap_point(point(
            trigger.left().max(px(8.)) + px(12.),
            trigger.bottom() + px(16.),
        ));
        let tolerance = px(0.01);
        assert!(
            (content.left() - expected.x).abs() < tolerance,
            "{trigger:?} -> {content:?}, expected {expected:?}"
        );
        assert!(
            (content.top() - expected.y).abs() < tolerance,
            "{trigger:?} -> {content:?}, expected {expected:?}"
        );
    }

    fn clear(&self) {
        self.trigger.set(None);
        self.content.set(None);
        self.content_mask.set(None);
    }
}

struct ScrollTestView {
    measurements: Measurements,
    outer: ScrollHandle,
    inner: ScrollHandle,
    open: bool,
}

impl Render for ScrollTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let trigger_bounds = self.measurements.trigger.clone();
        let content_bounds = self.measurements.content.clone();
        let content_mask = self.measurements.content_mask.clone();
        div().size_full().child(
            div()
                .id("outer-scroll")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.outer)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .w_full()
                        .h(px(1400.))
                        .child(div().h(px(100.)).flex_shrink_0())
                        .child(
                            div()
                                .id("inner-scroll")
                                .w(px(320.))
                                .h(px(110.))
                                .flex_shrink_0()
                                .overflow_y_scroll()
                                .track_scroll(&self.inner)
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .w_full()
                                        .h(px(600.))
                                        .child(div().h(px(60.)).flex_shrink_0())
                                        .child(
                                            Popover::new("scroll-test-popover")
                                                .open(self.open)
                                                .trigger(
                                                    canvas(
                                                        move |bounds, _, _| {
                                                            trigger_bounds.set(Some(bounds))
                                                        },
                                                        |_, (), _, _| {},
                                                    )
                                                    .w(px(100.))
                                                    .h(px(30.)),
                                                )
                                                .content(
                                                    canvas(
                                                        move |bounds, window, _| {
                                                            content_bounds.set(Some(bounds));
                                                            content_mask.set(Some(
                                                                window.content_mask().bounds,
                                                            ));
                                                        },
                                                        |_, (), _, _| {},
                                                    )
                                                    .w(px(160.))
                                                    .h(px(40.)),
                                                ),
                                        ),
                                ),
                        ),
                ),
        )
    }
}

fn test_window(cx: &mut TestAppContext) -> (WindowHandle<ScrollTestView>, Measurements) {
    let measurements = Measurements::default();
    let probes = measurements.clone();
    let window = cx.open_window(size(px(800.), px(600.)), move |_, _| ScrollTestView {
        measurements: probes,
        outer: ScrollHandle::new(),
        inner: ScrollHandle::new(),
        open: true,
    });
    (window, measurements)
}

#[gpui::test]
fn panel_tracks_each_scroll_event_in_the_same_frame(cx: &mut TestAppContext) {
    let (handle, measurements) = test_window(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        measurements.clear();
        window.refresh();
        window.draw(cx).clear(cx);
        // Also catches the initial frame appearing at (0, 0) before measuring.
        measurements.assert_anchored(window);
        let mut previous = measurements.trigger.get().unwrap();
        for delta in [-17., -9., 6., -13., 11.] {
            measurements.clear();
            window.dispatch_event(
                PlatformInput::Scroll(ScrollEvent {
                    // Outside the nested scroller and floating panel.
                    position: point(px(600.), px(300.)),
                    delta: ScrollDelta::Pixels(point(px(0.), px(delta))),
                    ..Default::default()
                }),
                cx,
            );
            // Notify effects have not flushed inside this update. Force exactly
            // one draw and assert *before* any next-frame callback can catch up.
            window.refresh();
            window.draw(cx).clear(cx);
            measurements.assert_anchored(window);
            let current = measurements.trigger.get().unwrap();
            assert!(
                current.top() != previous.top(),
                "scroll event must actually move the trigger"
            );
            previous = current;
        }
    })
    .unwrap();
}

#[gpui::test]
fn panel_tracks_nested_fractional_scroll_and_escapes_clipping(cx: &mut TestAppContext) {
    let (handle, measurements) = test_window(cx);
    cx.update_window(handle.into(), |root, window, cx| {
        let view = root.downcast::<ScrollTestView>().unwrap();
        let outer_scroll = view.read(cx).outer.clone();
        let inner_scroll = view.read(cx).inner.clone();
        for scale in [1., 1.25, 1.5, 2.] {
            window.set_scale_factor(scale);
            for (outer, inner) in [(0., 0.), (-10.4, -7.2), (-20.8, -13.6), (-5.6, -2.4)] {
                outer_scroll.set_offset(point(px(0.), px(outer)));
                inner_scroll.set_offset(point(px(0.), px(inner)));
                measurements.clear();
                window.refresh();
                window.draw(cx).clear(cx);
                measurements.assert_anchored(window);
                let content = measurements.content.get().unwrap();
                let mask = measurements.content_mask.get().unwrap();
                // The surface extends below the inner scroller (bottom <=210),
                // but its clipping mask remains the window, not that scroller.
                assert!(content.bottom() > px(210.) + px(outer));
                assert!(mask.contains(&content.bottom_right()));
            }
        }
    })
    .unwrap();
}

#[gpui::test]
fn reopened_panel_uses_the_current_trigger_position(cx: &mut TestAppContext) {
    let (handle, measurements) = test_window(cx);
    cx.update_window(handle.into(), |root, window, cx| {
        let view = root.downcast::<ScrollTestView>().unwrap();
        for (open, offset) in [(true, 0.), (false, -23.), (true, -41.)] {
            view.update(cx, |view, _| {
                view.open = open;
                view.outer.set_offset(point(px(0.), px(offset)));
            });
            measurements.clear();
            window.refresh();
            window.draw(cx).clear(cx);
            if open {
                measurements.assert_anchored(window);
            } else {
                assert!(measurements.trigger.get().is_some());
                assert!(measurements.content.get().is_none());
            }
        }
    })
    .unwrap();
}
