//! Behaviour tests for `Dialog` and `AlertDialog`.
//!
//! The tests drive a real window because the behaviour under test is which
//! element receives the event: backdrop presses, focus paths and key contexts
//! are hit-test and dispatch results, not callback shapes. Two facts matter
//! for every focus assertion: a `FocusHandle` only joins the tab order when
//! `tab_stop(true)` is set, and stops painted on one frame are visible to
//! `focus_next` on the next, so each test settles with two draws first.

use super::*;
use gpui::{
    AppContext, Context, FocusHandle, InputEvent as _, InteractiveElement as _, KeyBinding,
    KeyDownEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent, PlatformInput, Render,
    StatefulInteractiveElement as _, TestAppContext, WindowHandle, canvas, point, px, size,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What the harness recorded, in order.
#[derive(Clone, Default)]
struct Log {
    /// `(open, reason)` pairs the dialog reported.
    changes: Rc<RefCell<Vec<(bool, DialogChangeReason)>>>,
    /// Ok/cancel decisions and close notifications, in order.
    decisions: Rc<RefCell<Vec<&'static str>>>,
    /// Measured bounds of the card body, for aiming presses inside it.
    card: Rc<Cell<Option<gpui::Bounds<gpui::Pixels>>>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Dialog,
    Alert,
}

#[derive(Clone)]
struct Handles {
    behind: FocusHandle,
    inside_a: FocusHandle,
    inside_b: FocusHandle,
}

struct DialogView {
    kind: Kind,
    /// `Some` = controlled with this value; `None` = uncontrolled.
    open: Option<bool>,
    default_open: bool,
    /// When `Some`, `on_ok`/`on_cancel` answer with its contents.
    decision: Option<Rc<Cell<bool>>>,
    log: Log,
    handles: Handles,
}

impl Render for DialogView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let log = self.log.clone();
        let changes = log.changes.clone();
        let decisions = log.decisions.clone();
        let card = log.card.clone();
        let decision = self.decision.clone();

        let footer = vec![
            div()
                .id("dialog-inside-a")
                .track_focus(&self.handles.inside_a)
                .w(px(80.))
                .h(px(24.))
                .into_any_element(),
            div()
                .id("dialog-inside-b")
                .track_focus(&self.handles.inside_b)
                .w(px(80.))
                .h(px(24.))
                .into_any_element(),
        ];

        let body = canvas(move |bounds, _, _| card.set(Some(bounds)), |_, (), _, _| {})
            .w(px(200.))
            .h(px(48.));

        macro_rules! wire {
            ($d:expr) => {{
                let mut d = $d
                    .title("Title")
                    .on_open_change(move |open, reason, _, _| {
                        changes.borrow_mut().push((open, reason));
                    })
                    .on_ok({
                        let decisions = decisions.clone();
                        let decision = decision.clone();
                        move |_, _, _| {
                            decisions.borrow_mut().push("ok");
                            decision.as_ref().map(|v| v.get()).unwrap_or(true)
                        }
                    })
                    .on_cancel({
                        let decisions = decisions.clone();
                        let decision = decision.clone();
                        move |_, _, _| {
                            decisions.borrow_mut().push("cancel");
                            decision.as_ref().map(|v| v.get()).unwrap_or(true)
                        }
                    })
                    .on_close({
                        let decisions = decisions.clone();
                        move |_, _, _| decisions.borrow_mut().push("close")
                    })
                    .footer(footer);
                d = match self.open {
                    Some(open) => d.open(open),
                    None => d.default_open(self.default_open),
                };
                d.child(body).into_element().into_any_element()
            }};
        }

        let dialog_el = match self.kind {
            Kind::Dialog => wire!(Dialog::new("test-dialog")),
            Kind::Alert => wire!(AlertDialog::new("test-alert")),
        };

        crate::root::FluentRoot::new()
            .child(
                div()
                    .id("behind")
                    .track_focus(&self.handles.behind)
                    .w(px(120.))
                    .h(px(28.))
                    .child("behind"),
            )
            .child(dialog_el)
    }
}

fn open_view(
    cx: &mut TestAppContext,
    kind: Kind,
    open: Option<bool>,
    default_open: bool,
) -> (WindowHandle<DialogView>, Log, Handles) {
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
        DialogView {
            kind,
            open,
            default_open,
            decision: None,
            log: view_log,
            handles,
        }
    });
    cx.run_until_parked();
    let handles = slot.borrow().clone().expect("view built");
    (window, log, handles)
}

/// Draw twice so painted tab stops become visible to `focus_next`.
fn settle(cx: &mut TestAppContext, window: WindowHandle<DialogView>) {
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

fn focused(cx: &mut TestAppContext, window: WindowHandle<DialogView>) -> Option<FocusHandle> {
    let mut out = None;
    cx.update_window(window.into(), |_, window, cx| out = window.focused(cx))
        .unwrap();
    out
}

fn press_key(cx: &mut TestAppContext, window: WindowHandle<DialogView>, key: &'static str) {
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse(key).unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn click_at(
    cx: &mut TestAppContext,
    window: WindowHandle<DialogView>,
    position: gpui::Point<gpui::Pixels>,
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
}

#[gpui::test]
fn opening_moves_focus_into_the_card(cx: &mut TestAppContext) {
    let (window, _log, h) = open_view(cx, Kind::Dialog, Some(false), false);
    settle(cx, window);
    cx.update_window(window.into(), |_, window, cx| h.behind.focus(window, cx))
        .unwrap();
    settle(cx, window);

    cx.update_window(window.into(), |view, _, cx| {
        view.downcast::<DialogView>().unwrap().update(cx, |v, cx| {
            v.open = Some(true);
            cx.notify();
        });
    })
    .unwrap();
    settle(cx, window);

    let focus = focused(cx, window).expect("something holds focus");
    assert_ne!(focus, h.behind, "focus must leave the page behind");
    assert_ne!(
        focus, h.inside_a,
        "focus lands on the card host, not a footer stop"
    );
}

#[gpui::test]
fn tab_stays_inside_the_dialog(cx: &mut TestAppContext) {
    let (window, _log, h) = open_view(cx, Kind::Dialog, None, true);
    settle(cx, window);
    cx.update_window(window.into(), |_, window, cx| h.inside_a.focus(window, cx))
        .unwrap();
    settle(cx, window);

    for _ in 0..4 {
        press_key(cx, window, "tab");
        assert_ne!(
            focused(cx, window),
            Some(h.behind.clone()),
            "Tab escaped the dialog to the page behind"
        );
    }
    for _ in 0..4 {
        press_key(cx, window, "shift-tab");
        assert_ne!(
            focused(cx, window),
            Some(h.behind.clone()),
            "Shift+Tab escaped the dialog to the page behind"
        );
    }
}

#[gpui::test]
fn closing_returns_focus_to_where_it_was(cx: &mut TestAppContext) {
    let (window, _log, h) = open_view(cx, Kind::Dialog, Some(false), false);
    settle(cx, window);
    cx.update_window(window.into(), |_, window, cx| h.behind.focus(window, cx))
        .unwrap();
    settle(cx, window);

    cx.update_window(window.into(), |view, _, cx| {
        view.downcast::<DialogView>().unwrap().update(cx, |v, cx| {
            v.open = Some(true);
            cx.notify();
        });
    })
    .unwrap();
    settle(cx, window);
    assert_ne!(
        focused(cx, window),
        Some(h.behind.clone()),
        "opening must move focus into the dialog"
    );

    cx.update_window(window.into(), |view, _, cx| {
        view.downcast::<DialogView>().unwrap().update(cx, |v, cx| {
            v.open = Some(false);
            cx.notify();
        });
    })
    .unwrap();
    settle(cx, window);
    assert_eq!(
        focused(cx, window),
        Some(h.behind.clone()),
        "closing must hand focus back to the element that had it"
    );
}

#[gpui::test]
fn escape_requests_cancel(cx: &mut TestAppContext) {
    let (window, log, h) = open_view(cx, Kind::Dialog, None, true);
    settle(cx, window);
    cx.update_window(window.into(), |_, window, cx| h.inside_a.focus(window, cx))
        .unwrap();
    settle(cx, window);

    press_key(cx, window, "escape");
    assert_eq!(
        &*log.changes.borrow(),
        &[(false, DialogChangeReason::Cancel)],
        "Escape must request a Cancel close"
    );
    assert_eq!(
        &*log.decisions.borrow(),
        &["cancel", "close"],
        "cancel decides, then close is reported"
    );
}

#[gpui::test]
fn escape_does_nothing_for_alert_dialog(cx: &mut TestAppContext) {
    let (window, log, h) = open_view(cx, Kind::Alert, None, true);
    settle(cx, window);
    cx.update_window(window.into(), |_, window, cx| h.inside_a.focus(window, cx))
        .unwrap();
    settle(cx, window);

    press_key(cx, window, "escape");
    assert!(
        log.changes.borrow().is_empty(),
        "an alert dialog must not answer Escape"
    );
}

#[gpui::test]
fn backdrop_press_requests_cancel_for_dialog(cx: &mut TestAppContext) {
    let (window, log, _h) = open_view(cx, Kind::Dialog, None, true);
    settle(cx, window);
    // Top-left corner is scrim, away from the centred card.
    click_at(cx, window, point(px(10.), px(10.)));
    assert_eq!(
        &*log.changes.borrow(),
        &[(false, DialogChangeReason::BackdropPress)]
    );
    assert_eq!(&*log.decisions.borrow(), &["cancel", "close"]);
}

#[gpui::test]
fn backdrop_press_does_nothing_for_alert_dialog(cx: &mut TestAppContext) {
    let (window, log, _h) = open_view(cx, Kind::Alert, None, true);
    settle(cx, window);
    click_at(cx, window, point(px(10.), px(10.)));
    assert!(log.changes.borrow().is_empty());
    assert!(log.decisions.borrow().is_empty());
}

#[gpui::test]
fn pressing_inside_the_card_does_not_close(cx: &mut TestAppContext) {
    let (window, log, _h) = open_view(cx, Kind::Dialog, None, true);
    settle(cx, window);
    let card = log.card.get().expect("card measured");
    click_at(
        cx,
        window,
        point(
            card.origin.x + card.size.width / 2.,
            card.origin.y + card.size.height / 2.,
        ),
    );
    assert!(
        log.changes.borrow().is_empty(),
        "a press inside the card must stay inside"
    );
}

#[gpui::test]
fn a_vetoed_confirm_keeps_the_dialog_open(cx: &mut TestAppContext) {
    let (window, log, _h) = open_view(cx, Kind::Dialog, None, true);
    cx.update_window(window.into(), |view, _, cx| {
        view.downcast::<DialogView>()
            .unwrap()
            .update(cx, |v, _| v.decision = Some(Rc::new(Cell::new(false))));
    })
    .unwrap();
    settle(cx, window);

    press_key(cx, window, "enter");
    assert_eq!(
        &*log.decisions.borrow(),
        &["ok"],
        "ok ran and vetoed the close"
    );
    assert!(
        log.changes.borrow().is_empty(),
        "a vetoed confirm must not close the dialog"
    );
}

#[gpui::test]
fn enter_confirms_and_reports_confirm(cx: &mut TestAppContext) {
    let (window, log, h) = open_view(cx, Kind::Dialog, None, true);
    settle(cx, window);
    cx.update_window(window.into(), |_, window, cx| h.inside_a.focus(window, cx))
        .unwrap();
    settle(cx, window);

    press_key(cx, window, "enter");
    assert_eq!(
        &*log.changes.borrow(),
        &[(false, DialogChangeReason::Confirm)],
        "Enter must request a Confirm close"
    );
    assert_eq!(&*log.decisions.borrow(), &["ok", "close"]);
}

#[gpui::test]
fn controlled_caller_that_ignores_the_request_stays_open(cx: &mut TestAppContext) {
    let (window, log, h) = open_view(cx, Kind::Dialog, Some(true), false);
    settle(cx, window);
    cx.update_window(window.into(), |_, window, cx| h.inside_a.focus(window, cx))
        .unwrap();
    settle(cx, window);

    press_key(cx, window, "escape");
    assert_eq!(
        &*log.changes.borrow(),
        &[(false, DialogChangeReason::Cancel)],
        "the request is reported"
    );
    // No caller re-render happened: the card must still paint its body.
    cx.update_window(window.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert!(
        log.card.get().is_some(),
        "a controlled dialog stays open until the caller agrees"
    );
}

#[gpui::test]
fn uncontrolled_close_survives_a_redraw(cx: &mut TestAppContext) {
    let (window, log, h) = open_view(cx, Kind::Dialog, None, true);
    settle(cx, window);
    cx.update_window(window.into(), |_, window, cx| h.inside_a.focus(window, cx))
        .unwrap();
    settle(cx, window);

    press_key(cx, window, "escape");
    assert_eq!(log.changes.borrow().len(), 1);
    // Force redraws; the uncontrolled store must keep the dialog closed.
    settle(cx, window);
    assert_eq!(
        log.changes.borrow().len(),
        1,
        "a redraw must not reopen the closed uncontrolled dialog"
    );
}

#[gpui::test]
fn dialog_role_is_not_observable_at_the_element_edge(cx: &mut TestAppContext) {
    // The deferred host's `role` is applied to the inner div during layout,
    // not propagated to the outer element, so `a11y_role` on the composed
    // element returns `None` in this snapshot. Recorded here so a future bump
    // that starts forwarding it turns this into a real assertion.
    use gpui::Element as _;
    cx.update(|_cx| {
        let el = Dialog::new("role-dialog").default_open(true).into_element();
        assert_eq!(el.a11y_role(), None);
        let el = AlertDialog::new("role-alert")
            .default_open(true)
            .into_element();
        assert_eq!(el.a11y_role(), None);
    });
}

/// Per-frame evidence for the footer-flicker report using the real `Button`:
/// with the pointer resting on Cancel and a continuously-animating sibling,
/// the focused handle and every painted quad must be identical on
/// consecutive draws — a footer button that flickers between its focused and
/// unfocused look would change one or the other every frame.
#[gpui::test]
fn an_open_dialogs_footer_buttons_hold_state_across_repaints(cx: &mut TestAppContext) {
    use crate::{Button, ButtonAppearance};

    struct FlickerView;
    impl Render for FlickerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            crate::root::FluentRoot::new()
                .child(crate::ProgressBar::new(0.).indeterminate().w(px(200.)))
                .child(
                    Dialog::new("flicker-dialog")
                        .title("Edit details")
                        .open(true)
                        .child("Dialogs dim the page, take focus and return it on close.")
                        .footer_child(
                            Button::new("flicker-cancel")
                                .appearance(ButtonAppearance::Secondary)
                                .child("Cancel"),
                        )
                        .footer_child(
                            Button::new("flicker-save")
                                .appearance(ButtonAppearance::Primary)
                                .child("Save"),
                        ),
                )
        }
    }

    cx.update(fluent_core::gpui_compat::init_base);
    let window = cx.open_window(size(px(800.), px(600.)), |_, _| FlickerView);
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    // Rest the pointer on the footer row: the card is centred in the 800x600
    // window and the footer hugs its bottom-right, so the sweep finds the
    // hover edge of the first button it touches.
    for pos in [(560., 340.), (620., 340.), (560., 355.), (620., 355.)] {
        cx.update_window(window.into(), |_, window, cx| {
            window.dispatch_event(
                gpui::MouseMoveEvent {
                    position: point(px(pos.0), px(pos.1)),
                    pressed_button: None,
                    modifiers: Modifiers::default(),
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
    }
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    let mut frames = Vec::new();
    for _ in 0..8 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            let focus = window.focused(cx).map(|h| format!("{h:?}"));
            // The ProgressBar keeps animating (its segment grows), so only
            // quads inside the centred card are compared — the footer
            // buttons' background, border and focus-ring shadows all live
            // there, below the window's upper half (logical px * 2 DPR).
            let quads: Vec<_> = window
                .painted_quads()
                .into_iter()
                .filter(|q| q.bounds.origin.y.0 > 220.)
                .map(|q| {
                    (
                        q.background.as_solid().unwrap_or_default(),
                        q.border_color,
                        q.bounds.origin.x.0,
                        q.bounds.origin.y.0,
                        q.bounds.size.width.0,
                        q.bounds.size.height.0,
                    )
                })
                .collect();
            frames.push((focus, quads));
        })
        .unwrap();
        cx.run_until_parked();
    }
    for (ix, f) in frames.iter().enumerate() {
        eprintln!("frame {ix}: focus={:?} quads={:?}", f.0, f.1);
    }
    let first = &frames[0];
    for (ix, f) in frames.iter().enumerate() {
        assert_eq!(
            f, first,
            "frame {ix} diverged: the footer buttons' state must hold across repaints"
        );
    }
}

/// The footer-flicker mechanism, pinned down on a real window: `on_hover`'s
/// bootstrap reads `is_hovered` against the *previous* frame's
/// `mouse_hit_test`, so a hitbox regenerated by a `deferred` repaint reports
/// a spurious `false` for a frame even with a stationary pointer — and the
/// animated background fades out and back in. The `Button` keeps `on_hover`
/// as the occlusion-aware edge source but latches `true`: a `false` edge is
/// only honoured when the live pointer is outside the painted bounds. This
/// test parks the pointer dead-centre on a real `Button` and asserts the
/// hover colour persists across repaints. The deterministic harness cannot
/// churn `HitboxId`s the way the real compositor does, so the failing case
/// lives in the recorded real-window trace; this test locks the contract.
#[gpui::test]
fn a_parked_pointer_keeps_a_buttons_hover_colour(cx: &mut TestAppContext) {
    use crate::{Button, ButtonAppearance};

    struct View;
    impl Render for View {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            crate::root::FluentRoot::new().child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_full()
                    .child(
                        Button::new("hover-target")
                            .appearance(ButtonAppearance::Subtle)
                            .child("Hover me"),
                    ),
            )
        }
    }

    cx.update(fluent_core::gpui_compat::init_base);
    let window = cx.open_window(size(px(400.), px(300.)), |_, _| View);
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    let centre = point(px(200.), px(150.));
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::MouseMoveEvent {
                position: centre,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    for _ in 0..3 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    let mut solid_frames = 0;
    for _ in 0..6 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            let scaled = centre.scale(2.);
            let hovered_fill = window
                .painted_quads()
                .into_iter()
                .find(|q| q.bounds.contains(&scaled))
                .and_then(|q| q.background.as_solid())
                .map(|c| c.a > 0.);
            if hovered_fill == Some(true) {
                solid_frames += 1;
            }
        })
        .unwrap();
        cx.run_until_parked();
    }
    assert_eq!(
        solid_frames, 6,
        "a parked pointer must keep the button's hover background painted every frame"
    );
}

/// The recorded hover state is only cleared when the pointer genuinely
/// leaves the button's bounds; a keyboard event (Tab/key) does not move the
/// pointer, so a parked pointer must keep the hover surface. This pins the
/// modality decision: a focused-and-pointed button keeps its hover look.
#[gpui::test]
fn a_keyboard_event_does_not_clear_a_parked_hover(cx: &mut TestAppContext) {
    use crate::{Button, ButtonAppearance};

    struct View;
    impl Render for View {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            crate::root::FluentRoot::new().child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_full()
                    .child(
                        Button::new("kbd-hover")
                            .appearance(ButtonAppearance::Subtle)
                            .child("Hover me"),
                    ),
            )
        }
    }

    cx.update(fluent_core::gpui_compat::init_base);
    let window = cx.open_window(size(px(400.), px(300.)), |_, _| View);
    cx.run_until_parked();
    let centre = point(px(200.), px(150.));
    let scaled = centre.scale(2.);
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::MouseMoveEvent {
                position: centre,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    for _ in 0..3 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    // Press a key — the window flips to keyboard modality but the pointer
    // has not moved off the button.
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::KeyDownEvent {
                keystroke: Keystroke::parse("tab").unwrap(),
                is_held: false,
                prefer_character_input: false,
            }
            .to_platform_input(),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    // The hover fill must still be painted: keyboard modality must not blink
    // the hover surface off a button the pointer still rests on.
    let still_hovered = cx
        .update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            window
                .painted_quads()
                .into_iter()
                .find(|q| q.bounds.contains(&scaled))
                .and_then(|q| q.background.as_solid())
                .map(|c| c.a > 0.)
        })
        .unwrap();
    assert_eq!(
        still_hovered,
        Some(true),
        "a key event must not clear the hover surface of a parked pointer"
    );
}

/// Occlusion caveat: the hover guard validates a `false` `on_hover` edge
/// against the button's geometric bounds + live pointer position only — it
/// cannot tell that a modal scrim is covering the button. The covered
/// button's `hovered` may therefore stay true while gpui's `is_hovered` is
/// false. That divergence is invisible because the scrim repaints on top:
/// this test pins that the modal layer paints a fullscreen quad over the
/// button's area, so the stale hover surface is always hidden behind it.
#[gpui::test]
fn a_modal_dialog_paints_its_scrim_over_a_button(cx: &mut TestAppContext) {
    use crate::{Button, ButtonAppearance};

    struct View;
    impl Render for View {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            crate::root::FluentRoot::new()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size_full()
                        .child(
                            Button::new("covered-target")
                                .appearance(ButtonAppearance::Subtle)
                                .child("Cover me"),
                        ),
                )
                .child(
                    Dialog::new("covering-dialog")
                        .title("On top")
                        .open(true)
                        .child("A modal over the button"),
                )
        }
    }

    cx.update(fluent_core::gpui_compat::init_base);
    let window = cx.open_window(size(px(400.), px(300.)), |_, _| View);
    cx.run_until_parked();
    for _ in 0..3 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    let centre = point(px(200.), px(150.));
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::MouseMoveEvent {
                position: centre,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    for _ in 0..3 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    // The scrim paints a fullscreen quad over the button's point.
    let scaled = centre.scale(2.);
    let covered = cx
        .update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            window
                .painted_quads()
                .into_iter()
                .any(|q| q.bounds.contains(&scaled) && q.bounds.size.width > 600.0.into())
        })
        .unwrap();
    assert!(
        covered,
        "the modal scrim must paint a fullscreen quad over the button"
    );
}

/// A dialog is an overlay: opening it must not change the flow layout around
/// it. The base host renders `deferred(anchored(...))` while open and an empty
/// `div()` while closed, so the wrapper has to keep the same zero-height
/// in-flow slot in both states or the content below jumps by the flex gap.
#[gpui::test]
fn opening_does_not_shift_the_content_below(cx: &mut TestAppContext) {
    struct ShiftView {
        open: bool,
        marker: Rc<Cell<Option<gpui::Bounds<gpui::Pixels>>>>,
    }
    impl Render for ShiftView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let marker = self.marker.clone();
            crate::root::FluentRoot::new().child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .p(px(16.))
                    .child(div().h(px(32.)).child("Open"))
                    .child(
                        Dialog::new("shift-dialog")
                            .open(self.open)
                            .title("Title")
                            .child("Body"),
                    )
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
    let marker: Rc<Cell<Option<gpui::Bounds<gpui::Pixels>>>> = Default::default();
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
        "opening the dialog must not move the content below it: {closed:?} then {open:?}"
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
    let closed_again = draw(cx);
    assert_eq!(
        closed_again.origin.y, closed.origin.y,
        "closing the dialog must not move the content below it either"
    );
}

/// The same promise while the page is scrolled to its bottom, which is where
/// the gallery's dialog section sits: the absolute host must not change the
/// scrollable content or clamp the offset when it appears.
#[gpui::test]
fn opening_does_not_shift_a_scrolled_page(cx: &mut TestAppContext) {
    struct ScrollView {
        open: bool,
        scroll: gpui::ScrollHandle,
        marker: Rc<Cell<Option<gpui::Bounds<gpui::Pixels>>>>,
    }
    impl Render for ScrollView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let marker = self.marker.clone();
            crate::root::FluentRoot::new().child(
                div()
                    .id("dialog-scroll")
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
                            .child(
                                Dialog::new("scroll-dialog")
                                    .open(self.open)
                                    .title("Title")
                                    .child("Body"),
                            )
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
    let marker: Rc<Cell<Option<gpui::Bounds<gpui::Pixels>>>> = Default::default();
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
        "opening the dialog must not shift a page scrolled to the bottom: {bottom:?} then {open:?}"
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
    let closed = draw(cx);
    assert_eq!(
        closed.origin.y, bottom.origin.y,
        "closing must not shift a page scrolled to the bottom either"
    );
}
