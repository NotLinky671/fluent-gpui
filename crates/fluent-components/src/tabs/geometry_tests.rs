//! Assert immediately after a single draw; helpers must not hide a stale frame.
use super::*;
use crate::FluentRoot;
use gpui::{AppContext, Context, Render, TestAppContext, WindowHandle, size};

struct View {
    visible: bool,
    vertical: bool,
    label: SharedString,
}
impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(div().p(px(20.)).when(self.visible, |el| {
            el.child(
                TabList::new("geometry-tabs")
                    .when(self.vertical, |el| el.vertical())
                    .selected(1)
                    .tab(TabItem::new("first", self.label.clone()))
                    .tab(TabItem::new("second", "Second")),
            )
        }))
    }
}
fn open(cx: &mut TestAppContext, vertical: bool) -> WindowHandle<View> {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| cx.set_reduce_motion(true));
    let handle = cx.open_window(size(px(800.), px(400.)), move |_, _| View {
        visible: false,
        vertical,
        label: "A".into(),
    });
    cx.run_until_parked();
    handle
}
fn draw(
    cx: &mut TestAppContext,
    handle: WindowHandle<View>,
    edit: impl FnOnce(&mut View),
) -> Option<(f32, f32)> {
    cx.update_window(handle.into(), |view, window, cx| {
        let view = view.downcast::<View>().unwrap();
        let vertical = view.read(cx).vertical;
        view.update(cx, |view, cx| {
            edit(view);
            cx.notify();
        });
        window.refresh();
        window.draw(cx).clear(cx);
        let scale = window.scale_factor();
        window
            .painted_quads()
            .into_iter()
            .find(|q| {
                let thickness = if vertical {
                    q.bounds.size.width.0
                } else {
                    q.bounds.size.height.0
                };
                (thickness / scale - 2.).abs() < 0.6
            })
            .map(|q| {
                if vertical {
                    (q.bounds.origin.y.0 / scale, q.bounds.size.height.0 / scale)
                } else {
                    (q.bounds.origin.x.0 / scale, q.bounds.size.width.0 / scale)
                }
            })
    })
    .unwrap()
}
#[gpui::test]
fn the_horizontal_indicator_is_present_on_its_first_frame(cx: &mut TestAppContext) {
    let handle = open(cx, false);
    let first = draw(cx, handle, |view| view.visible = true)
        .expect("first frame must include the indicator");
    assert!(first.0 > 20. && first.1 > 0.);
}
#[gpui::test]
fn the_vertical_indicator_is_present_on_its_first_frame(cx: &mut TestAppContext) {
    let handle = open(cx, true);
    let first = draw(cx, handle, |view| view.visible = true)
        .expect("first frame must include the indicator");
    assert!(
        (first.0 - 60.).abs() < 1. && (first.1 - 36.).abs() < 1.,
        "selected second tab: {first:?}"
    );
}
#[gpui::test]
fn a_label_change_positions_the_indicator_in_the_same_frame(cx: &mut TestAppContext) {
    let handle = open(cx, false);
    draw(cx, handle, |view| view.visible = true);
    let before = draw(cx, handle, |_| {}).expect("initial indicator");
    let changed = draw(cx, handle, |view| {
        view.label = "A much longer first tab label".into()
    })
    .expect("changed frame indicator");
    let stable = draw(cx, handle, |_| {}).expect("stable frame indicator");
    assert!(
        stable.0 > before.0 + 50.,
        "precondition: longer first tab moves the second tab"
    );
    assert_eq!(
        changed, stable,
        "the event frame must already use the new tab bounds"
    );
}
