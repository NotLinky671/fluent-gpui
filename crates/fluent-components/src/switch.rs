//! Fluent 2 `Switch`.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled,
    Window, div, px,
};
use fluent_core::spec::ControlSize;
use fluent_core::spec::switch::{self, SwitchProps};
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui_base::Switch as BaseSwitch;

/// Change handler shape shared by the selection controls.
pub type ChangeHandler = Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

/// A Fluent 2 switch.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    checked: bool,
    disabled: bool,
    size: ControlSize,
    label: Option<SharedString>,
    on_change: Option<ChangeHandler>,
    style: StyleRefinement,
}

impl Switch {
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

    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = switch::resolve(
            &theme,
            &SwitchProps {
                size: self.size,
                checked: self.checked,
                disabled: self.disabled,
            },
        );
        let checked = self.checked;

        let track = div()
            .flex_shrink_0()
            .w(spec.track_width)
            .h(spec.track_height)
            .rounded(px(10000.))
            .border_1()
            .border_color(spec.track_border)
            .bg(spec.track_bg)
            .flex()
            .items_center()
            .pl(if checked { spec.thumb_offset } else { px(3.) })
            .child(
                div()
                    .flex_shrink_0()
                    .size(spec.thumb_size)
                    .rounded(px(10000.))
                    .bg(spec.thumb),
            );

        let content = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(track)
            .when_some(self.label, |el, label| el.child(label));

        let mut el = BaseSwitch::new(self.id)
            .checked(checked)
            .disabled(self.disabled)
            .child(content);

        if let Some(handler) = self.on_change {
            el = el.on_change(move |value, _, window, cx| handler(&value, window, cx));
        }
        el.refine_style(&self.style)
    }
}
