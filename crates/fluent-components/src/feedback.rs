//! Fluent 2 `Badge`, `ProgressBar` and `Avatar`.

use fluent_core::gpui_compat::StyledExt as _;
use std::time::Duration;

use fluent_core::gpui_compat::{
    App, Context, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled,
    Window, div, px, relative,
};
use fluent_core::spec::feedback::{
    self, AvatarSize, BadgeAppearance, BadgeSize, ProgressAppearance,
};
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui::{Animation, AnimationExt, AnyView, AppContext as _, ElementId, Render};

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
            // segment whose insets travel from off the left edge to off the right.
            // The phase comes from a repeating animation rather than a transition
            // because there is no target state to settle on.
            let animation = Animation::new(Duration::from_millis(2000)).repeat();
            let id = ElementId::Name(format!("progress-{:?}", self.appearance).into());
            return track
                .with_animation(id, animation, move |track, delta| {
                    let travel = -0.4 + delta * 1.4;
                    track.child(
                        div()
                            .absolute()
                            .left(relative(travel))
                            .right(relative(0.6 - travel))
                            .top(px(0.))
                            .bottom(px(0.))
                            .rounded(spec.radius)
                            .bg(spec.bar),
                    )
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
