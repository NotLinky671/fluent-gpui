//! Pointer routing and painted output; the test platform cannot execute Win32
//! non-client actions. Those are checked separately in a real window.
use super::*;
use gpui::{AppContext, Context, Modifiers, PlatformInput, Render, TestAppContext, WindowHandle};

struct Harness {
    backdrop_presses: usize,
    menu_presses: usize,
    backdrop: bool,
    menu: bool,
    native: bool,
    allow_close: bool,
    close_requests: usize,
}

impl Render for Harness {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let spec = titlebar::bar(&FluentTheme::global(cx));
        // Use the same cell builder for both the native route and Linux fallback
        // on every CI host, without introducing a platform override in the API.
        let close = Rc::new(cx.listener(|this, _: &(), _, cx| {
            this.close_requests += 1;
            cx.notify();
        }));
        let allow = self.allow_close;
        let row =
            div()
                .flex()
                .children(
                    [Caption::Minimize, Caption::Maximize, Caption::Close].map(|button| {
                        let close = close.clone();
                        caption_cell(
                            "test-caption".into(),
                            button,
                            spec,
                            &TitleBarLabels::default(),
                            self.native,
                            Some(Rc::new(move |window, cx| {
                                close(&(), window, cx);
                                allow
                            })),
                        )
                    }),
                );
        let mut root = div().size_full().child(TitleBar::new(
            "test-bar",
            "很长的标题 / A long window title that must leave the window controls visible",
        ));
        if self.backdrop {
            root = root.child(
                deferred(
                    div()
                        .id("modal-backdrop")
                        .absolute()
                        .inset_0()
                        .occlude()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, _| this.backdrop_presses += 1),
                        ),
                )
                .with_priority(10),
            );
        }
        // Deliberately paint an earlier Drag area under all three cells.
        root = root.child(caption_layer(row, window));
        if self.menu {
            root = root.child(
                crate::Popover::new("caption-popup")
                    .open(true)
                    .draw_panel_surface(false)
                    .absolute()
                    .left(px(480.))
                    .top(px(0.))
                    .trigger(div().size(px(0.)))
                    .content(
                        div()
                            .debug_selector(|| "caption-popup-content".into())
                            .w(px(20.))
                            .h(px(100.))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.menu_presses += 1;
                                    cx.stop_propagation();
                                }),
                            ),
                    ),
            );
        }
        root
    }
}

fn open(cx: &mut TestAppContext, native: bool) -> WindowHandle<Harness> {
    cx.update(|cx| {
        fluent_core::gpui_compat::init_base(cx);
        FluentTheme::web_light().install(cx);
    });
    let handle = cx.open_window(size(px(500.), px(300.)), |_, _| Harness {
        backdrop_presses: 0,
        menu_presses: 0,
        backdrop: false,
        menu: false,
        native,
        allow_close: false,
        close_requests: 0,
    });
    cx.run_until_parked();
    handle
}

fn draw(cx: &mut TestAppContext, handle: WindowHandle<Harness>) -> Vec<gpui::Quad> {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        window.painted_quads()
    })
    .unwrap()
}

fn mouse(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
    x: f32,
    y: f32,
    down: Option<bool>,
) {
    cx.update_window(handle.into(), |_, window, cx| {
        let position = point(px(x), px(y));
        window.dispatch_event(
            PlatformInput::MouseMove(gpui::MouseMoveEvent {
                position,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }),
            cx,
        );
        if let Some(down) = down {
            let event = if down {
                PlatformInput::MouseDown(gpui::MouseDownEvent {
                    position,
                    button: MouseButton::Left,
                    modifiers: Modifiers::default(),
                    click_count: 1,
                    first_mouse: false,
                })
            } else {
                PlatformInput::MouseUp(gpui::MouseUpEvent {
                    position,
                    button: MouseButton::Left,
                    modifiers: Modifiers::default(),
                    click_count: 1,
                })
            };
            window.dispatch_event(event, cx);
        }
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn native_caption_has_no_manual_close_or_application_quit(cx: &mut TestAppContext) {
    let handle = open(cx, true);
    mouse(cx, handle, 477., 16., Some(true));
    mouse(cx, handle, 477., 16., Some(false));
    cx.read(|cx| {
        assert_eq!(cx.windows().len(), 1);
        assert_eq!(
            handle.read(cx).unwrap().close_requests,
            0,
            "Windows close must be requested by the native backend"
        );
    });
}

#[gpui::test]
fn fallback_close_can_be_vetoed_and_never_quits_other_windows(cx: &mut TestAppContext) {
    let handle = open(cx, false);
    let other = cx.open_window(size(px(200.), px(100.)), |_, _| Harness {
        backdrop_presses: 0,
        menu_presses: 0,
        backdrop: false,
        menu: false,
        native: true,
        allow_close: false,
        close_requests: 0,
    });
    mouse(cx, handle, 477., 16., Some(true));
    mouse(cx, handle, 477., 16., Some(false));
    cx.read(|cx| {
        assert_eq!(handle.read(cx).unwrap().close_requests, 1);
        assert_eq!(cx.windows().len(), 2);
    });
    handle
        .update(cx, |view, _, cx| {
            view.allow_close = true;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    mouse(cx, handle, 477., 16., Some(true));
    mouse(cx, handle, 477., 16., Some(false));
    cx.read(|cx| {
        assert_eq!(cx.windows().len(), 1);
        assert!(cx.windows().contains(&other.into()));
    });
}

#[gpui::test]
fn captions_block_modal_input_but_popups_stay_above_them(cx: &mut TestAppContext) {
    let handle = open(cx, true);
    handle
        .update(cx, |view, _, cx| {
            view.backdrop = true;
            view.menu = true;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    let mut visual = gpui::VisualTestContext::from_window(handle.into(), cx);
    let popup = visual
        .debug_bounds("caption-popup-content")
        .expect("the real popup is painted");
    assert!(
        popup.contains(&point(px(490.), px(16.))),
        "the popup must overlap the close button: {popup:?}"
    );
    for x in [385., 431., 470.] {
        mouse(cx, handle, x, 16., Some(true));
        mouse(cx, handle, x, 16., Some(false));
    }
    cx.read(|cx| assert_eq!(handle.read(cx).unwrap().backdrop_presses, 0));
    mouse(cx, handle, 490., 16., Some(true));
    mouse(cx, handle, 490., 16., Some(false));
    cx.read(|cx| assert_eq!(handle.read(cx).unwrap().menu_presses, 1));
    mouse(cx, handle, 340., 16., Some(true));
    mouse(cx, handle, 340., 16., Some(false));
    cx.read(|cx| assert_eq!(handle.read(cx).unwrap().backdrop_presses, 1));
}

#[gpui::test]
fn close_hover_and_pressed_backgrounds_are_actually_painted(cx: &mut TestAppContext) {
    let handle = open(cx, true);
    let colors = cx.read(|cx| titlebar::bar(&FluentTheme::global(cx)).colors);
    mouse(cx, handle, 477., 16., None);
    let scale = cx
        .update_window(handle.into(), |_, window, _| window.scale_factor())
        .unwrap();
    let probe = point(
        gpui::ScaledPixels(477. * scale),
        gpui::ScaledPixels(16. * scale),
    );
    let has_color = |quads: Vec<gpui::Quad>, color: gpui::Hsla| {
        quads
            .iter()
            .any(|quad| quad.bounds.contains(&probe) && quad.background.as_solid() == Some(color))
    };
    assert!(has_color(draw(cx, handle), colors.close_hover));
    mouse(cx, handle, 477., 16., Some(true));
    assert!(has_color(draw(cx, handle), colors.close_pressed));
    mouse(cx, handle, 477., 16., Some(false));
    assert!(has_color(draw(cx, handle), colors.close_hover));
    mouse(cx, handle, 300., 100., None);
    assert!(!has_color(draw(cx, handle), colors.close_hover));
}

/// The glyph at 150% used to round both the 1 DIP stroke and the 1.5 DIP radius
/// up to 2 device pixels, leaving an inner radius of zero: a square, heavy box.
#[test]
fn window_outline_keeps_an_inner_corner_and_cuts_its_square_corners() {
    let metrics = titlebar::bar(&FluentTheme::web_light()).metrics;
    let stroke = metrics.glyph_stroke_width;
    let radius = metrics.glyph_corner_radius;
    assert!(
        radius > stroke,
        "the inner corner needs radius > stroke, got {radius:?} vs {stroke:?}"
    );

    let bounds = Bounds::new(
        point(px(20.), px(20.)),
        size(metrics.glyph_size, metrics.glyph_size),
    );
    let path = caption_outline(bounds, stroke, radius).expect("a stroked rounded rectangle");
    assert!(!path.vertices.is_empty(), "the outline must have ink");

    let r = f32::from(radius);
    let (left, top) = (f32::from(bounds.left()), f32::from(bounds.top()));
    let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
    let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
    for vertex in &path.vertices {
        let (x, y) = (
            f32::from(vertex.xy_position.x),
            f32::from(vertex.xy_position.y),
        );
        // A square corner puts ink at the exact corner; the arc's closest
        // approach along the diagonal is (1 - 1/√2) · r ≈ 0.29r.
        assert!(
            !(x < left + 0.25 * r && y < top + 0.25 * r),
            "ink reaches the square corner at ({x}, {y})"
        );
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    // The arc only rounds the corners; the box still spans the ink box.
    let side = f32::from(metrics.glyph_size);
    assert!(
        (max_x - min_x - side).abs() < 0.02,
        "width {}",
        max_x - min_x
    );
    assert!(
        (max_y - min_y - side).abs() < 0.02,
        "height {}",
        max_y - min_y
    );
}

#[test]
fn titlebar_options_configure_platform_chrome() {
    let options = TitleBar::window_options("Test");
    let titlebar = options.titlebar.unwrap();
    assert!(titlebar.appears_transparent);
    assert_eq!(titlebar.title.as_deref(), Some("Test"));
    assert_eq!(titlebar.traffic_light_position, Some(point(px(9.), px(9.))));
    assert!(options.app_owns_titlebar_drag);
    assert_eq!(options.window_decorations, Some(WindowDecorations::Client));
    assert!(options.icon.is_none(), "channels stay opt-in");
}

/// A 1×1 transparent PNG, enough to exercise the encoded-bytes entry point.
const PNG_1X1: [u8; 67] = [
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1, 13, 10,
    45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[test]
fn window_options_with_icon_feeds_the_platform_icon() {
    let icon = Arc::new(image::RgbaImage::new(32, 32));
    let options = TitleBar::window_options_with_icon("Test", icon);
    let platform_icon = options.icon.expect("the platform icon is set");
    assert_eq!(platform_icon.width(), 32);
    // The rest of the chrome is unchanged.
    assert!(options.titlebar.unwrap().appears_transparent);
    assert!(options.app_owns_titlebar_drag);
}

#[test]
fn logo_bytes_keeps_the_encoded_image_for_the_slot() {
    let bar = TitleBar::new("bar", "T").logo_bytes(ImageFormat::Png, PNG_1X1.to_vec());
    match bar.mark {
        Some(TitleBarMark::Logo(image)) => {
            assert_eq!(image.format, ImageFormat::Png);
            assert_eq!(image.bytes, PNG_1X1);
        }
        _ => panic!("encoded bytes must become a bitmap logo"),
    }

    assert!(matches!(
        TitleBar::new("bar", "T").icon(FluentIcon::Home).mark,
        Some(TitleBarMark::Glyph(FluentIcon::Home))
    ));

    // Only the explicit slot reserves space with nothing in it.
    let reserved = TitleBar::new("bar", "T").icon_slot();
    assert!(reserved.mark.is_none() && reserved.reserve_mark);

    let plain = TitleBar::new("bar", "T");
    assert!(
        plain.mark.is_none() && !plain.reserve_mark,
        "the slot stays opt-in"
    );
}

#[gpui::test]
fn the_icon_slot_holds_the_title_in_place_for_every_kind_of_artwork(cx: &mut TestAppContext) {
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Plain,
        Reserved,
        Glyph,
        Logo,
    }

    struct View(Kind);

    impl Render for View {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bar = TitleBar::new("slot-bar", "Title");
            let bar = match self.0 {
                Kind::Plain => bar,
                Kind::Reserved => bar.icon_slot(),
                Kind::Glyph => bar.icon(FluentIcon::Home),
                Kind::Logo => bar.logo_bytes(ImageFormat::Png, PNG_1X1.to_vec()),
            };
            div().size_full().child(bar)
        }
    }

    let metrics = titlebar::bar(&FluentTheme::web_light()).metrics;
    let slot = f32::from(metrics.icon_size) + f32::from(metrics.gap);
    let mut title_x = |kind: Kind| -> f32 {
        let handle = cx.open_window(size(px(400.), px(120.)), |_, _| View(kind));
        cx.run_until_parked();
        let mut visual = gpui::VisualTestContext::from_window(handle.into(), cx);
        f32::from(
            visual
                .debug_bounds("titlebar-title")
                .expect("the title is laid out")
                .origin
                .x,
        )
    };

    let plain = title_x(Kind::Plain);
    let reserved = title_x(Kind::Reserved);
    let glyph = title_x(Kind::Glyph);
    let logo = title_x(Kind::Logo);
    assert_eq!(
        reserved,
        plain + slot,
        "an empty slot must reserve the same space as drawn artwork"
    );
    assert_eq!(glyph, reserved, "a glyph takes the reserved slot");
    assert_eq!(
        logo, reserved,
        "a bitmap logo takes the reserved slot without shifting the title"
    );
}
