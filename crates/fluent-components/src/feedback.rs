//! Fluent 2 `Badge`, `ProgressBar` and `Avatar`.

use fluent_core::gpui_compat::StyledExt as _;
use std::time::{Duration, Instant};

use fluent_core::gpui_compat::{
    App, Context, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, div, px, relative,
};
use fluent_core::spec::feedback::{
    self, AvatarSize, BadgeAppearance, BadgeSize, ProgressAppearance, SpinnerSize,
};
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyView, AppContext as _, ElementId, InteractiveElement as _, PathBuilder, Render,
    StatefulInteractiveElement as _, canvas, point,
};

/// A Fluent 2 badge.
#[derive(IntoElement)]
pub struct Badge {
    text: Option<SharedString>,
    appearance: BadgeAppearance,
    size: BadgeSize,
    style: StyleRefinement,
}

impl Badge {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: Some(text.into()),
            appearance: BadgeAppearance::Brand,
            size: BadgeSize::Medium,
            style: StyleRefinement::default(),
        }
    }

    /// A badge with no text, e.g. a status dot.
    pub fn dot() -> Self {
        Self {
            text: None,
            ..Self::new("")
        }
    }

    pub fn appearance(mut self, appearance: BadgeAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }
}

impl Styled for Badge {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = feedback::badge(&theme, self.appearance, self.size);
        div()
            .h(spec.height)
            .min_w(spec.min_width)
            .px(spec.padding_x)
            .rounded(px(10000.))
            .bg(spec.background)
            .text_color(spec.foreground)
            .text_size(spec.font_size)
            .line_height(spec.height)
            .flex()
            .items_center()
            .justify_center()
            .when_some(self.text, |el, text| el.child(text))
            .refine_style(&self.style)
    }
}

/// A Fluent 2 progress bar.
#[derive(IntoElement)]
pub struct ProgressBar {
    value: f32,
    appearance: ProgressAppearance,
    indeterminate: bool,
    style: StyleRefinement,
}

impl ProgressBar {
    /// `value` is a percentage in 0..=100.
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0., 100.),
            appearance: ProgressAppearance::Default,
            indeterminate: false,
            style: StyleRefinement::default(),
        }
    }

    pub fn appearance(mut self, appearance: ProgressAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// An unknown-duration progress: `value` is ignored and a segment sweeps
    /// across the track on a loop.
    pub fn indeterminate(mut self) -> Self {
        self.indeterminate = true;
        self
    }
}

impl Styled for ProgressBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ProgressBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = feedback::progress(&theme, self.appearance);

        let track = div()
            .relative()
            .h(spec.height)
            .w_full()
            .rounded(spec.radius)
            .bg(spec.track)
            .refine_style(&self.style);

        if self.indeterminate {
            // Fluent's indeterminate bar sweeps across on a loop: a fixed-width
            // segment whose insets travel from off the left edge to off the
            // right.
            //
            // The phase is read from the clock on every frame rather than from
            // `with_animation`: in this GPUI snapshot an animation's progress
            // never advances (the menu reveal hit the same wall and had to be
            // driven by time), so an animated bar would simply stand still.
            //
            // The policy is explicit for reduced motion: the same segment is
            // parked at the fixed reduced-motion inset with no loop.
            let segment = move |left_edge: f32, right_edge: f32| {
                // Clamping both edges to the track keeps the segment inside
                // without a clip, which would square off the rounded ends.
                div()
                    .absolute()
                    .left(relative(left_edge))
                    .right(relative(1.0 - right_edge))
                    .top(px(0.))
                    .bottom(px(0.))
                    .rounded(spec.radius)
                    .bg(spec.bar)
            };
            if !feedback::indeterminate_animation_allowed(cx.reduce_motion()) {
                let (left_edge, right_edge) = feedback::indeterminate_segment_reduced_insets();
                return track
                    .child(segment(left_edge, right_edge))
                    .into_any_element();
            }
            let id = ElementId::Name(format!("progress-{:?}", self.appearance).into());
            let phase = loop_phase(window, cx, &id, INDETERMINATE_PERIOD);
            let (left_edge, right_edge) = feedback::indeterminate_segment_insets(phase);
            return track
                .child(segment(left_edge, right_edge))
                .into_any_element();
        }

        track
            .child(
                div()
                    .h_full()
                    .w(relative(self.value / 100.0))
                    .rounded(spec.radius)
                    .bg(spec.bar),
            )
            .into_any_element()
    }
}

/// How long one indeterminate sweep takes.
///
/// Approximation: the upstream stylesheet does not publish this period as a
/// token, and no official recording has been measured yet. The value is kept
/// from the previous implementation so the change under review only alters
/// whether the bar moves at all.
const INDETERMINATE_PERIOD: Duration = Duration::from_millis(2000);

/// The phase (0..1) of a looping animation, driven by the wall clock.
///
/// Each id keeps its own start instant, so two spinners that appear at
/// different times are not forced into the same phase — matching CSS, where an
/// animation starts when its element is created. Every frame that is still
/// animating asks for the next one; without that request a `RenderOnce`
/// component would be painted once and never again.
fn loop_phase(window: &mut Window, cx: &mut App, id: &ElementId, period: Duration) -> f32 {
    let state = window.use_keyed_state(id.clone(), cx, |_, _| LoopStart(None));
    let now = cx.background_executor().now();
    let start = match state.read(cx).0 {
        Some(start) => start,
        None => {
            state.update(cx, |slot, _| slot.0 = Some(now));
            now
        }
    };
    window.request_animation_frame();
    let elapsed = now.saturating_duration_since(start).as_secs_f32();
    (elapsed / period.as_secs_f32()).fract()
}

/// Start instant of a looping animation, stored per element id.
struct LoopStart(Option<Instant>);

/// A Fluent 2 spinner: a stroked ring with a sweeping brand arc.
#[derive(IntoElement)]
pub struct Spinner {
    id: Option<ElementId>,
    size: SpinnerSize,
    label: Option<SharedString>,
    style: StyleRefinement,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            id: None,
            size: SpinnerSize::Medium,
            label: None,
            style: StyleRefinement::default(),
        }
    }

    /// An explicit element id. Only needed when the derived default
    /// (`spinner-{size}-{label}`) could collide, e.g. two spinners that share
    /// both size and label.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }

    /// Accessible name announced by assistive technology.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Spinner {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Spinner {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = feedback::spinner(&theme, self.size);

        // `div()` becomes a StatefulInteractiveElement once it carries an id;
        // the id is also what accesskit needs before `role`/`aria_label`
        // produce a node. A shared literal id would collide when more than one
        // spinner is on screen, so the default derives from size + label and
        // callers can still override with `Spinner::id`.
        let id = self
            .id
            .unwrap_or_else(|| spinner_element_id(self.size, self.label.as_ref()));
        let mut el = div()
            .id(id.clone())
            .size(spec.diameter)
            .role(gpui::accesskit::Role::ProgressIndicator);
        if let Some(label) = self.label {
            el = el.aria_label(label);
        }
        let el = el.refine_style(&self.style);

        // The phase comes from the clock, not from `with_animation`: in this
        // GPUI snapshot an animation's progress never advances, so the ring
        // used to be painted once and never move again.
        let (start, sweep) = if feedback::spinner_animation_allowed(cx.reduce_motion()) {
            let ease = theme.tokens().curve.easy_ease;
            let phase = loop_phase(window, cx, &id, feedback::SPINNER_PERIOD);
            feedback::spinner_arc(phase, ease)
        } else {
            // Reduced motion freezes the ring on the static two-thirds arc.
            // Upstream only slows the ring here (1.5s -> 1.8s) but keeps
            // turning it; an object that never stops moving is exactly what
            // the preference exists to suppress.
            feedback::spinner_reduced_arc()
        };

        el.child(
            // A canvas has no intrinsic size: without this it gets the parent's
            // width and zero height, and `paint_spinner` returns early on a
            // zero radius — the ring would never be painted at all.
            canvas(
                move |_, _, _| (),
                move |bounds, _, window, _| paint_spinner(bounds, spec, start, sweep, window),
            )
            .size_full(),
        )
        .into_any_element()
    }
}

/// Paints the track ring and the indicator arc.
///
/// `start` and `sweep` are degrees clockwise from 12 o'clock, both resolved by
/// [`fluent_core::spec::feedback::spinner_arc`] so the shape stays on the
/// measured upstream keyframes instead of being re-derived here.
fn paint_spinner(
    bounds: gpui::Bounds<Pixels>,
    spec: feedback::SpinnerSpec,
    start: f32,
    sweep: f32,
    window: &mut Window,
) {
    let center = bounds.center();
    let radius = (bounds.size.width.min(bounds.size.height) - spec.stroke_width) / 2.0;
    if radius <= px(0.) {
        return;
    }

    // Track: a full stroked circle drawn as two half arcs.
    let mut track = PathBuilder::stroke(spec.stroke_width);
    let right = point(center.x + radius, center.y);
    let left = point(center.x - radius, center.y);
    track.move_to(right);
    track.arc_to(point(radius, radius), px(0.), true, true, left);
    track.arc_to(point(radius, radius), px(0.), true, true, right);
    if let Ok(path) = track.build() {
        window.paint_path(path, spec.track);
    }

    // Indicator: an arc that both turns and stretches. The sweep changes every
    // frame, so the large-arc flag has to follow it; hard-coding it would make
    // the ring snap the moment the arc passed 180 degrees.
    let (sx, sy) = circle_point(start);
    let (ex, ey) = circle_point(start + sweep);
    let mut arc = PathBuilder::stroke(spec.stroke_width);
    arc.move_to(point(center.x + radius * sx, center.y + radius * sy));
    arc.arc_to(
        point(radius, radius),
        px(0.),
        sweep > 180.0,
        true,
        point(center.x + radius * ex, center.y + radius * ey),
    );
    if let Ok(path) = arc.build() {
        window.paint_path(path, spec.indicator);
    }
}

/// Point on a unit circle at `degrees` clockwise from 12 o'clock (y down).
fn circle_point(degrees: f32) -> (f32, f32) {
    let rad = degrees.to_radians();
    (rad.sin(), -rad.cos())
}

/// Default element id: `spinner-{size}-{label}`.
///
/// Pure so callers can predict it; `Spinner::id` overrides. Two spinners that
/// share both size and label still collide, which is why the explicit builder
/// exists.
fn spinner_element_id(size: SpinnerSize, label: Option<&SharedString>) -> ElementId {
    let name = match label {
        Some(label) if !label.is_empty() => {
            format!("spinner-{size:?}-{}", label.as_ref())
        }
        _ => format!("spinner-{size:?}"),
    };
    ElementId::Name(name.into())
}

/// A Fluent 2 avatar showing initials.
#[derive(IntoElement)]
pub struct Avatar {
    name: SharedString,
    size: AvatarSize,
    style: StyleRefinement,
}

impl Avatar {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            size: AvatarSize::Medium,
            style: StyleRefinement::default(),
        }
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }
}

impl Styled for Avatar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = feedback::avatar(&theme, self.size);
        let initials = initials(&self.name);
        div()
            .size(spec.size)
            .rounded(px(10000.))
            .bg(spec.background)
            .text_color(spec.foreground)
            .text_size(spec.font_size)
            .font_weight(theme.tokens().font_weight.semibold)
            .line_height(spec.size)
            .flex()
            .items_center()
            .justify_center()
            .child(initials)
            .refine_style(&self.style)
    }
}

/// The Fluent tooltip surface. Rendered by [`TooltipView`] and reusable as a
/// static preview.
#[derive(IntoElement)]
pub struct TooltipSurface {
    text: SharedString,
}

impl TooltipSurface {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl RenderOnce for TooltipSurface {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = feedback::tooltip(&theme);
        div()
            .bg(spec.background)
            .text_color(spec.foreground)
            .text_size(spec.font_size)
            .line_height(spec.line_height)
            .px(spec.padding_x)
            .pt(spec.padding_top)
            .pb(spec.padding_bottom)
            .rounded(spec.radius)
            .max_w(spec.max_width)
            .shadow(spec.shadow.to_vec())
            .child(self.text)
    }
}

/// The tooltip view GPUI instantiates for `InteractiveElement::tooltip`.
pub struct TooltipView {
    text: SharedString,
}

impl TooltipView {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl Render for TooltipView {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        TooltipSurface::new(self.text.clone())
    }
}

/// Builds the closure expected by `InteractiveElement::tooltip`.
///
/// ```ignore
/// div().id("save").tooltip(tooltip("Save document")).child(button)
/// ```
pub fn tooltip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| TooltipView { text: text.clone() }).into()
}

/// First letter of the first and last words, uppercased.
fn initials(name: &str) -> SharedString {
    let words: Vec<&str> = name.split_whitespace().collect();
    let text = match (words.first(), words.last()) {
        (Some(first), Some(last)) if words.len() > 1 => {
            format!(
                "{}{}",
                first.chars().next().unwrap_or(' '),
                last.chars().next().unwrap_or(' ')
            )
        }
        (Some(only), _) => only.chars().take(1).collect(),
        _ => String::new(),
    };
    text.to_uppercase().into()
}

#[cfg(test)]
mod tests;
