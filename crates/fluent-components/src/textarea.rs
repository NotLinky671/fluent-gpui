//! Fluent 2 `Textarea`.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, Entity, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels,
    RenderOnce, StyleRefinement, Styled, Window, div, px,
};
use fluent_core::spec::ControlSize;
use fluent_core::spec::textarea::{self, TextareaAppearance, TextareaProps};
use fluent_theme::FluentTheme;
use gpui_base::Textarea as BaseTextarea;
use gpui_base::input::TextareaState;

/// A Fluent 2 multi-line text field.
#[derive(IntoElement)]
pub struct Textarea {
    state: Entity<TextareaState>,
    props: TextareaProps,
    style: StyleRefinement,
}

impl Textarea {
    /// Wraps a textarea state created with [`TextareaState::new`].
    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self {
            state: state.clone(),
            props: TextareaProps::default(),
            style: StyleRefinement::default(),
        }
    }

    pub fn appearance(mut self, appearance: TextareaAppearance) -> Self {
        self.props.appearance = appearance;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.props.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.props.invalid = invalid;
        self
    }
}

impl Styled for Textarea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

fn overlay_line(color: Hsla, height: Pixels) -> impl IntoElement {
    div()
        .absolute()
        .left(px(0.))
        .right(px(0.))
        .bottom(px(0.))
        .h(height)
        .bg(color)
}

impl RenderOnce for Textarea {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = textarea::resolve(&theme, &self.props);
        let m = spec.metrics;
        let focus_handle = self.state.read(cx).presentation().focus_handle().clone();
        let is_disabled = self.props.disabled;

        let rest = spec.rest;
        let hover = spec.hover;
        let disabled = spec.disabled;
        let invalid = spec.invalid_border;
        let underline = spec.focus_underline;
        let text_color = if is_disabled {
            spec.disabled_text
        } else {
            spec.text
        };
        let border = if is_disabled {
            disabled.border
        } else if self.props.invalid {
            invalid
        } else {
            rest.border
        };

        let weak_state = self.state.downgrade();

        let mut root = div()
            .relative()
            .flex()
            .flex_col()
            .min_h(m.min_height)
            .w_full()
            .bg(if is_disabled {
                disabled.background
            } else {
                rest.background
            })
            .border_1()
            .border_color(border)
            .rounded(m.radius)
            .px(m.padding_x)
            .py(m.padding_y)
            .text_size(m.font_size)
            .line_height(m.line_height)
            .text_color(text_color)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                weak_state
                    .update(cx, |state, cx| state.focus(window, cx))
                    .ok();
            })
            .refine_style(&self.style);

        if !is_disabled {
            root = root.hover(move |s| {
                s.bg(hover.background).border_color(if self.props.invalid {
                    invalid
                } else {
                    hover.border
                })
            });
        }

        root = root.child(
            div()
                .flex_1()
                .min_h(px(0.))
                .child(BaseTextarea::new(&self.state)),
        );

        let bottom_edge = if is_disabled {
            disabled.border_bottom
        } else if self.props.invalid {
            invalid
        } else {
            rest.border_bottom
        };
        root = root.child(overlay_line(bottom_edge, m.border_width));

        if !is_disabled {
            root = root.child(
                div()
                    .absolute()
                    .left(px(0.))
                    .right(px(0.))
                    .bottom(px(0.))
                    .h(m.underline_height)
                    .track_focus(&focus_handle)
                    .focus(move |s| s.bg(underline)),
            );
        }

        root
    }
}
