//! Fluent window chrome. Mount one `TitleBar` at the top of each window and use
//! [`TitleBar::window_options`] when opening it. Windows handles caption mouse
//! actions natively; macOS keeps its traffic lights; Linux draws controls only
//! with client-side decorations. The component never quits the application.

use crate::icon::Icon;
use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
};
use fluent_core::spec::titlebar::{self, TitleBarMetrics, TitleBarSpec};
use fluent_icons::FluentIcon;
use fluent_theme::FluentTheme;
use gpui::{
    Bounds, Decorations, Image, ImageFormat, MouseButton, PathBuilder, Pixels, Role,
    TitlebarOptions, WindowControlArea, WindowDecorations, WindowOptions, anchored, canvas,
    deferred, fill, img, point, px, size,
};
use std::{cell::Cell, rc::Rc, sync::Arc};

// Dialogs use 10 + layer; anchored tooltips use 200. Popovers/menus use 100.
const CAPTION_PRIORITY: usize = 50;
type CloseRequest = Rc<dyn Fn(&mut Window, &mut App) -> bool>;

/// Accessible caption names, overridable by applications for their locale.
#[derive(Clone, Debug)]
pub struct TitleBarLabels {
    pub minimize: SharedString,
    pub maximize: SharedString,
    pub restore: SharedString,
    pub close: SharedString,
}

impl Default for TitleBarLabels {
    fn default() -> Self {
        Self {
            minimize: "Minimize".into(),
            maximize: "Maximize".into(),
            restore: "Restore".into(),
            close: "Close".into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Caption {
    Minimize,
    Maximize,
    Restore,
    Close,
}

impl Caption {
    fn name(self) -> &'static str {
        match self {
            Self::Minimize => "minimize",
            Self::Maximize => "maximize",
            Self::Restore => "restore",
            Self::Close => "close",
        }
    }

    fn area(self) -> WindowControlArea {
        match self {
            Self::Minimize => WindowControlArea::Min,
            Self::Maximize | Self::Restore => WindowControlArea::Max,
            Self::Close => WindowControlArea::Close,
        }
    }

    fn label(self, labels: &TitleBarLabels) -> SharedString {
        match self {
            Self::Minimize => labels.minimize.clone(),
            Self::Maximize => labels.maximize.clone(),
            Self::Restore => labels.restore.clone(),
            Self::Close => labels.close.clone(),
        }
    }
}

/// Artwork for the bar's leading slot.
#[derive(Clone)]
enum TitleBarMark {
    /// A Fluent glyph, tinted by the theme.
    Glyph(FluentIcon),
    /// A bitmap logo, drawn at its natural colours.
    Logo(Arc<Image>),
}

/// Custom title bar with platform caption controls. Place at the window's top.
///
/// Windows mouse actions use `WindowControlArea` exclusively, including close,
/// which reaches the application's `Window::on_window_should_close` callback.
/// Use [`Self::on_close`] for the non-Windows custom close button. Closing one
/// window never overrides the application's GPUI `QuitMode`.
#[derive(IntoElement)]
pub struct TitleBar {
    id: ElementId,
    title: SharedString,
    mark: Option<TitleBarMark>,
    /// Keep the leading slot even when there is no artwork to draw.
    reserve_mark: bool,
    labels: TitleBarLabels,
    on_close: Option<CloseRequest>,
    style: StyleRefinement,
}

impl TitleBar {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            mark: None,
            reserve_mark: false,
            labels: TitleBarLabels::default(),
            on_close: None,
            style: StyleRefinement::default(),
        }
    }

    /// A Fluent glyph in the leading slot, tinted by the theme.
    pub fn icon(mut self, icon: FluentIcon) -> Self {
        self.mark = Some(TitleBarMark::Glyph(icon));
        self
    }

    /// A bitmap logo in the leading slot, drawn at its own colours. It is
    /// scaled with `ObjectFit::Contain` into
    /// [`TitleBarMetrics::icon_size`], so pass a square, high-resolution source.
    pub fn logo(mut self, image: Arc<Image>) -> Self {
        self.mark = Some(TitleBarMark::Logo(image));
        self
    }

    /// The same as [`Self::logo`], from encoded bytes (PNG, JPEG, SVG, …). GPUI
    /// decodes it off the UI thread, so this does not block the first frame.
    pub fn logo_bytes(self, format: ImageFormat, bytes: Vec<u8>) -> Self {
        self.logo(Arc::new(Image::from_bytes(format, bytes)))
    }

    /// Reserve the leading slot without drawing anything in it.
    ///
    /// The title then starts at the same x whether or not artwork is present,
    /// so a logo that resolves later cannot shift the caption. Artwork passed
    /// to [`Self::icon`], [`Self::logo`] or [`Self::logo_bytes`] occupies that
    /// same slot, so the title stays put when it appears.
    pub fn icon_slot(mut self) -> Self {
        self.reserve_mark = true;
        self
    }

    pub fn labels(mut self, labels: TitleBarLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Veto the non-Windows, client-decorated close button by returning `false`.
    /// Native Windows/macOS closes use `Window::on_window_should_close` instead;
    /// this builder deliberately does not replace that application-owned hook.
    pub fn on_close(mut self, request: impl Fn(&mut Window, &mut App) -> bool + 'static) -> Self {
        self.on_close = Some(Rc::new(request));
        self
    }

    /// Base options for a window containing this component. Override bounds,
    /// minimum size and other explicit application options with struct update.
    pub fn window_options(title: impl Into<SharedString>) -> WindowOptions {
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(title.into()),
                appears_transparent: true,
                // The default 32px bar centres macOS's native traffic lights.
                traffic_light_position: Some(point(px(9.), px(9.))),
            }),
            app_owns_titlebar_drag: true,
            window_decorations: Some(WindowDecorations::Client),
            ..Default::default()
        }
    }

    /// [`Self::window_options`] plus the platform window icon, so an
    /// application can feed one piece of artwork to the caption and the task
    /// bar. Pass the same bytes to [`Self::logo_bytes`] to keep them identical.
    ///
    /// In this GPUI snapshot `WindowOptions::icon` only reaches the window
    /// manager on X11 (`_NET_WM_ICON`). Windows takes its icon from the
    /// executable's resources and macOS from the app bundle, so this field is
    /// inert there; the caption logo still draws.
    pub fn window_options_with_icon(
        title: impl Into<SharedString>,
        icon: Arc<image::RgbaImage>,
    ) -> WindowOptions {
        WindowOptions {
            icon: Some(icon),
            ..Self::window_options(title)
        }
    }
}

impl Styled for TitleBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

fn custom_controls(window: &Window) -> bool {
    cfg!(target_os = "windows")
        || (cfg!(any(target_os = "linux", target_os = "freebsd"))
            && matches!(window.window_decorations(), Decorations::Client { .. }))
}

fn captions(window: &Window) -> Vec<Caption> {
    if window.is_fullscreen() || !custom_controls(window) {
        return Vec::new();
    }
    let supported = window.window_controls();
    [
        (supported.minimize && window.is_minimizable()).then_some(Caption::Minimize),
        (supported.maximize && window.is_resizable()).then_some(if window.is_maximized() {
            Caption::Restore
        } else {
            Caption::Maximize
        }),
        Some(Caption::Close),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The leading slot keeps one width with or without artwork, so a logo that
/// resolves later cannot shift the title.
fn mark_slot(size: Pixels, gap: Pixels, art: Option<AnyElement>) -> gpui::Div {
    div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .w(size)
        .h(size)
        .mr(gap)
        .children(art)
}

// A vector stroke bypasses paint_quad's automatic border-width snapping, which
// otherwise discards the half-pixel edge coverage at fractional display scales.
fn caption_outline(
    bounds: Bounds<Pixels>,
    stroke: Pixels,
    radius: Pixels,
) -> Option<gpui::Path<Pixels>> {
    let half = stroke / 2.;
    let left = bounds.left() + half;
    let right = bounds.right() - half;
    let top = bounds.top() + half;
    let bottom = bounds.bottom() - half;
    let radius = (radius - half).max(px(0.));
    let arc = point(radius, radius);
    let mut path = PathBuilder::stroke(stroke).with_style(gpui::PathStyle::Stroke(
        gpui::StrokeOptions::default()
            .with_line_width(f32::from(stroke))
            .with_tolerance(0.01),
    ));
    path.move_to(point(left + radius, top));
    path.line_to(point(right - radius, top));
    path.arc_to(arc, px(0.), false, true, point(right, top + radius));
    path.line_to(point(right, bottom - radius));
    path.arc_to(arc, px(0.), false, true, point(right - radius, bottom));
    path.line_to(point(left + radius, bottom));
    path.arc_to(arc, px(0.), false, true, point(left, bottom - radius));
    path.line_to(point(left, top + radius));
    path.arc_to(arc, px(0.), false, true, point(left + radius, top));
    path.close();
    path.build().ok()
}

// All glyphs share the same ink box. Read the *paint-time* foreground, so the
// close X follows hover/pressed white rather than retaining a render-time colour.
fn caption_glyph(caption: Caption, metrics: TitleBarMetrics) -> impl IntoElement {
    let glyph_size = metrics.glyph_size;
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let color = window.text_style().color;
            let scale = window.scale_factor();
            let snap = |value: Pixels| px((f32::from(value) * scale).round() / scale);
            let origin = point(snap(bounds.origin.x), snap(bounds.origin.y));
            let side = snap(glyph_size);
            let stroke = px(scale.round().max(1.) / scale);
            let mut rect = |x: Pixels, y: Pixels, width: Pixels, height: Pixels| {
                window.paint_quad(fill(
                    Bounds::new(origin + point(x, y), size(width, height)),
                    color,
                ));
            };
            let zero = px(0.);
            match caption {
                Caption::Minimize => rect(zero, snap((side - stroke) / 2.), side, stroke),
                Caption::Maximize | Caption::Restore => {
                    let offset = if caption == Caption::Restore {
                        snap(side * 0.2)
                    } else {
                        zero
                    };
                    let front = side - offset;
                    // Snap the outer bounds, but not the outline thickness or
                    // radius. At 150%, rounding both to 2 device pixels made
                    // the inner radius zero and the glyph look heavy/square.
                    let outline_stroke = metrics.glyph_stroke_width.max(px(1. / scale));
                    let outline = |bounds| {
                        caption_outline(
                            bounds,
                            outline_stroke,
                            metrics.glyph_corner_radius.max(outline_stroke),
                        )
                    };
                    if caption == Caption::Restore {
                        let back = Bounds::new(origin + point(offset, zero), size(front, front));
                        // Only the top and right strips of the rear window show.
                        // Clip a rounded outline rather than hiding it with an
                        // opaque fill, so every theme/hover background still shows.
                        if let Some(path) = outline(back) {
                            for visible in [
                                Bounds::new(origin + point(offset, zero), size(front, offset)),
                                Bounds::new(origin + point(front, offset), size(offset, front)),
                            ] {
                                window.with_content_mask(
                                    Some(gpui::ContentMask { bounds: visible }),
                                    |window| window.paint_path(path.clone(), color),
                                );
                            }
                        }
                    }
                    if let Some(path) = outline(Bounds::new(
                        origin + point(zero, offset),
                        size(front, front),
                    )) {
                        window.paint_path(path, color);
                    }
                }
                Caption::Close => {
                    for (from, to) in [
                        (point(zero, zero), point(side, side)),
                        (point(side, zero), point(zero, side)),
                    ] {
                        let mut path = PathBuilder::stroke(stroke);
                        path.move_to(origin + from);
                        path.line_to(origin + to);
                        if let Ok(path) = path.build() {
                            window.paint_path(path, color);
                        }
                    }
                }
            }
        },
    )
    .size(glyph_size)
    .flex_shrink_0()
}

fn caption_cell(
    id: ElementId,
    caption: Caption,
    spec: TitleBarSpec,
    labels: &TitleBarLabels,
    native: bool,
    on_close: Option<CloseRequest>,
) -> gpui::Stateful<gpui::Div> {
    let colors = spec.colors;
    let close = caption == Caption::Close;
    let cell = div()
        .id(ElementId::NamedChild(id.into(), caption.name().into()))
        .role(Role::Button)
        .aria_label(caption.label(labels))
        .occlude()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .w(spec.metrics.caption_width)
        .h(spec.metrics.caption_height)
        .text_color(colors.glyph)
        .hover(move |style| {
            style
                .bg(if close {
                    colors.close_hover
                } else {
                    colors.button_hover
                })
                .text_color(if close {
                    colors.close_glyph_hover
                } else {
                    colors.glyph
                })
        })
        .active(move |style| {
            style
                .bg(if close {
                    colors.close_pressed
                } else {
                    colors.button_pressed
                })
                .text_color(if close {
                    colors.close_glyph_hover
                } else {
                    colors.glyph
                })
        })
        .child(caption_glyph(caption, spec.metrics));
    #[cfg(test)]
    let cell = cell.debug_selector(move || format!("caption-{}", caption.name()));
    if native {
        // No on_click here: the backend forwards NC input to GPUI before doing
        // the native action. Registering both would execute two window actions.
        cell.window_control_area(caption.area())
    } else {
        cell.on_mouse_down(MouseButton::Left, |_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            match caption {
                Caption::Minimize => window.minimize_window(),
                Caption::Maximize | Caption::Restore => window.zoom_window(),
                Caption::Close => {
                    if on_close.as_ref().is_none_or(|request| request(window, cx)) {
                        window.remove_window();
                    }
                }
            }
        })
    }
}

fn caption_layer(row: impl IntoElement, window: &Window) -> AnyElement {
    deferred(
        anchored()
            .anchor(gpui::Anchor::TopRight)
            .position(point(window.viewport_size().width, px(0.)))
            .child(row),
    )
    .with_priority(CAPTION_PRIORITY)
    .into_any_element()
}

impl RenderOnce for TitleBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let spec = titlebar::bar(&FluentTheme::global(cx));
        let metrics = spec.metrics;
        let buttons = captions(window);
        let width = metrics.caption_width * buttons.len() as f32;
        let leading = if cfg!(target_os = "macos") && !window.is_fullscreen() {
            metrics.macos_padding_x
        } else {
            metrics.padding_x
        };
        let id = self.id.clone();
        let mut bar = div()
            .id(self.id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .h(metrics.height)
            .w_full()
            .pl(leading)
            .pr(width)
            // Include padding and the full height, not just the text box. The
            // deferred buttons occlude this area in GPUI's actual hit test.
            .window_control_area(WindowControlArea::Drag);
        #[cfg(test)]
        {
            bar = bar.debug_selector(|| "titlebar".into());
        }
        if !cfg!(target_os = "windows") {
            let intent = window.use_keyed_state(
                ElementId::NamedChild(id.clone().into(), "drag".into()),
                cx,
                |_, _| Rc::new(Cell::new(false)),
            );
            let press = intent.read(cx).clone();
            let release = press.clone();
            let movement = press.clone();
            bar = bar
                .on_mouse_down(MouseButton::Left, move |_, _, _| press.set(true))
                .on_mouse_up(MouseButton::Left, move |_, _, _| release.set(false))
                .on_mouse_down_out(move |_, _, _| movement.set(false));
            let movement = intent.read(cx).clone();
            bar = bar
                .on_mouse_move(move |event, window, _| {
                    if movement.replace(false) && event.dragging() {
                        window.start_window_move();
                    }
                })
                .on_click(|event, window, _| {
                    if event.click_count() == 2 {
                        if cfg!(target_os = "macos") {
                            window.titlebar_double_click();
                        } else if window.is_resizable() {
                            window.zoom_window();
                        }
                    }
                })
                .on_mouse_down(MouseButton::Right, |event, window, _| {
                    if cfg!(any(target_os = "linux", target_os = "freebsd")) {
                        window.show_window_menu(event.position);
                    }
                });
        }
        if let Some(mark) = self.mark {
            let art: AnyElement = match mark {
                TitleBarMark::Glyph(icon) => {
                    Icon::new(icon).size(metrics.icon_size).into_any_element()
                }
                TitleBarMark::Logo(image) => img(image)
                    .w(metrics.icon_size)
                    .h(metrics.icon_size)
                    .into_any_element(),
            };
            bar = bar.child(mark_slot(metrics.icon_size, metrics.gap, Some(art)));
        } else if self.reserve_mark {
            bar = bar.child(mark_slot(metrics.icon_size, metrics.gap, None));
        }
        let title = div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_size(metrics.title_font_size)
            .text_color(spec.colors.title)
            .child(self.title);
        #[cfg(test)]
        let title = title.debug_selector(|| "titlebar-title".into());
        bar = bar.child(title);
        if !buttons.is_empty() {
            let row =
                div()
                    .flex()
                    .h(metrics.height)
                    .items_center()
                    .children(buttons.into_iter().map(|caption| {
                        caption_cell(
                            id.clone(),
                            caption,
                            spec,
                            &self.labels,
                            cfg!(target_os = "windows"),
                            self.on_close.clone(),
                        )
                    }));
            bar = bar.child(caption_layer(row, window));
        }
        bar.refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests;
