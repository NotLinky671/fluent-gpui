//! Fluent 2 `Checkbox` and `Radio`.

use crate::icon::Icon;
use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled,
    Window, div, px,
};
use fluent_core::spec::ControlSize;
use fluent_core::spec::selection::{self, CheckboxProps, RadioProps};
use fluent_icons::FluentIcon;
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui_base::Checkbox as BaseCheckbox;
use gpui_base::{CheckboxState, Radio as BaseRadio};

/// Change handler shared by the selection controls.
pub type ChangeHandler = Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

/// A Fluent 2 checkbox.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    checked: bool,
    disabled: bool,
    size: ControlSize,
    label: Option<SharedString>,
    on_change: Option<ChangeHandler>,
    style: StyleRefinement,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            size: ControlSize::Medium,
            label: None,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
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

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Reports the next checked value.
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = selection::checkbox(
            &theme,
            &CheckboxProps {
                size: self.size,
                checked: self.checked,
                disabled: self.disabled,
            },
        );
        let checked = self.checked;
        let mark = spec.mark;

        let indicator = div()
            .flex_shrink_0()
            .size(spec.box_size)
            .rounded(spec.radius)
            .border_1()
            .border_color(spec.border)
            .bg(spec.background)
            .flex()
            .items_center()
            .justify_center()
            .when(checked, |el| {
                el.child(Icon::new(FluentIcon::Checkmark).size(px(12.)).color(mark))
            });

        // Fluent lays the indicator and the label out in a row with an 8px gap.
        let content = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(indicator)
            .when_some(self.label, |el, label| el.child(label));

        let mut el = BaseCheckbox::new(self.id)
            .checked(checked)
            .disabled(self.disabled)
            .child(content);

        if let Some(handler) = self.on_change {
            el = el.on_change(move |state, _, window, cx| {
                let checked = state == CheckboxState::Checked;
                handler(&checked, window, cx)
            });
        }
        el.refine_style(&self.style)
    }
}

/// A Fluent 2 radio button.
#[derive(IntoElement)]
pub struct Radio {
    id: ElementId,
    checked: bool,
    disabled: bool,
    label: Option<SharedString>,
    on_change: Option<ChangeHandler>,
    style: StyleRefinement,
}

impl Radio {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            label: None,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl Styled for Radio {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Radio {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = selection::radio(
            &theme,
            &RadioProps {
                checked: self.checked,
                disabled: self.disabled,
            },
        );
        let checked = self.checked;

        // Ring painted as an inset overlay: keeping the border out of layout
        // means flex centring puts the dot exactly in the middle.
        let indicator = div()
            .relative()
            .flex_shrink_0()
            .size(spec.circle)
            .rounded(px(10000.))
            .flex()
            .items_center()
            .justify_center()
            .when(checked, |el| {
                el.child(
                    div()
                        .flex_shrink_0()
                        .size(spec.dot)
                        .rounded(px(10000.))
                        .bg(spec.dot_color),
                )
            })
            .child(
                div()
                    .absolute()
                    .top(px(0.))
                    .left(px(0.))
                    .size(spec.circle)
                    .rounded(px(10000.))
                    .border_1()
                    .border_color(spec.border),
            );

        let content = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(indicator)
            .when_some(self.label, |el, label| el.child(label));

        let mut el = BaseRadio::new(self.id)
            .checked(checked)
            .disabled(self.disabled)
            .child(content);

        if let Some(handler) = self.on_change {
            el = el.on_change(move |value, _, window, cx| handler(&value, window, cx));
        }
        el.refine_style(&self.style)
    }
}
