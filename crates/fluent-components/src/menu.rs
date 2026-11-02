//! Fluent 2 `Menu` rendering.
//!
//! Metrics and colours come from [`fluent_core::spec::menu`]; this file only
//! paints them. Trigger wiring and keyboard navigation live in later tasks.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
use fluent_core::spec::feedback;
use fluent_core::spec::menu::{self, MenuItemProps};
use fluent_theme::FluentTheme;

/// Click handler shape for menu items.
pub type MenuHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// A Fluent 2 menu panel. Place [`MenuItem`]s inside it.
#[derive(IntoElement)]
pub struct Menu {
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Menu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Menu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Menu {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let surface = menu::surface(&theme);
        let shadow = feedback::tooltip(&theme).shadow.to_vec();
        div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .p(surface.padding)
            .min_w(surface.min_width)
            .rounded(surface.radius)
            .bg(surface.background)
            .text_color(surface.foreground)
            .shadow(shadow)
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// One actionable row.
#[derive(IntoElement)]
pub struct MenuItem {
    id: ElementId,
    label: SharedString,
    disabled: bool,
    danger: bool,
    on_click: Option<MenuHandler>,
    style: StyleRefinement,
}

impl MenuItem {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            disabled: false,
            danger: false,
            on_click: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Destructive action: red label and red hover.
    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
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

impl Styled for MenuItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MenuItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = menu::item(
            &theme,
            &MenuItemProps {
                danger: self.danger,
                disabled: self.disabled,
            },
        );

        let hover_state = window.use_keyed_state(
            SharedString::from(format!("menu-item-hovered-{:?}", self.id)),
            cx,
            |_, _| false,
        );
        let hovered = *hover_state.read(cx) && !self.disabled;

        let background = crate::motion::state_color(
            SharedString::from(format!("menu-item-bg-{:?}", self.id)),
            if hovered {
                spec.hover_background
            } else {
                spec.background
            },
            window,
            cx,
        );
        let foreground = if self.disabled {
            spec.disabled_foreground
        } else {
            crate::motion::state_color(
                SharedString::from(format!("menu-item-fg-{:?}", self.id)),
                if hovered {
                    spec.hover_foreground
                } else {
                    spec.foreground
                },
                window,
                cx,
            )
        };

        let mut el = div()
            .id(self.id)
            .flex()
            .items_center()
            .h(spec.height)
            .px(spec.padding_x)
            .gap(spec.gap)
            .rounded(spec.radius)
            .bg(background)
            .text_color(foreground)
            .text_size(spec.font_size)
            .line_height(spec.line_height)
            .child(self.label)
            .refine_style(&self.style);

        if !self.disabled {
            let hover_listener = hover_state.clone();
            el = el.cursor_pointer().on_hover(move |is_hovered, _, cx| {
                hover_listener.update(cx, |state, cx| {
                    if *state != *is_hovered {
                        *state = *is_hovered;
                        cx.notify();
                    }
                });
            });
            if let Some(handler) = self.on_click {
                el = el.on_click(handler);
            }
        }

        el
    }
}

/// A hairline between groups.
#[derive(IntoElement)]
pub struct MenuSeparator;

impl RenderOnce for MenuSeparator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        div()
            .h(px(1.))
            .my(px(4.))
            .bg(theme.tokens().color.neutral_stroke_1)
    }
}

/// A small heading above a group of items.
#[derive(IntoElement)]
pub struct MenuHeader {
    text: SharedString,
}

impl MenuHeader {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl RenderOnce for MenuHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = menu::item(&theme, &MenuItemProps::default());
        div()
            .flex()
            .items_center()
            .h(spec.height)
            .px(spec.padding_x)
            .text_size(px(12.))
            .font_weight(fluent_core::gpui_compat::FontWeight::SEMIBOLD)
            .text_color(theme.tokens().color.neutral_foreground_3)
            .child(self.text)
    }
}
