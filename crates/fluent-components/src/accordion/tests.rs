//! Accordion behaviour: which panels open, when a second panel closes the
//! first, what the keyboard can reach, and where the body and the focus ring
//! actually land on screen.
//!
//! These are end-to-end tests: every assertion drives real clicks or key
//! presses through a window and reads the painted result, so a header that
//! looks interactive but never toggles fails here.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Bounds, Context, FocusHandle, Hsla, InputEvent as _, KeyBinding, KeyDownEvent,
    Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, PlatformInput, Point,
    Render, Size, TestAppContext, Window, WindowHandle, point, px, size,
};

/// The colour of every panel body in the fixture. Nothing else in the window
/// uses it, so a painted quad of this colour is a body and nothing else.
fn body_colour() -> Hsla {
    gpui::rgba(0x14_a0_3c_ff).into()
}

struct AccordionView {
    focus: FocusHandle,
    size: AccordionSize,
    multiple: bool,
    collapsible: bool,
    default_open: Vec<usize>,
}

impl AccordionView {
    fn item(&self, id: &'static str, label: &'static str) -> AccordionItem {
        AccordionItem::new(id, label).content(div().h(px(40.)).w_full().bg(body_colour()))
    }

    fn accordion(&self) -> Accordion {
        let body = self.item("probe-one", "One");
        let second = self.item("probe-two", "Two");
        let disabled = AccordionItem::new("probe-three", "Three")
            .disabled(true)
            .content(div().h(px(40.)).w_full().bg(body_colour()));

        Accordion::new("probe-accordion")
            .size(self.size)
            .multiple(self.multiple)
            .collapsible(self.collapsible)
            .default_open_items(self.default_open.clone())
            .item(body)
            .item(second)
            .item(disabled)
    }
}

impl Render for AccordionView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // A preceding focus stop gives Tab somewhere to come from: GPUI's
        // `focus_next` does nothing when nothing is focused.
        FluentRoot::new().child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .w(px(360.))
                .child(
                    div()
                        .id("probe-before")
                        .track_focus(&self.focus)
                        .h(px(24.))
                        .child("before"),
                )
                .child(self.accordion()),
        )
    }
}

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([KeyBinding::new(
            "tab",
            crate::FocusNext,
            Some(crate::ROOT_KEY_CONTEXT),
        )]);
    });
}

fn open_view(
    cx: &mut TestAppContext,
    default_open: Vec<usize>,
    multiple: bool,
    collapsible: bool,
) -> WindowHandle<AccordionView> {
    bind_keys(cx);
    let handle = cx.open_window(size(px(420.), px(400.)), move |_, cx| AccordionView {
        focus: cx.focus_handle().tab_stop(true),
        size: AccordionSize::Medium,
        multiple,
        collapsible,
        default_open,
    });
    cx.run_until_parked();
    draw(cx, handle);
    handle
}

fn open(cx: &mut TestAppContext) -> WindowHandle<AccordionView> {
    open_view(cx, Vec::new(), false, false)
}

fn draw(cx: &mut TestAppContext, handle: WindowHandle<AccordionView>) {
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

/// The panel bodies on screen, in logical pixels.
fn bodies(cx: &mut TestAppContext, handle: WindowHandle<AccordionView>) -> Vec<Bounds<Pixels>> {
    let mut found = Vec::new();
    let body = body_colour();
    cx.update_window(handle.into(), |_, window, _| {
        let scale = window.scale_factor();
        let to_px = |value: gpui::ScaledPixels| px(value.0 / scale);
        for quad in window.painted_quads() {
            let Some(colour) = quad.background.as_solid() else {
                continue;
            };
            if colour != body {
                continue;
            }
            found.push(Bounds {
                origin: Point {
                    x: to_px(quad.bounds.origin.x),
                    y: to_px(quad.bounds.origin.y),
                },
                size: Size {
                    width: to_px(quad.bounds.size.width),
                    height: to_px(quad.bounds.size.height),
                },
            });
        }
    })
    .unwrap();
    found
}

/// The quads carrying the focus ring, found by their border colour.
fn ringed(cx: &mut TestAppContext, handle: WindowHandle<AccordionView>) -> Vec<Bounds<Pixels>> {
    let mut found = Vec::new();
    let ring = cx.update(|cx| FluentTheme::global(cx).tokens().color.stroke_focus_2);
    cx.update_window(handle.into(), |_, window, _| {
        let scale = window.scale_factor();
        let to_px = |value: gpui::ScaledPixels| px(value.0 / scale);
        for quad in window.painted_quads() {
            if quad.border_color != ring {
                continue;
            }
            if quad.border_widths.top == gpui::ScaledPixels(0.) {
                continue;
            }
            let bounds = Bounds {
                origin: Point {
                    x: to_px(quad.bounds.origin.x),
                    y: to_px(quad.bounds.origin.y),
                },
                size: Size {
                    width: to_px(quad.bounds.size.width),
                    height: to_px(quad.bounds.size.height),
                },
            };
            // One element can contribute several primitives with the ring
            // colour; the surface is what matters.
            if !found.contains(&bounds) {
                found.push(bounds);
            }
        }
    })
    .unwrap();
    found
}

/// Presses and releases the pointer at `position`.
fn click(cx: &mut TestAppContext, handle: WindowHandle<AccordionView>, position: Point<Pixels>) {
    cx.update_window(handle.into(), move |_, window, cx| {
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
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    draw(cx, handle);
}

fn press(cx: &mut TestAppContext, handle: WindowHandle<AccordionView>, key: &'static str) {
    let keystroke = Keystroke::parse(key).unwrap();
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.dispatch_event(PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }), cx);
    })
    .unwrap();
    cx.run_until_parked();
    draw(cx, handle);
}

/// The 32px above the accordion: the fixture's leading focus stop and its gap.
const BEFORE: f32 = 32.;

/// The centre of a header when nothing is open: 44px rows stacked below `BEFORE`.
fn header_at(index: usize) -> Point<Pixels> {
    point(px(120.), px(BEFORE + 44. * index as f32 + 22.))
}

#[gpui::test]
fn clicking_a_header_opens_its_panel(cx: &mut TestAppContext) {
    let handle = open(cx);
    assert!(
        bodies(cx, handle).is_empty(),
        "nothing is open until a header is clicked"
    );

    click(cx, handle, header_at(0));
    assert_eq!(bodies(cx, handle).len(), 1, "the first panel must open");

    click(cx, handle, header_at(1));
    assert_eq!(
        bodies(cx, handle).len(),
        1,
        "a single accordion keeps one panel open and moves to the second"
    );
}

#[gpui::test]
fn a_collapsible_accordion_closes_on_a_second_click(cx: &mut TestAppContext) {
    let handle = open_view(cx, Vec::new(), false, true);
    click(cx, handle, header_at(0));
    assert_eq!(bodies(cx, handle).len(), 1, "the first click opens");

    click(cx, handle, header_at(0));
    assert!(
        bodies(cx, handle).is_empty(),
        "a collapsible accordion closes its open panel"
    );
}

#[gpui::test]
fn a_non_collapsible_accordion_keeps_its_panel_open(cx: &mut TestAppContext) {
    let handle = open(cx);
    click(cx, handle, header_at(0));
    click(cx, handle, header_at(0));
    assert_eq!(
        bodies(cx, handle).len(),
        1,
        "without collapsible the only open panel stays open"
    );
}

#[gpui::test]
fn multiple_keeps_the_first_panel_when_the_second_opens(cx: &mut TestAppContext) {
    let handle = open_view(cx, Vec::new(), true, true);
    click(cx, handle, header_at(0));
    // The open panel is 40px tall, so the second header has moved down by then.
    click(cx, handle, point(px(120.), px(BEFORE + 44. + 40. + 22.)));
    assert_eq!(
        bodies(cx, handle).len(),
        2,
        "multiple opens every panel on its own"
    );
}

#[gpui::test]
fn a_disabled_header_ignores_clicks_and_keeps_its_panel_closed(cx: &mut TestAppContext) {
    let handle = open(cx);
    click(cx, handle, header_at(2));
    assert!(
        bodies(cx, handle).is_empty(),
        "a disabled header must not open its panel"
    );
}

#[gpui::test]
fn the_panel_body_sits_inside_the_panel_margins(cx: &mut TestAppContext) {
    let handle = open_view(cx, vec![0], false, false);
    let bodies = bodies(cx, handle);
    let body = bodies.first().expect("the default open panel must paint");
    assert_eq!(
        body.origin.x,
        px(12.),
        "the panel carries Fluent's 12px side margin, and the accordion fills its 360px parent"
    );
    assert_eq!(
        body.size.width,
        px(336.),
        "both margins are taken from the panel's width"
    );
}

#[gpui::test]
fn the_keyboard_toggles_the_focused_header(cx: &mut TestAppContext) {
    let handle = open(cx);
    focus_before(cx, handle);
    press(cx, handle, "tab");
    press(cx, handle, "enter");
    assert_eq!(
        bodies(cx, handle).len(),
        1,
        "Enter on a focused header opens its panel"
    );

    press(cx, handle, "space");
    assert_eq!(
        bodies(cx, handle).len(),
        1,
        "Space on the same header toggles it back, and a non-collapsible accordion keeps it open"
    );
}

#[gpui::test]
fn arrow_down_moves_focus_without_toggling(cx: &mut TestAppContext) {
    let handle = open(cx);
    focus_before(cx, handle);
    press(cx, handle, "tab");
    press(cx, handle, "down");
    assert!(
        bodies(cx, handle).is_empty(),
        "moving between headers must not open anything"
    );

    press(cx, handle, "enter");
    assert_eq!(
        bodies(cx, handle).len(),
        1,
        "Enter after the arrow opens the second header"
    );
    let body = bodies(cx, handle)[0];
    assert!(
        body.origin.y >= px(BEFORE + 44. * 2.),
        "the opened panel belongs to the second header, not the first one"
    );
}

#[gpui::test]
fn the_focus_ring_appears_only_for_the_keyboard(cx: &mut TestAppContext) {
    let handle = open(cx);
    focus_before(cx, handle);
    assert!(
        ringed(cx, handle).is_empty(),
        "nothing is focused yet, so no ring"
    );

    press(cx, handle, "tab");
    assert_eq!(
        ringed(cx, handle).len(),
        1,
        "a focused header draws the 2px focus ring"
    );

    // A mouse press focuses through the pointer, which is not :focus-visible.
    click(cx, handle, header_at(0));
    assert!(
        ringed(cx, handle).is_empty(),
        "the ring is for keyboard focus only, as focus_visible is"
    );
}
/// Puts the keyboard on the fixture's leading stop, so Tab has a starting point.
fn focus_before(cx: &mut TestAppContext, handle: WindowHandle<AccordionView>) {
    let focus = cx.update(|cx| handle.read(cx).unwrap().focus.clone());
    cx.update_window(handle.into(), move |_, window, cx| {
        window.focus(&focus, cx);
    })
    .unwrap();
    cx.run_until_parked();
    draw(cx, handle);
}

#[gpui::test]
fn a_non_collapsible_multiple_accordion_keeps_its_last_panel(cx: &mut TestAppContext) {
    let handle = open_view(cx, vec![0], true, false);
    assert_eq!(
        bodies(cx, handle).len(),
        1,
        "the default open panel must start open"
    );
    click(cx, handle, header_at(0));
    assert_eq!(
        bodies(cx, handle).len(),
        1,
        "with one panel left and no collapsible flag, Fluent keeps it open"
    );
}

#[gpui::test]
fn a_multiple_collapsible_accordion_closes_its_last_panel(cx: &mut TestAppContext) {
    let handle = open_view(cx, vec![0], true, true);
    click(cx, handle, header_at(0));
    assert!(
        bodies(cx, handle).is_empty(),
        "a collapsible multiple accordion can close every panel"
    );
}
