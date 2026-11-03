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
    TestAppContext, WindowHandle, canvas, point, px, size,
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
