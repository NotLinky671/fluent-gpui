//! Fluent 2 `TextInput`.
//!
//! The text engine, IME, selection and cursor come from
//! `gpui_base::input::InputState`; this file only draws Fluent chrome around it
//! and forwards clicks on the padding to the field.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
    relative,
};
use fluent_core::spec::ControlSize;
use fluent_core::spec::input::{self, InputAppearance, InputProps};
use fluent_theme::FluentTheme;
use gpui_base::Input;
use gpui_base::input::InputState;

/// A Fluent 2 single-line text field.
///
/// ```
/// use fluent_components::TextInput;
/// use gpui::{AppContext as _, Window};
/// use gpui_base::input::InputState;
///
/// fn build(window: &mut Window, cx: &mut gpui::App) {
///     let state = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
///     let _field = TextInput::new(&state);
/// }
/// ```
#[derive(IntoElement)]
pub struct TextInput {
    state: Entity<InputState>,
    props: InputProps,
    content_before: Option<AnyElement>,
    content_after: Option<AnyElement>,
    style: StyleRefinement,
}

impl TextInput {
    /// Wraps an input state created with [`InputState::new`].
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            props: InputProps::default(),
            content_before: None,
            content_after: None,
            style: StyleRefinement::default(),
        }
    }

    /// Fluent `appearance` prop.
    pub fn appearance(mut self, appearance: InputAppearance) -> Self {
        self.props.appearance = appearance;
        self
    }

    /// Fluent `size` prop.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.props.size = size;
        self
    }

    /// Disables the field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Shows the error border while the field is not focused.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.props.invalid = invalid;
        self
    }

    /// Content before the text, such as an icon.
    pub fn content_before(mut self, element: impl IntoElement) -> Self {
        self.content_before = Some(element.into_any_element());
        self
    }

    /// Content after the text, such as a unit or a clear button.
    pub fn content_after(mut self, element: impl IntoElement) -> Self {
        self.content_after = Some(element.into_any_element());
        self
    }
}

impl Styled for TextInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TextInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = input::resolve(&theme, &self.props);
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
        // Hover is tracked per input so the border and fill ease instead of
        // snapping, the same way the selection controls do it.
        let hover_key = SharedString::from(format!("input-hovered-{:?}", self.state.entity_id()));
        let hover_state = window.use_keyed_state(hover_key, cx, |_, _| false);
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
            SharedString::from(format!("input-border-{:?}", self.state.entity_id())),
            target_border,
            window,
            cx,
        );
        let background = crate::motion::state_color(
            SharedString::from(format!("input-bg-{:?}", self.state.entity_id())),
            target_background,
            window,
            cx,
        );

        let weak_state = self.state.downgrade();

        // A stateful root: hover listeners need an element id.
        let root_id = SharedString::from(format!("input-root-{:?}", self.state.entity_id()));
        let mut root = div()
            .id(root_id)
            .relative()
            .flex()
            .items_center()
            .h(m.height)
            .w_full()
            .bg(background)
            // No bottom border: the bottom edge is an overlay so the focus
            // underline can replace it, and clipping to the radius makes both
            // wrap the corners the way Fluent's clip-path does.
            .border_t_1()
            .border_l_1()
            .border_r_1()
            .border_color(border)
            // Replaces the removed bottom border so the content box keeps the
            // exact size it had before the edge became an overlay.
            .pb(m.border_width)
            .rounded(m.radius)
            .px(m.padding_x)
            .gap(m.gap)
            .text_size(m.font_size)
            .line_height(m.line_height)
            .text_color(text_color)
            // Clicking the padding should focus the field, not just the text.
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                weak_state
                    .update(cx, |state, cx| state.focus(window, cx))
                    .ok();
            })
            // GPUI does not drop focus when the pointer lands elsewhere, and
            // gpui-base's text input does not do it either, so without this a
            // field stays focused (and keeps drawing its underline) after the
            // user clicks away.
            .on_mouse_down_out({
                let weak_state = self.state.downgrade();
                move |_, window, cx| {
                    let Some(state) = weak_state.upgrade() else {
                        return;
                    };
                    if state
                        .read(cx)
                        .presentation()
                        .focus_handle()
                        .is_focused(window)
                    {
                        window.blur(cx);
                    }
                }
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

        if let Some(before) = self.content_before {
            root = root.child(before);
        }

        root = root.child(div().flex_1().min_w(px(0.)).child(Input::new(&self.state)));

        if let Some(after) = self.content_after {
            root = root.child(after);
        }

        // The heavier bottom edge Fluent draws on outline/underline fields.
        // An invalid field paints every edge red, including this one.
        let bottom_edge = if is_disabled {
            disabled.border_bottom
        } else if self.props.invalid {
            invalid
        } else {
            rest.border_bottom
        };
        // Bottom edge painted as a border on a rounded overlay: GPUI clips
        // rectangles only, so a filled bar would end in a straight line, while a
        // border follows the element's corner radius — which is how Fluent's
        // clip-path version ends up looking.
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

        // Focus underline: present but transparent until the field is focused.
        // `track_focus` + `focus()` gives us focus-without-modality styling.
        if !is_disabled {
            // Fluent grows the focus underline out of the centre with a scaleX
            // transform: 200 ms decelerating in, 50 ms accelerating out. Insetting
            // both edges from half the width reproduces it without knowing the
            // field's pixel width.
            let focused = focus_handle.is_focused(window);
            let policy = if focused {
                fluent_core::spec::motion::reveal_policy()
            } else {
                fluent_core::spec::motion::hide_policy()
            };
            let inset = crate::motion::fraction(
                SharedString::from(format!("input-underline-{:?}", self.state.entity_id())),
                if focused { 0.0 } else { 0.5 },
                policy,
                window,
                cx,
            );
            // Fluent draws this as a 2px bottom border on an element as tall as
            // the corner radius, then clips everything but the border away. A
            // filled bar with only its bottom corners rounded is the same shape
            // without the trimming step.
            // The outer strip is the field's border box grown by the 1px border,
            // matching Fluent's `left/right/bottom: -1px`. The inner bar keeps the
            // animated insets, which are fractions of the strip.
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

#[cfg(test)]
mod focus_tests;
#[cfg(test)]
mod tab_order_tests;
