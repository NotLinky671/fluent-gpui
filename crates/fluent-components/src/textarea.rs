//! Fluent 2 `Textarea`.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, div, px, relative,
};
use fluent_core::spec::ControlSize;
use fluent_core::spec::textarea::{self, TextareaAppearance, TextareaProps};
use fluent_theme::FluentTheme;
use gpui::Role;
use gpui::prelude::FluentBuilder as _;
use gpui_base::Textarea as BaseTextarea;
use gpui_base::input::{InputAccessibility, TextareaState};

/// What Tab and Shift+Tab do while a [`Textarea`] is focused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextareaTabBehavior {
    /// gpui-base's default: indent/outdent the text in place.
    #[default]
    Indent,
    /// Move focus to the next/previous control without touching the text.
    Navigate,
}

/// A Fluent 2 multi-line text field.
#[derive(IntoElement)]
pub struct Textarea {
    state: Entity<TextareaState>,
    props: TextareaProps,
    /// Explicit `disabled` from the builder; `None` defers to the state.
    disabled: Option<bool>,
    tab_behavior: TextareaTabBehavior,
    a11y_label: Option<SharedString>,
    style: StyleRefinement,
}

impl Textarea {
    /// Wraps a textarea state created with [`TextareaState::new`].
    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self {
            state: state.clone(),
            props: TextareaProps::default(),
            disabled: None,
            tab_behavior: TextareaTabBehavior::default(),
            a11y_label: None,
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

    /// Disables the field.
    ///
    /// The value is written to the [`TextareaState`], so a disabled field also
    /// rejects pointer focus, Tab traversal and edits, not just the styling.
    /// Without this builder the state's own flag (`set_disabled`) decides, and
    /// the component never re-enables a state the caller disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.props.invalid = invalid;
        self
    }

    /// Chooses whether Tab edits the text or moves focus.
    ///
    /// Defaults to [`TextareaTabBehavior::Indent`], gpui-base's existing
    /// behavior. Use [`TextareaTabBehavior::Navigate`] when the field is part of
    /// a normal form: Tab and Shift+Tab then leave the field without modifying
    /// its value, and no window-level key interception is needed.
    pub fn tab_behavior(mut self, behavior: TextareaTabBehavior) -> Self {
        self.tab_behavior = behavior;
        self
    }

    /// Sets the accessible name reported for this field.
    ///
    /// The name is merged into the text engine's accessibility state: a name
    /// provided by this builder wins over the state name, and other caller settings
    /// (placeholder, role, value visibility) are preserved. Without either, the
    /// field still exposes the multiline-text role and its value.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.a11y_label = Some(label.into());
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
        // See input.rs: an explicit builder value is pushed into the text
        // engine; otherwise the state's own flag wins.
        let state_disabled = self.state.read(cx).presentation().is_disabled();
        if let Some(disabled) = self.disabled
            && disabled != state_disabled
        {
            self.state
                .update(cx, |state, cx| state.set_disabled(disabled, cx));
        }
        let mut props = self.props;
        props.disabled = self.disabled.unwrap_or(state_disabled);

        let theme = FluentTheme::global(cx);
        let spec = textarea::resolve(&theme, &props);
        let m = spec.metrics;
        let focus_handle = self.state.read(cx).presentation().focus_handle().clone();
        let is_disabled = props.disabled;

        // Merge instead of replacing: the caller may already have set a
        // placeholder, a custom role or hidden value exposure on the state.
        // The builder's name overrides the state's name when present; every
        // other field is preserved. A projection with no role gets the
        // multiline role, and a missing projection gets one.
        let requested_label = self.a11y_label.clone();
        self.state.update(cx, |state, _| {
            let fallback_placeholder = state.presentation().placeholder().clone();
            let current = state.accessibility().map(|existing| {
                (
                    existing.role(),
                    existing.label().cloned(),
                    existing.placeholder().cloned(),
                    existing.accessibility_id().cloned(),
                    existing.value_exposed(),
                )
            });
            match current {
                Some((
                    existing_role,
                    existing_label,
                    existing_placeholder,
                    existing_id,
                    exposed,
                )) => {
                    let role = existing_role.or(Some(Role::MultilineTextInput));
                    let label = requested_label.clone().or(existing_label.clone());
                    let placeholder = existing_placeholder.clone().or(Some(fallback_placeholder));
                    // id and value exposure are preserved verbatim, so only
                    // role, name and placeholder can change. Reinstalling the
                    // same projection every frame would be needless churn.
                    if existing_role != role
                        || existing_label != label
                        || existing_placeholder != placeholder
                    {
                        state.set_accessibility(
                            InputAccessibility::new(role)
                                .with_label(label)
                                .with_placeholder(placeholder)
                                .with_accessibility_id(existing_id)
                                .with_value_exposed(exposed),
                        );
                    }
                }
                None => state.set_accessibility(
                    InputAccessibility::new(Some(Role::MultilineTextInput))
                        .with_label(requested_label.clone())
                        .with_placeholder(Some(fallback_placeholder)),
                ),
            }
        });

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
        } else if props.invalid {
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
            .when(self.tab_behavior == TextareaTabBehavior::Navigate, |el| {
                el.key_context(crate::TEXT_INPUT_NAVIGATE_KEY_CONTEXT)
            })
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
            // Read the live flag: a disabled field must not take focus.
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                weak_state
                    .update(cx, |state, cx| {
                        if !state.presentation().is_disabled() {
                            state.focus(window, cx);
                        }
                    })
                    .ok();
            })
            // Same gap as input.rs: neither GPUI nor the text engine clears focus
            // when the pointer lands elsewhere.
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
            // Capture runs before the text engine's tracked-focus default.
            .capture_any_mouse_down({
                let state = self.state.downgrade();
                move |_, window, cx| {
                    if state
                        .upgrade()
                        .is_some_and(|state| state.read(cx).presentation().is_disabled())
                    {
                        window.prevent_default();
                    }
                }
            })
            .refine_style(&self.style);

        if !is_disabled {
            crate::interaction::hover(root.interactivity(), hover_state);
        }

        root = root.child(
            div()
                .flex_1()
                .min_h(px(0.))
                .child(BaseTextarea::new(&self.state)),
        );

        let bottom_edge = if is_disabled {
            disabled.border_bottom
        } else if props.invalid {
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

#[cfg(test)]
mod ax_tests;
#[cfg(test)]
mod disabled_tests;

#[cfg(test)]
mod tab_tests;
