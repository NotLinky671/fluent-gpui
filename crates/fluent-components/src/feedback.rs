//! Fluent 2 `Badge`, `ProgressBar` and `Avatar`.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    div, px, relative,
};
use fluent_core::spec::feedback::{
    self, AvatarSize, BadgeAppearance, BadgeSize, ProgressAppearance,
};
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;

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
    style: StyleRefinement,
}

impl ProgressBar {
    /// `value` is a percentage in 0..=100.
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0., 100.),
            appearance: ProgressAppearance::Default,
            style: StyleRefinement::default(),
        }
    }

    pub fn appearance(mut self, appearance: ProgressAppearance) -> Self {
        self.appearance = appearance;
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
        div()
            .h(spec.height)
            .w_full()
            .rounded(spec.radius)
            .bg(spec.track)
            .child(
                div()
                    .h_full()
                    .w(relative(self.value / 100.0))
                    .rounded(spec.radius)
                    .bg(spec.bar),
            )
            .refine_style(&self.style)
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
