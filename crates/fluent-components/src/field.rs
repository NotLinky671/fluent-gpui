//! Fluent 2 `Label` and `Field`.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, FocusHandle, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, StyleRefinement, Styled, Window, div, px,
    relative,
};
use fluent_core::spec::ControlSize;
use fluent_core::spec::field::{self, FieldOrientation, FieldSpec};
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;

/// A validation message shown under a field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationMessage {
    Error(SharedString),
    Warning(SharedString),
    Success(SharedString),
}

impl ValidationMessage {
    fn text(&self) -> &SharedString {
        match self {
            Self::Error(text) | Self::Warning(text) | Self::Success(text) => text,
        }
    }

    fn color(&self, spec: &FieldSpec) -> Hsla {
        match self {
            Self::Error(_) => spec.error_color,
            Self::Warning(_) => spec.warning_color,
            Self::Success(_) => spec.success_color,
        }
    }
}

/// A Fluent 2 form label.
#[derive(IntoElement)]
pub struct Label {
    text: SharedString,
    required: bool,
    disabled: bool,
    size: ControlSize,
    focus: Option<FocusHandle>,
    style: StyleRefinement,
}

impl Label {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            required: false,
            disabled: false,
            size: ControlSize::Medium,
            focus: None,
            style: StyleRefinement::default(),
        }
    }

    /// Appends the red required asterisk.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Uses the disabled foreground.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Clicking the label focuses the control that owns this handle.
    pub fn for_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus = Some(focus_handle.clone());
        self
    }
}

impl Styled for Label {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Label {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = field::resolve(&theme, self.size);
        let handle = self.focus.clone();

        div()
            .flex()
            .items_center()
            .text_size(spec.label_font_size)
            .line_height(spec.label_line_height)
            .font_weight(theme.tokens().font_weight.semibold)
            .text_color(if self.disabled {
                spec.label_disabled_color
            } else {
                spec.label_color
            })
            .when(handle.is_some(), |el| el.cursor_pointer())
            .when_some(handle, |el, handle| {
                el.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.focus(&handle, cx);
                })
            })
            .child(self.text)
            .when(self.required, |el| {
                el.child(div().pl(px(4.)).text_color(spec.required_color).child("*"))
            })
            .refine_style(&self.style)
    }
}

/// A Fluent 2 form field: label, control, optional hint and validation message.
#[derive(IntoElement)]
pub struct Field {
    label: Option<SharedString>,
    required: bool,
    disabled: bool,
    size: ControlSize,
    hint: Option<SharedString>,
    validation: Option<ValidationMessage>,
    orientation: FieldOrientation,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Field {
    /// A field with a label above the control.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: Some(label.into()),
            required: false,
            disabled: false,
            size: ControlSize::Medium,
            hint: None,
            validation: None,
            orientation: FieldOrientation::Vertical,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// A field without a label (keeps the horizontal gutter aligned).
    pub fn unlabeled() -> Self {
        Self {
            label: None,
            ..Self::new("")
        }
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Hint text shown below the control.
    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Validation message shown below the control and hint.
    pub fn validation_message(mut self, message: ValidationMessage) -> Self {
        self.validation = Some(message);
        self
    }

    /// Label beside the control instead of above it.
    pub fn horizontal(mut self) -> Self {
        self.orientation = FieldOrientation::Horizontal;
        self
    }
}

impl Styled for Field {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Field {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

fn message_line(text: &SharedString, color: Hsla, gap: Pixels) -> impl IntoElement {
    div()
        .mt(gap)
        .text_size(px(12.))
        .line_height(px(16.))
        .text_color(color)
        .child(text.clone())
}

impl RenderOnce for Field {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = field::resolve(&theme, self.size);

        let label = self.label.map(|text| {
            Label::new(text)
                .required(self.required)
                .disabled(self.disabled)
                .size(self.size)
        });

        let mut body = div().flex().flex_col().flex_1().children(self.children);
        if let Some(hint) = &self.hint {
            body = body.child(message_line(hint, spec.hint_color, spec.hint_gap));
        }
        if let Some(validation) = &self.validation {
            body = body.child(message_line(
                validation.text(),
                validation.color(&spec),
                spec.message_gap,
            ));
        }

        match self.orientation {
            FieldOrientation::Vertical => {
                let mut root = div().flex().flex_col().refine_style(&self.style);
                if let Some(label) = label {
                    root = root.child(label.mb(spec.label_gap));
                }
                root.child(body)
            }
            FieldOrientation::Horizontal => {
                let mut root = div()
                    .flex()
                    .items_start()
                    .gap(spec.horizontal_gap)
                    .refine_style(&self.style);
                root = root.child(
                    div()
                        .w(relative(spec.horizontal_label_fraction))
                        .flex_shrink_0()
                        .when_some(label, |el, label| el.child(label.pt(px(6.)))),
                );
                root.child(body)
            }
        }
    }
}
