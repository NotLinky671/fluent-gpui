//! Mixed modal stacks must agree on paint order and event ownership.
use super::*;
use crate::{AlertDialog, Dialog, FluentRoot};
use gpui::{
    AppContext, Context, KeyDownEvent, Keystroke, PlatformInput, Render, TestAppContext, size,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug)]
enum Kind {
    Dialog,
    Alert,
    Sheet,
}
type Changes = Rc<RefCell<Vec<&'static str>>>;

fn modal(
    kind: Kind,
    id: &'static str,
    child: gpui::AnyElement,
    log: &Changes,
    controlled: bool,
) -> gpui::AnyElement {
    let log = log.clone();
    let changed = move |_: bool, _: &mut Window, _: &mut App| log.borrow_mut().push(id);
    match kind {
        Kind::Dialog => Dialog::new(id)
            .default_open(true)
            .on_open_change(move |open, _, window, cx| changed(open, window, cx))
            .child(child)
            .into_any_element(),
        Kind::Alert => AlertDialog::new(id)
            .default_open(true)
            .on_open_change(move |open, _, window, cx| changed(open, window, cx))
            .child(child)
            .into_any_element(),
        Kind::Sheet => {
            let sheet = Sheet::new(id)
                .default_open(true)
                .on_open_change(changed)
                .child(child);
            if controlled {
                sheet.open(true).into_any_element()
            } else {
                sheet.into_any_element()
            }
        }
    }
}
struct View {
    kinds: Vec<Kind>,
    log: Changes,
    controlled: bool,
}
impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut child = div().into_any_element();
        for (i, kind) in self.kinds.iter().enumerate().rev() {
            let id = if i == 0 {
                "outer"
            } else if i == self.kinds.len() - 1 {
                "inner"
            } else {
                "middle"
            };
            let color = [0xff0000, 0x00ff00, 0x0000ff][i];
            child = div()
                .child(div().w(px(40.)).h(px(24.)).bg(gpui::rgb(color)))
                .child(child)
                .into_any_element();
            child = modal(
                *kind,
                id,
                child,
                &self.log,
                self.controlled && i == self.kinds.len() - 1,
            );
        }
        FluentRoot::new().child(child)
    }
}
fn open(
    cx: &mut TestAppContext,
    kinds: &[Kind],
    controlled: bool,
) -> (gpui::WindowHandle<View>, Changes) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| cx.set_reduce_motion(true));
    let log: Changes = Default::default();
    let view_log = log.clone();
    let kinds = kinds.to_vec();
    let handle = cx.open_window(size(px(800.), px(600.)), move |_, _| View {
        kinds,
        log: view_log,
        controlled,
    });
    cx.run_until_parked();
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    (handle, log)
}
fn escape(cx: &mut TestAppContext, handle: gpui::WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse("escape").unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
}
#[gpui::test]
fn escape_in_a_nested_sheet_does_not_close_its_dialog(cx: &mut TestAppContext) {
    let (handle, log) = open(cx, &[Kind::Dialog, Kind::Sheet], false);
    escape(cx, handle);
    assert_eq!(*log.borrow(), ["inner"]);
}
#[gpui::test]
fn escape_in_a_nested_sheet_does_not_close_its_sheet(cx: &mut TestAppContext) {
    let (handle, log) = open(cx, &[Kind::Sheet, Kind::Sheet], false);
    escape(cx, handle);
    assert_eq!(*log.borrow(), ["inner"]);
}
#[gpui::test]
fn a_refused_nested_sheet_close_still_consumes_escape(cx: &mut TestAppContext) {
    let (handle, log) = open(cx, &[Kind::Dialog, Kind::Sheet], true);
    escape(cx, handle);
    escape(cx, handle);
    assert_eq!(*log.borrow(), ["inner", "inner"]);
}
fn assert_paint_order(cx: &mut TestAppContext, kinds: &[Kind]) {
    let (handle, _) = open(cx, kinds, false);
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        let quads = window.painted_quads();
        let markers: Vec<_> = [0xff0000, 0x00ff00, 0x0000ff].iter().take(kinds.len()).map(|color| {
            quads.iter().position(|quad| quad.background.as_solid() == Some(gpui::rgb(*color).into())).expect("modal content painted")
        }).collect();
        let scrims: Vec<_> = quads
            .iter()
            .enumerate()
            .filter(|(_, quad)| {
                quad.background
                    .as_solid()
                    .is_some_and(|color| color.l < 0.05 && (0.3..=0.6).contains(&color.a))
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            scrims.len(),
            kinds.len(),
            "every modal must paint its scrim: {kinds:?}"
        );
        for (depth, marker) in markers.iter().enumerate() {
            assert!(*marker > scrims[depth], "content must paint after its scrim: {kinds:?}, markers {markers:?}, scrims {scrims:?}");
            if let Some(next_scrim) = scrims.get(depth + 1) {
                assert!(*marker < *next_scrim, "child scrim must paint above parent content: {kinds:?}, markers {markers:?}, scrims {scrims:?}");
            }
        }
    })
    .unwrap();
}
#[gpui::test]
fn a_nested_sheet_paints_above_its_dialog_scrim(cx: &mut TestAppContext) {
    assert_paint_order(cx, &[Kind::Dialog, Kind::Sheet]);
}
#[gpui::test]
fn a_nested_sheet_paints_above_its_alert_scrim(cx: &mut TestAppContext) {
    assert_paint_order(cx, &[Kind::Alert, Kind::Sheet]);
}
#[gpui::test]
fn a_nested_dialog_paints_above_its_sheet_scrim(cx: &mut TestAppContext) {
    assert_paint_order(cx, &[Kind::Sheet, Kind::Dialog]);
}
#[gpui::test]
fn three_mixed_modals_paint_in_nesting_order(cx: &mut TestAppContext) {
    assert_paint_order(cx, &[Kind::Dialog, Kind::Sheet, Kind::Dialog]);
}
