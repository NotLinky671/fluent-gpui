//! Fluent 2 `Badge`, `ProgressBar` and `Avatar`.

use fluent_core::gpui_compat::StyledExt as _;
use std::time::Duration;

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
    Animation, AnimationExt, AnyView, AppContext as _, ElementId, InteractiveElement as _,
    PathBuilder, Render, StatefulInteractiveElement as _, canvas, point,
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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
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
            // right. The phase comes from a repeating animation rather than a
            // transition because there is no target state to settle on.
            //
            // Raw `Animation` does not read `cx.reduce_motion()`, so the
            // policy is explicit: under reduced motion the same segment is
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
            let animation = Animation::new(Duration::from_millis(2000)).repeat();
            let id = ElementId::Name(format!("progress-{:?}", self.appearance).into());
            return track
                .with_animation(id, animation, move |track, delta| {
                    let (left_edge, right_edge) = feedback::indeterminate_segment_insets(delta);
                    track.child(segment(left_edge, right_edge))
                })
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

/// How long one spinner revolution takes.
///
/// Approximation: Fluent's spinner period is not a published token; ~1.5s per
/// turn matches the perceived pace of the upstream component. Verify against
/// official recordings.
const SPINNER_PERIOD: Duration = Duration::from_millis(1500);

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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = feedback::spinner(&theme, self.size);

        // The whole indicator is one painted canvas: a faint full-circle track
        // plus the brand arc. `PathBuilder::stroke` gives round caps; rotating
        // the start angle by `delta * 360°` per frame gives the sweep.
        let ring = move |delta: f32| {
            canvas(
                move |_, _, _| (),
                move |bounds, _, window, _| {
                    paint_spinner(bounds, spec, delta, window);
                },
            )
        };

        // `div()` becomes a StatefulInteractiveElement once it carries an id;
        // the id is also what accesskit needs before `role`/`aria_label`
        // produce a node. A shared literal id would collide when more than one
        // spinner is on screen, so the default derives from size + label and
        // callers can still override with `Spinner::id`.
        let id = self
            .id
            .unwrap_or_else(|| spinner_element_id(self.size, self.label.as_ref()));
        let mut el = div()
            .id(id)
            .size(spec.diameter)
            .role(gpui::accesskit::Role::ProgressIndicator);
        if let Some(label) = self.label {
            el = el.aria_label(label);
        }
        let el = el.refine_style(&self.style);

        if feedback::spinner_animation_allowed(cx.reduce_motion()) {
            // `repeat_synced` keeps every spinner on screen phase-locked, like
            // Fluent where all spinners share one clock.
            let animation = Animation::new(SPINNER_PERIOD).repeat_synced();
            return el
                .with_animation("spin", animation, move |el, delta| el.child(ring(delta)))
                .into_any_element();
        }

        // Reduced motion: same ring + arc, frozen at phase 0, no loop.
        el.child(ring(0.0)).into_any_element()
    }
}

/// Paints the track ring and the indicator arc for one phase.
fn paint_spinner(
    bounds: gpui::Bounds<Pixels>,
    spec: feedback::SpinnerSpec,
    delta: f32,
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

    // Indicator: the arc endpoints rotate with the phase.
    let [(sx, sy), (ex, ey)] = feedback::spinner_arc_endpoints(delta);
    let start = point(center.x + radius * sx, center.y + radius * sy);
    let end = point(center.x + radius * ex, center.y + radius * ey);
    let mut arc = PathBuilder::stroke(spec.stroke_width);
    arc.move_to(start);
    arc.arc_to(
        point(radius, radius),
        px(0.),
        feedback::SPINNER_ARC_DEGREES > 180.0,
        true,
        end,
    );
    if let Ok(path) = arc.build() {
        window.paint_path(path, spec.indicator);
    }
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
