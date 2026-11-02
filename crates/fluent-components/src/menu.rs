//! Fluent 2 `Menu` rendering.
//!
//! Metrics and colours come from [`fluent_core::spec::menu`]; this file only
//! paints them. Trigger wiring and keyboard navigation live in later tasks.

use crate::icon::Icon;
use fluent_core::gpui_compat::Hsla;
use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
use fluent_core::spec::feedback;
use fluent_core::spec::menu::{self, MenuItemProps};
use fluent_icons::FluentIcon;
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;

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

/// A [`Menu`] that opens from a trigger.
///
/// Wraps the panel in a [`Popover`](crate::Popover) so the menu gets anchored
/// placement, outside-click and Escape dismissal. This is composition, not a
/// new interaction model: the popover owns open state and dismissal, the menu
/// owns the rows.
///
/// Keyboard navigation *inside* the menu (arrow keys, Home/End, submenus) is not
/// implemented yet, so the panel is reachable by pointer only.
#[derive(IntoElement)]
pub struct MenuDropdown {
    popover: crate::Popover,
}

impl MenuDropdown {
    pub fn new(id: impl Into<ElementId>, trigger: impl IntoElement, menu: Menu) -> Self {
        Self {
            popover: crate::Popover::new(id)
                .trigger(trigger)
                .content(menu)
                .default_open(false),
        }
    }

    /// Controlled open state, forwarded to the popover.
    pub fn open(mut self, open: bool) -> Self {
        self.popover = self.popover.open(open);
        self
    }

    /// Uncontrolled initial state.
    pub fn default_open(mut self, open: bool) -> Self {
        self.popover = self.popover.default_open(open);
        self
    }

    pub fn track_focus(mut self, focus: &gpui::FocusHandle) -> Self {
        self.popover = self.popover.track_focus(focus);
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.popover = self.popover.on_open_change(handler);
        self
    }
}

impl RenderOnce for MenuDropdown {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.popover.render(window, cx)
    }
}

/// One actionable row.
#[derive(IntoElement)]
pub struct MenuItem {
    id: ElementId,
    label: SharedString,
    icon: Option<FluentIcon>,
    /// `Some(checked)` renders a check mark in the leading slot.
    check: Option<bool>,
    /// `Some(selected)` renders a radio dot in the leading slot.
    radio: Option<bool>,
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
            icon: None,
            check: None,
            radio: None,
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

    /// Leading icon, shown in the same slot a check mark would occupy.
    pub fn icon(mut self, icon: FluentIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Renders as a checkable row: a check mark appears while `checked`.
    pub fn checked(mut self, checked: bool) -> Self {
        self.check = Some(checked);
        self
    }

    /// Renders as a radio row: a dot appears while `selected`.
    pub fn selected(mut self, selected: bool) -> Self {
        self.radio = Some(selected);
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

impl MenuItem {
    /// The fixed-width slot before the label: icon, check mark, radio dot, or
    /// nothing.
    ///
    /// It keeps its width when empty so labels stay aligned down the menu.
    fn leading_slot(&self, spec: &menu::MenuItemSpec, foreground: Hsla) -> AnyElement {
        let slot = div()
            .flex()
            .items_center()
            .justify_center()
            .w(spec.indicator_slot)
            .h(spec.icon_size)
            .flex_shrink_0();

        if let Some(checked) = self.check {
            return slot
                .when(checked, |el| {
                    el.child(
                        Icon::new(FluentIcon::Checkmark)
                            .size(px(14.))
                            .color(foreground),
                    )
                })
                .into_any_element();
        }

        if let Some(selected) = self.radio {
            return slot
                .when(selected, |el| {
                    // A dot rather than a ring: the ring is the row's own
                    // affordance in Fluent's radio menu items.
                    el.child(div().w(px(8.)).h(px(8.)).rounded_full().bg(foreground))
                })
                .into_any_element();
        }

        if let Some(icon) = self.icon {
            return slot
                .child(Icon::new(icon).size(px(16.)).color(foreground))
                .into_any_element();
        }

        slot.into_any_element()
    }
}

impl RenderOnce for MenuItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = menu::item(
            &theme,
            &MenuItemProps {
                danger: self.danger,
                disabled: self.disabled,
            },
        );

        // Hover is applied as a style rather than through tracked state: with
        // tracked state the highlight flickered (the state was observed
        // resetting while the pointer was still over the row).
        let foreground = if self.disabled {
            spec.disabled_foreground
        } else {
            spec.foreground
        };

        let label = self.label.clone();
        let leading = self.leading_slot(&spec, foreground);
        let mut el = div()
            .id(self.id)
            .flex()
            .items_center()
            .h(spec.height)
            .px(spec.padding_x)
            .gap(spec.gap)
            .rounded(spec.radius)
            .bg(spec.background)
            .text_color(foreground)
            .text_size(spec.font_size)
            .line_height(spec.line_height)
            .child(leading)
            .child(label)
            .refine_style(&self.style);

        if !self.disabled {
            el = el.cursor_pointer().hover(move |style| {
                style
                    .bg(spec.hover_background)
                    .text_color(spec.hover_foreground)
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
