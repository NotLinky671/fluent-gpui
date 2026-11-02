//! Fluent 2 `Textarea`.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px, relative,
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

impl RenderOnce for Textarea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
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
        // Hover is tracked per textarea so the chrome eases instead of snapping.
        let hover_state = window.use_keyed_state(
            SharedString::from(format!("textarea-hovered-{:?}", self.state.entity_id())),
            cx,
            |_, _| false,
        );
        let hovered = *hover_state.read(cx) && !is_disabled;

        let target_border = if is_disabled {
            disabled.border
        } else if self.props.invalid {
            invalid
        } else if hovered {
            hover.border
        } else {
            rest.border
        };
        let target_background = if is_disabled {
            disabled.background
        } else if hovered {
            hover.background
        } else {
            rest.background
        };
        let border = crate::motion::state_color(
            SharedString::from(format!("textarea-border-{:?}", self.state.entity_id())),
            target_border,
            window,
            cx,
        );
        let background = crate::motion::state_color(
            SharedString::from(format!("textarea-bg-{:?}", self.state.entity_id())),
            target_background,
            window,
            cx,
        );

        let weak_state = self.state.downgrade();

        let root_id = SharedString::from(format!("textarea-root-{:?}", self.state.entity_id()));
        let mut root = div()
            .id(root_id)
            .relative()
            .flex()
            .flex_col()
            .min_h(m.min_height)
            .w_full()
            .bg(background)
            // See input.rs: the bottom edge is an overlay so the focus
            // underline can replace it, and both follow the corner radius.
            .border_t_1()
            .border_l_1()
            .border_r_1()
            .border_color(border)
            .pb(m.border_width)
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
            let hover_listener = hover_state.clone();
            root = root.on_hover(move |is_hovered, _, cx| {
                hover_listener.update(cx, |state, cx| {
                    if *state != *is_hovered {
                        *state = *is_hovered;
                        cx.notify();
                    }
                });
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
        // Bottom edge as a border on a rounded overlay: GPUI clips rectangles
        // only, so a filled bar would end straight instead of turning the corner.
        root = root.child(
            div()
                .absolute()
                .left(px(0.))
                .right(px(0.))
                .top(px(0.))
                .bottom(px(0.))
                .rounded(m.radius)
                .border_b(m.border_width)
                .border_color(bottom_edge),
        );

        if !is_disabled {
            // Fluent grows the focus underline out of the centre: 200 ms in,
            // 50 ms out. Insetting both edges from half the width reproduces the
            // scaleX transform without knowing the pixel width.
            let focused = focus_handle.is_focused(window);
            let policy = if focused {
                fluent_core::spec::motion::reveal_policy()
            } else {
                fluent_core::spec::motion::hide_policy()
            };
            let inset = crate::motion::fraction(
                SharedString::from(format!("textarea-underline-{:?}", self.state.entity_id())),
                if focused { 0.0 } else { 0.5 },
                policy,
                window,
                cx,
            );
            root = root.child(
                div()
                    .absolute()
                    .left(relative(inset))
                    .right(relative(inset))
                    .top(px(0.))
                    .bottom(px(0.))
                    .rounded(m.radius)
                    .border_b(m.underline_height)
                    .border_color(underline),
            );
        }

        root
    }
}
