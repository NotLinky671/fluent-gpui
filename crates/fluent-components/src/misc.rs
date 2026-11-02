//! Fluent 2 `Divider` and `Link`.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce,
    Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
use fluent_core::spec::misc::{self, DividerAppearance, LinkAppearance};
use fluent_theme::FluentTheme;

/// Click handler shape shared by the lightweight controls.
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// A Fluent 2 divider.
#[derive(IntoElement)]
pub struct Divider {
    appearance: DividerAppearance,
    vertical: bool,
    inset: Option<Pixels>,
    style: StyleRefinement,
}

impl Divider {
    pub fn new() -> Self {
        Self {
            appearance: DividerAppearance::Default,
            vertical: false,
            inset: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn appearance(mut self, appearance: DividerAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// A vertical divider that stretches across its container.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Insets the line from both ends.
    pub fn inset(mut self, inset: impl Into<Pixels>) -> Self {
        self.inset = Some(inset.into());
        self
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Divider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Divider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = misc::divider(&theme, self.appearance);

        let mut el = div();
        if self.vertical {
            el = el.w(spec.thickness).min_h(px(20.)).bg(spec.color);
        } else {
            el = el.h(spec.thickness).w_full().bg(spec.color);
        }
        if let Some(inset) = self.inset {
            el = if self.vertical {
                el.my(inset)
            } else {
                el.mx(inset)
            };
        }
        el.refine_style(&self.style)
    }
}

/// A Fluent 2 link.
#[derive(IntoElement)]
pub struct Link {
    id: ElementId,
    text: SharedString,
    appearance: LinkAppearance,
    disabled: bool,
    on_click: Option<ClickHandler>,
    style: StyleRefinement,
}

impl Link {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            appearance: LinkAppearance::Default,
            disabled: false,
            on_click: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn appearance(mut self, appearance: LinkAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Styled for Link {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Link {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = misc::link(&theme, self.appearance, self.disabled);

        let mut el = div()
            .id(self.id)
            .role(Role::Link)
            .text_size(spec.font_size)
            .line_height(spec.line_height)
            .text_color(spec.color)
            .child(self.text)
            .refine_style(&self.style);

        if !self.disabled {
            el = el
                .cursor_pointer()
                .hover(move |s| s.text_color(spec.hover_color))
                .active(move |s| s.text_color(spec.pressed_color));
        }
        if let Some(handler) = self.on_click {
            el = el.on_click(handler);
        }
        el
    }
}
