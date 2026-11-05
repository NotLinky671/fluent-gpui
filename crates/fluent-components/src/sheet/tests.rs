//! Behaviour tests for `Sheet`.
//!
//! The tests drive a real window because the promises under test are hit-test
//! and dispatch results: which edge the panel occupies, whether a press on the
//! scrim reaches the close path, whether Tab can escape the trap, and when
//! focus returns during the exit animation. Drawn bounds are recorded from the
//! body element's prepaint so the slide can be asserted frame by frame without
//! a pixel baseline.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Bounds, Context, FocusHandle, InputEvent as _, KeyBinding, KeyDownEvent, Keystroke,
    Modifiers, MouseButton, MouseDownEvent, PlatformInput, Render, TestAppContext, WindowHandle,
    canvas, point, size,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// What the harness recorded.
#[derive(Clone, Default)]
struct Log {
    /// Requested open states, in order.
    changes: Rc<RefCell<Vec<bool>>>,
    /// Bounds of the body element, rewritten on every prepaint.
    body: Rc<Cell<Option<Bounds<Pixels>>>>,
}

#[derive(Clone)]
struct Handles {
    behind: FocusHandle,
    inside_a: FocusHandle,
    inside_b: FocusHandle,
}

struct SheetView {
    side: SheetSide,
    /// `Some` = controlled with this value; `None` = uncontrolled.
    open: Option<bool>,
    default_open: bool,
    log: Log,
    handles: Handles,
}

impl Render for SheetView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let changes = self.log.changes.clone();
        let body = self.log.body.clone();

        let mut sheet = Sheet::new("test-sheet")
            .side(self.side)
            .title("Filters")
            .on_open_change(move |open, _, _| changes.borrow_mut().push(open))
            .child(
                canvas(move |bounds, _, _| body.set(Some(bounds)), |_, (), _, _| {})
                    .w_full()
                    .h(px(40.)),
            )
            .footer_child(
                div()
                    .id("sheet-inside-a")
                    .track_focus(&self.handles.inside_a)
                    .w(px(80.))
                    .h(px(24.)),
            )
            .footer_child(
                div()
                    .id("sheet-inside-b")
                    .track_focus(&self.handles.inside_b)
                    .w(px(80.))
                    .h(px(24.)),
            );
        sheet = match self.open {
            Some(open) => sheet.open(open),
            None => sheet.default_open(self.default_open),
        };

        FluentRoot::new()
            .child(
                div()
                    .id("behind")
                    .track_focus(&self.handles.behind)
                    .w(px(120.))
                    .h(px(28.))
                    .child("behind"),
            )
            .child(sheet)
    }
}

fn open_view(
    cx: &mut TestAppContext,
    side: SheetSide,
    open: Option<bool>,
    default_open: bool,
) -> (WindowHandle<SheetView>, Log, Handles) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("tab", crate::FocusNext, Some(crate::ROOT_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", crate::FocusPrev, Some(crate::ROOT_KEY_CONTEXT)),
        ]);
    });
    let log = Log::default();
    let slot: Rc<RefCell<Option<Handles>>> = Default::default();
    let for_view = slot.clone();
    let view_log = log.clone();
    let window = cx.open_window(size(px(800.), px(600.)), move |_, cx| {
        let handles = Handles {
            behind: cx.focus_handle().tab_stop(true),
            inside_a: cx.focus_handle().tab_stop(true),
            inside_b: cx.focus_handle().tab_stop(true),
        };
        for_view.replace(Some(handles.clone()));
        SheetView {
            side,
            open,
            default_open,
            log: view_log,
            handles,
        }
    });
    cx.run_until_parked();
    let handles = slot.borrow().clone().expect("view built");
    (window, log, handles)
}

/// Draw twice so painted tab stops become visible to `focus_next`.
/// Draw twice so painted tab stops become visible to `focus_next`.
///
/// The recorded body bounds are cleared before each draw: a canvas that is no
/// longer mounted never runs, so a stale value must not be read as "still
/// painted".
fn settle(cx: &mut TestAppContext, window: WindowHandle<SheetView>) {
    for _ in 0..2 {
        cx.update_window(window.into(), |view, window, cx| {
            view.downcast::<SheetView>()
                .unwrap()
                .read(cx)
                .log
                .body
                .set(None);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

/// Advances the test clock and redraws, so presence transitions progress.
fn advance(cx: &mut TestAppContext, window: WindowHandle<SheetView>, millis: u64) {
    cx.executor().advance_clock(Duration::from_millis(millis));
    cx.run_until_parked();
    settle(cx, window);
}

fn set_open(cx: &mut TestAppContext, window: WindowHandle<SheetView>, open: bool) {
    cx.update_window(window.into(), |view, _, cx| {
        view.downcast::<SheetView>()
            .unwrap()
            .update(cx, |view, cx| {
                view.open = Some(open);
                cx.notify();
            });
    })
    .unwrap();
    cx.run_until_parked();
}

fn focus_inside(cx: &mut TestAppContext, window: WindowHandle<SheetView>, handle: &FocusHandle) {
    cx.update_window(window.into(), |_, window, cx| handle.focus(window, cx))
        .unwrap();
    settle(cx, window);
}

fn focused(cx: &mut TestAppContext, window: WindowHandle<SheetView>) -> Option<FocusHandle> {
    let mut out = None;
    cx.update_window(window.into(), |_, window, cx| out = window.focused(cx))
        .unwrap();
    out
}

fn press_key(cx: &mut TestAppContext, window: WindowHandle<SheetView>, key: &'static str) {
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse(key).unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, window);
}

fn click_at(
    cx: &mut TestAppContext,
    window: WindowHandle<SheetView>,
    position: gpui::Point<Pixels>,
) {
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            MouseDownEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, window);
}

#[gpui::test]
fn opening_moves_focus_into_the_sheet(cx: &mut TestAppContext) {
    let (window, _log, handles) = open_view(cx, SheetSide::Right, Some(false), false);
    settle(cx, window);
    focus_inside(cx, window, &handles.behind);

    set_open(cx, window, true);
    settle(cx, window);

    let focus = focused(cx, window).expect("something holds focus");
    assert_ne!(focus, handles.behind, "focus must leave the page behind");
    assert_ne!(
        focus, handles.inside_a,
        "focus lands on the sheet host, not a footer stop"
    );
}

#[gpui::test]
fn tab_stays_inside_the_sheet(cx: &mut TestAppContext) {
    let (window, _log, handles) = open_view(cx, SheetSide::Left, None, true);
    settle(cx, window);
    focus_inside(cx, window, &handles.inside_a);

    for _ in 0..4 {
        press_key(cx, window, "tab");
        assert_ne!(
            focused(cx, window),
            Some(handles.behind.clone()),
            "Tab escaped the sheet to the page behind"
        );
    }
    for _ in 0..4 {
        press_key(cx, window, "shift-tab");
        assert_ne!(
            focused(cx, window),
            Some(handles.behind.clone()),
            "Shift+Tab escaped the sheet to the page behind"
        );
    }
}

#[gpui::test]
fn closing_returns_focus_only_after_the_exit_finishes(cx: &mut TestAppContext) {
    let (window, _log, handles) = open_view(cx, SheetSide::Right, Some(false), false);
    settle(cx, window);
    focus_inside(cx, window, &handles.behind);
    set_open(cx, window, true);
    settle(cx, window);
    focus_inside(cx, window, &handles.inside_a);
    assert_ne!(focused(cx, window), Some(handles.behind.clone()));
    // Let the entry finish: closing during the entry intentionally collapses
    // the reversal instead of playing a full exit.
    advance(cx, window, 400);

    set_open(cx, window, false);
    settle(cx, window);
    assert_ne!(
        focused(cx, window),
        Some(handles.behind.clone()),
        "returning focus at the start of the exit would let Tab reach the covered page"
    );

    advance(cx, window, 150);
    assert_ne!(
        focused(cx, window),
        Some(handles.behind.clone()),
        "focus must stay trapped while the panel is still visible"
    );

    advance(cx, window, 400);
    assert_eq!(
        focused(cx, window),
        Some(handles.behind.clone()),
        "closing must hand focus back once the exit has finished"
    );
}

#[gpui::test]
fn closing_keeps_the_edge_it_opened_from(cx: &mut TestAppContext) {
    let (window, log, _handles) = open_view(cx, SheetSide::Left, Some(false), false);
    settle(cx, window);
    set_open(cx, window, true);
    settle(cx, window);
    advance(cx, window, 400);
    let open = log.body.get().expect("body measured");
    assert!(
        open.origin.x < px(100.),
        "the left sheet must be on the left: {open:?}"
    );

    // Close and reset the side to the default in the same update, which is what
    // a caller that derives `side` from an Option (the gallery) does.
    cx.update_window(window.into(), |view, _, cx| {
        view.downcast::<SheetView>()
            .unwrap()
            .update(cx, |view, cx| {
                view.open = Some(false);
                view.side = SheetSide::Right;
                cx.notify();
            });
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, window);
    advance(cx, window, 150);
    let leaving = log.body.get().expect("still painted mid-exit");
    assert!(
        leaving.origin.x < px(100.),
        "the exit must keep sliding out from the left, not jump to the right: {leaving:?}"
    );

    advance(cx, window, 400);
    assert!(
        log.body.get().is_none(),
        "the panel unmounts once the exit finishes"
    );
}

#[gpui::test]
fn escape_requests_a_close(cx: &mut TestAppContext) {
    let (window, log, handles) = open_view(cx, SheetSide::Right, None, true);
    settle(cx, window);
    focus_inside(cx, window, &handles.inside_a);

    press_key(cx, window, "escape");
    assert_eq!(
        &*log.changes.borrow(),
        &[false],
        "Escape must ask the sheet to close"
    );
}

#[gpui::test]
fn a_scrim_press_requests_a_close(cx: &mut TestAppContext) {
    let (window, log, _handles) = open_view(cx, SheetSide::Right, None, true);
    settle(cx, window);
    advance(cx, window, 400);

    // The right-edge panel starts at x = 480, so this point is scrim.
    click_at(cx, window, point(px(100.), px(300.)));
    assert_eq!(
        &*log.changes.borrow(),
        &[false],
        "a press on the scrim must ask the sheet to close"
    );
}

#[gpui::test]
fn a_press_on_the_panel_does_not_close(cx: &mut TestAppContext) {
    let (window, log, _handles) = open_view(cx, SheetSide::Right, None, true);
    settle(cx, window);
    advance(cx, window, 400);
    let body = log.body.get().expect("body measured");

    // Empty panel area below the body: it must still block the scrim layer.
    click_at(
        cx,
        window,
        point(body.origin.x + px(20.), body.origin.y + px(180.)),
    );
    assert!(
        log.changes.borrow().is_empty(),
        "a press inside the panel must not reach the dismiss path"
    );
}

#[gpui::test]
fn a_controlled_sheet_stays_open_when_the_caller_ignores_the_request(cx: &mut TestAppContext) {
    let (window, log, handles) = open_view(cx, SheetSide::Top, Some(true), false);
    settle(cx, window);
    focus_inside(cx, window, &handles.inside_a);

    press_key(cx, window, "escape");
    assert_eq!(&*log.changes.borrow(), &[false], "the request is reported");

    advance(cx, window, 500);
    assert!(
        log.body.get().is_some(),
        "a controlled sheet stays open until the caller passes open(false)"
    );
}

#[gpui::test]
fn an_uncontrolled_close_survives_a_redraw(cx: &mut TestAppContext) {
    let (window, log, handles) = open_view(cx, SheetSide::Bottom, None, true);
    settle(cx, window);
    focus_inside(cx, window, &handles.inside_a);

    press_key(cx, window, "escape");
    assert_eq!(log.changes.borrow().len(), 1);
    advance(cx, window, 500);
    settle(cx, window);
    assert!(
        log.body.get().is_none(),
        "the uncontrolled sheet must stay closed after the exit"
    );
    assert_eq!(log.changes.borrow().len(), 1);
}

#[gpui::test]
fn every_side_slides_in_from_its_own_edge(cx: &mut TestAppContext) {
    for side in [
        SheetSide::Right,
        SheetSide::Left,
        SheetSide::Top,
        SheetSide::Bottom,
    ] {
        let (window, log, _handles) = open_view(cx, side, Some(false), false);
        settle(cx, window);
        set_open(cx, window, true);
        settle(cx, window);
        let first = log.body.get().expect("the panel is laid out offscreen");
        advance(cx, window, 500);
        let settled = log.body.get().expect("the panel is laid out onscreen");

        match side {
            SheetSide::Right => assert!(
                first.origin.x > settled.origin.x + px(100.),
                "{side:?} must enter from the right: {first:?} then {settled:?}"
            ),
            SheetSide::Left => assert!(
                first.origin.x < settled.origin.x - px(100.),
                "{side:?} must enter from the left: {first:?} then {settled:?}"
            ),
            SheetSide::Top => assert!(
                first.origin.y + first.size.height < settled.origin.y - px(100.),
                "{side:?} must enter from the top: {first:?} then {settled:?}"
            ),
            SheetSide::Bottom => assert!(
                first.origin.y > settled.origin.y + px(100.),
                "{side:?} must enter from the bottom: {first:?} then {settled:?}"
            ),
        }
    }
}

#[gpui::test]
fn the_exit_animates_before_the_panel_unmounts(cx: &mut TestAppContext) {
    let (window, log, _handles) = open_view(cx, SheetSide::Right, Some(true), false);
    settle(cx, window);
    advance(cx, window, 500);
    let settled = log.body.get().expect("body measured");

    set_open(cx, window, false);
    settle(cx, window);
    advance(cx, window, 150);
    let leaving = log.body.get().expect("the panel is still painted mid-exit");
    assert!(
        leaving.origin.x > settled.origin.x + px(40.),
        "the exit must move the panel toward its edge: {settled:?} then {leaving:?}"
    );

    advance(cx, window, 400);
    assert!(
        log.body.get().is_none(),
        "the panel unmounts once the exit finishes"
    );
}

#[gpui::test]
fn every_side_anchors_flush_to_its_edge(cx: &mut TestAppContext) {
    for side in [
        SheetSide::Right,
        SheetSide::Left,
        SheetSide::Top,
        SheetSide::Bottom,
    ] {
        let (window, log, _handles) = open_view(cx, side, None, true);
        settle(cx, window);
        advance(cx, window, 500);
        let body = log.body.get().expect("body measured");
        assert!(
            body.origin.x >= px(0.) && body.origin.x + body.size.width <= px(800.),
            "{side:?} must be onscreen: {body:?}"
        );
        match side {
            SheetSide::Right => {
                let right = body.origin.x + body.size.width;
                assert_eq!(right, px(776.), "{side:?} must hug the right edge");
                assert!(body.size.width < px(320.), "{side:?} keeps its width");
            }
            SheetSide::Left => {
                assert_eq!(body.origin.x, px(24.), "{side:?} must hug the left edge");
                assert!(body.size.width < px(320.), "{side:?} keeps its width");
            }
            SheetSide::Top => {
                assert_eq!(body.origin.y, px(68.), "{side:?} must hug the top edge");
                assert_eq!(body.size.width, px(752.), "{side:?} spans the width");
            }
            SheetSide::Bottom => {
                // The body starts below the title and above the footer; the
                // recorded canvas is its first line, 68px below the panel top
                // (280) => 348.
                assert_eq!(body.origin.y, px(348.), "{side:?} must hug the bottom edge");
                assert_eq!(body.size.width, px(752.), "{side:?} spans the width");
            }
        }
    }
}

#[gpui::test]
fn reduced_motion_skips_the_slide(cx: &mut TestAppContext) {
    let (window, log, _handles) = open_view(cx, SheetSide::Right, Some(false), false);
    settle(cx, window);
    cx.update(|cx| cx.set_reduce_motion(true));
    set_open(cx, window, true);
    settle(cx, window);

    let body = log.body.get().expect("body measured");
    assert_eq!(
        body.origin.x + body.size.width,
        px(776.),
        "reduce-motion must place the panel at its resting position immediately"
    );
}

/// The modal layer is deferred like `Dialog`'s, so a page that clips its
/// children must not clip the scrim to the page's own bounds.
#[gpui::test]
fn the_modal_layer_escapes_a_clipped_parent(cx: &mut TestAppContext) {
    struct ClippedView;
    impl Render for ClippedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            FluentRoot::new().child(
                div().w(px(400.)).h(px(300.)).overflow_hidden().child(
                    Sheet::new("clipped-sheet")
                        .side(SheetSide::Right)
                        .open(true)
                        .title("Filters")
                        .child("A clipped ancestor"),
                ),
            )
        }
    }

    cx.update(fluent_core::gpui_compat::init_base);
    let window = cx.open_window(size(px(800.), px(600.)), |_, _| ClippedView);
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();

    let covered = cx
        .update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            let scale = window.scale_factor();
            let viewport = window.viewport_size();
            window.painted_quads().into_iter().any(|quad| {
                let Some(background) = quad.background.as_solid() else {
                    return false;
                };
                // The scrim is translucent black (40% light / 50% dark); the
                // page behind it is opaque, so this identifies the modal layer
                // without pinning one theme.
                if !(0.3..=0.6).contains(&background.a) || background.l > 0.05 {
                    return false;
                }
                let mask = quad.content_mask.bounds.size;
                mask.width.0 / scale >= viewport.width.as_f32() - 1.0
                    && mask.height.0 / scale >= viewport.height.as_f32() - 1.0
            })
        })
        .unwrap();
    assert!(
        covered,
        "the deferred modal layer must escape the clipped ancestor and cover the window"
    );
}

/// A sheet is an overlay: opening it must not change the flow layout around
/// it. The closed placeholder and the open host have to occupy the same
/// zero-height slot, or content below the sheet jumps by the flex gap.
#[gpui::test]
fn opening_does_not_shift_the_content_below(cx: &mut TestAppContext) {
    struct ShiftView {
        open: bool,
        marker: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for ShiftView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let marker = self.marker.clone();
            FluentRoot::new().child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .p(px(16.))
                    .child(div().h(px(32.)).child("Open"))
                    .child(Sheet::new("shift-sheet").open(self.open).child("Body"))
                    .child(
                        canvas(
                            move |bounds, _, _| marker.set(Some(bounds)),
                            |_, (), _, _| {},
                        )
                        .w(px(80.))
                        .h(px(24.)),
                    ),
            )
        }
    }

    cx.update(fluent_core::gpui_compat::init_base);
    let marker: Rc<Cell<Option<Bounds<Pixels>>>> = Default::default();
    let view_marker = marker.clone();
    let window = cx.open_window(size(px(400.), px(300.)), move |_, _| ShiftView {
        open: false,
        marker: view_marker,
    });
    cx.run_until_parked();

    let draw = |cx: &mut TestAppContext| {
        for _ in 0..2 {
            cx.update_window(window.into(), |view, window, cx| {
                view.downcast::<ShiftView>()
                    .unwrap()
                    .read(cx)
                    .marker
                    .set(None);
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .unwrap();
            cx.run_until_parked();
        }
        marker.get().expect("marker measured")
    };

    let closed = draw(cx);
    cx.update_window(window.into(), |view, _, cx| {
        view.downcast::<ShiftView>()
            .unwrap()
            .update(cx, |view, cx| {
                view.open = true;
                cx.notify();
            });
    })
    .unwrap();
    let open = draw(cx);
    assert_eq!(
        open.origin.y, closed.origin.y,
        "opening the sheet must not move the content below it: {closed:?} then {open:?}"
    );

    cx.update_window(window.into(), |view, _, cx| {
        view.downcast::<ShiftView>()
            .unwrap()
            .update(cx, |view, cx| {
                view.open = false;
                cx.notify();
            });
    })
    .unwrap();
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    let closed_again = draw(cx);
    assert_eq!(
        closed_again.origin.y, closed.origin.y,
        "closing the sheet must not move the content below it either"
    );
}

/// The same promise while the page is scrolled to its bottom, which is where
/// the gallery's sheet section sits: the absolute host must not change the
/// scrollable content or clamp the offset when it appears.
#[gpui::test]
fn opening_does_not_shift_a_scrolled_page(cx: &mut TestAppContext) {
    struct ScrollView {
        open: bool,
        scroll: gpui::ScrollHandle,
        marker: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for ScrollView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let marker = self.marker.clone();
            FluentRoot::new().child(
                div()
                    .id("sheet-scroll")
                    .w(px(300.))
                    .h(px(160.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .p(px(16.))
                            .child(div().h(px(120.)).child("spacer"))
                            .child(Sheet::new("scroll-sheet").open(self.open).child("Body"))
                            .child(div().h(px(120.)).child("after"))
                            .child(
                                canvas(
                                    move |bounds, _, _| marker.set(Some(bounds)),
                                    |_, (), _, _| {},
                                )
                                .w(px(80.))
                                .h(px(24.)),
                            ),
                    ),
            )
        }
    }

    cx.update(fluent_core::gpui_compat::init_base);
    let scroll = gpui::ScrollHandle::new();
    let marker: Rc<Cell<Option<Bounds<Pixels>>>> = Default::default();
    let view_scroll = scroll.clone();
    let view_marker = marker.clone();
    let window = cx.open_window(size(px(400.), px(300.)), move |_, _| ScrollView {
        open: false,
        scroll: view_scroll,
        marker: view_marker,
    });
    cx.run_until_parked();

    let draw = |cx: &mut TestAppContext| {
        for _ in 0..2 {
            cx.update_window(window.into(), |view, window, cx| {
                view.downcast::<ScrollView>()
                    .unwrap()
                    .read(cx)
                    .marker
                    .set(None);
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .unwrap();
            cx.run_until_parked();
        }
        marker.get().expect("marker measured")
    };

    draw(cx);
    scroll.set_offset(point(px(0.), px(-9999.)));
    let bottom = draw(cx);
    assert!(
        bottom.origin.y < px(160.),
        "the marker must be visible after scrolling to the bottom: {bottom:?}"
    );

    cx.update_window(window.into(), |view, _, cx| {
        view.downcast::<ScrollView>()
            .unwrap()
            .update(cx, |view, cx| {
                view.open = true;
                cx.notify();
            });
    })
    .unwrap();
    let open = draw(cx);
    assert_eq!(
        open.origin.y, bottom.origin.y,
        "opening the sheet must not shift a page scrolled to the bottom: {bottom:?} then {open:?}"
    );

    cx.update_window(window.into(), |view, _, cx| {
        view.downcast::<ScrollView>()
            .unwrap()
            .update(cx, |view, cx| {
                view.open = false;
                cx.notify();
            });
    })
    .unwrap();
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    let closed = draw(cx);
    assert_eq!(
        closed.origin.y, bottom.origin.y,
        "closing must not shift a page scrolled to the bottom either"
    );
}

/// The scrim must be an occluding hitbox, not just paint: otherwise the page
/// behind a modal still receives hover edges while it is covered.
#[gpui::test]
fn the_scrim_blocks_hover_on_the_page_behind(cx: &mut TestAppContext) {
    struct HoverView {
        log: Rc<RefCell<Vec<bool>>>,
    }
    impl Render for HoverView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let log = self.log.clone();
            FluentRoot::new()
                .child(
                    div()
                        .id("hover-probe")
                        .absolute()
                        .top(px(20.))
                        .left(px(20.))
                        .w(px(60.))
                        .h(px(32.))
                        .on_hover(move |hovered, _, _| log.borrow_mut().push(*hovered)),
                )
                .child(
                    Sheet::new("hover-sheet")
                        .side(SheetSide::Right)
                        .open(true)
                        .child("Body"),
                )
        }
    }

    cx.update(fluent_core::gpui_compat::init_base);
    let log: Rc<RefCell<Vec<bool>>> = Default::default();
    let view_log = log.clone();
    let window = cx.open_window(size(px(400.), px(300.)), move |_, _| HoverView {
        log: view_log,
    });
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    log.borrow_mut().clear();

    // The right-edge panel starts at x = 80, so this point is scrim.
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::MouseMoveEvent {
                position: point(px(50.), px(36.)),
                pressed_button: None,
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
            cx,
        );
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();

    assert!(
        !log.borrow().contains(&true),
        "hover must not reach the page behind the scrim: {:?}",
        log.borrow()
    );
}

/// The user-visible half of the same promise: a real `Button` behind the scrim
/// keeps its rest fill instead of fading to the hover fill.
#[gpui::test]
fn a_button_behind_the_scrim_keeps_its_rest_fill(cx: &mut TestAppContext) {
    use crate::{Button, ButtonAppearance};

    struct ButtonView;
    impl Render for ButtonView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            FluentRoot::new()
                .child(
                    div().absolute().top(px(20.)).left(px(20.)).child(
                        Button::new("behind-button")
                            .appearance(ButtonAppearance::Secondary)
                            .child("Behind"),
                    ),
                )
                .child(
                    Sheet::new("button-hover-sheet")
                        .side(SheetSide::Right)
                        .open(true)
                        .child("Body"),
                )
        }
    }

    cx.update(fluent_core::gpui_compat::init_base);
    let (rest, hover) = cx.update(|cx| {
        let theme = FluentTheme::global(cx);
        let tokens = theme.tokens();
        (
            tokens.color.neutral_background_1,
            tokens.color.neutral_background_1_hover,
        )
    });
    let window = cx.open_window(size(px(400.), px(300.)), |_, _| ButtonView);
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();

    // The panel starts at x = 80; this point is scrim over the button.
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::MouseMoveEvent {
                position: point(px(50.), px(36.)),
                pressed_button: None,
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
            cx,
        );
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.executor().advance_clock(Duration::from_millis(300));
    cx.run_until_parked();

    let fill = cx
        .update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            let scale = window.scale_factor();
            let point = point(px(50.), px(36.)).scale(scale);
            window
                .painted_quads()
                .into_iter()
                .filter(|quad| {
                    quad.bounds.contains(&point)
                        && quad.background.as_solid().is_some_and(|c| c.a > 0.99)
                        && quad.bounds.size.width.0 < 400. * scale
                })
                .filter_map(|quad| quad.background.as_solid())
                .next()
        })
        .unwrap();

    let fill = fill.expect("the button background is painted");
    assert!(
        (fill.h - rest.h).abs() < 1e-3
            && (fill.s - rest.s).abs() < 1e-3
            && (fill.l - rest.l).abs() < 1e-3,
        "the button must keep its rest fill under the scrim: rest={rest:?} hover={hover:?} painted={fill:?}"
    );
}
