//! Fluent 2 `Menu` rendering.
//!
//! Metrics and colours come from [`fluent_core::spec::menu`]; this file paints
//! them and owns the keyboard model.
//!
//! Keyboard navigation follows `aria-activedescendant`: the panel keeps focus and
//! tracks which row is active, rather than moving focus row by row. That is what
//! lets the popover's focus trap stay in charge of Tab while the arrow keys move
//! inside the menu.

use crate::icon::Icon;
use crate::popover::OpenChangeHandler;
use fluent_core::gpui_compat::Hsla;
use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
use fluent_core::spec::feedback;
use fluent_core::spec::menu::{self, MenuItemProps};
use fluent_icons::FluentIcon;
use fluent_theme::FluentTheme;
use gpui::Role;
use gpui::prelude::FluentBuilder as _;

/// What a menu item does when chosen, by pointer or by keyboard.
pub type MenuHandler = std::rc::Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// One row of a menu.
///
/// Typed entries are what make keyboard navigation possible: the panel has to
/// know which rows are selectable to move between them. Anything pushed through
/// `ParentElement::child` becomes [`Entry::Custom`] and is rendered but never
/// reachable by the arrow keys.
enum Entry {
    Item(Box<MenuItem>),
    Separator,
    Header(MenuHeader),
    Custom(AnyElement),
}

impl Entry {
    /// Index into the selectable list, or `None` when the row cannot be chosen.
    fn is_selectable(&self) -> bool {
        matches!(self, Entry::Item(item) if !item.disabled)
    }
}

/// A Fluent 2 menu panel.
///
/// The id is required because the active row has to survive redraws.
#[derive(IntoElement)]
pub struct Menu {
    id: ElementId,
    entries: Vec<Entry>,
    /// Focus lands here so the menu's own key context is on the dispatch path.
    /// The panel that hosts the menu is an *ancestor*, so a context set there
    /// would not include the menu's bindings.
    focus: Option<gpui::FocusHandle>,
    style: StyleRefinement,
}

impl Menu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            focus: None,
            style: StyleRefinement::default(),
        }
    }

    /// Adds a selectable row.
    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(Entry::Item(Box::new(item)));
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }

    pub fn header(mut self, header: MenuHeader) -> Self {
        self.entries.push(Entry::Header(header));
        self
    }

    /// Focus handle for keyboard navigation.
    ///
    /// Pass the same handle the hosting popover uses: the popover moves focus
    /// here on open, and the menu needs it on the dispatch path for the arrow
    /// keys to reach its handlers.
    pub fn track_focus(mut self, focus: &gpui::FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new("menu")
    }
}

impl Styled for Menu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Menu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.entries.extend(elements.into_iter().map(Entry::Custom));
    }
}

impl RenderOnce for Menu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let surface = menu::surface(&theme);
        let shadow = feedback::tooltip(&theme).shadow.to_vec();

        // Which rows the arrow keys can land on, in render order.
        let selectable: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.is_selectable())
            .map(|(index, _)| index)
            .collect();

        // Handlers are cloned out before the entries are consumed by rendering.
        let handlers: Vec<Option<MenuHandler>> = self
            .entries
            .iter()
            .map(|entry| match entry {
                Entry::Item(item) => item.on_select.clone(),
                _ => None,
            })
            .collect();

        let active_state = window.use_keyed_state(
            SharedString::from(format!("menu-active-{:?}", self.id)),
            cx,
            |_, _| None::<usize>,
        );
        let active = *active_state.read(cx);
        let count = selectable.len();

        // A row that is no longer selectable must not stay active; the count can
        // change when the caller swaps the menu contents.
        let active = active.filter(|index| *index < count);

        let mut container = div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(2.))
            .p(surface.padding)
            .min_w(surface.min_width)
            .rounded(surface.radius)
            .bg(surface.background)
            .text_color(surface.foreground)
            .shadow(shadow)
            .key_context(crate::MENU_KEY_CONTEXT)
            .role(Role::Menu)
            .when_some(self.focus.clone(), |el, focus| el.track_focus(&focus))
            .refine_style(&self.style);

        if count > 0 {
            let move_to = {
                let active_state = active_state.clone();
                move |cx: &mut App, next: Option<usize>| {
                    active_state.update(cx, |active, cx| {
                        if *active != next {
                            *active = next;
                            cx.notify();
                        }
                    });
                }
            };

            let next = move_to.clone();
            container = container.on_action(move |_: &crate::MenuNext, _, cx| {
                let target = match active {
                    None => Some(0),
                    Some(index) => Some((index + 1) % count),
                };
                next(cx, target);
            });
            let prev = move_to.clone();
            container = container.on_action(move |_: &crate::MenuPrev, _, cx| {
                let target = match active {
                    None | Some(0) => Some(count - 1),
                    Some(index) => Some(index - 1),
                };
                prev(cx, target);
            });
            let first = move_to.clone();
            container = container.on_action(move |_: &crate::MenuFirst, _, cx| first(cx, Some(0)));
            let last = move_to.clone();
            container =
                container.on_action(move |_: &crate::MenuLast, _, cx| last(cx, Some(count - 1)));

            let handlers = handlers.clone();
            let selectable = selectable.clone();
            container = container.on_action(move |_: &crate::MenuActivate, window, cx| {
                let Some(index) = active else {
                    return;
                };
                let Some(entry) = selectable.get(index) else {
                    return;
                };
                if let Some(Some(handler)) = handlers.get(*entry) {
                    handler(window, cx);
                }
            });
        }

        // Render rows, telling each item whether it is the active one.
        let mut selectable_index = 0;
        let children: Vec<AnyElement> = self
            .entries
            .into_iter()
            .map(|entry| match entry {
                Entry::Item(item) => {
                    let is_active = active == Some(selectable_index);
                    if !item.disabled {
                        selectable_index += 1;
                    }
                    (*item).active(is_active).into_any_element()
                }
                Entry::Separator => MenuSeparator.into_any_element(),
                Entry::Header(header) => header.into_any_element(),
                Entry::Custom(element) => element,
            })
            .collect();

        container.children(children)
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
    id: ElementId,
    trigger: Option<AnyElement>,
    menu: Menu,
    open: Option<bool>,
    default_open: bool,
    focus: Option<gpui::FocusHandle>,
    on_open_change: Option<OpenChangeHandler>,
}

impl MenuDropdown {
    pub fn new(id: impl Into<ElementId>, trigger: impl IntoElement, menu: Menu) -> Self {
        Self {
            id: id.into(),
            trigger: Some(trigger.into_any_element()),
            menu,
            open: None,
            default_open: false,
            focus: None,
            on_open_change: None,
        }
    }

    /// Controlled open state, forwarded to the popover.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Uncontrolled initial state.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Focus handle shared by the popover and the menu.
    ///
    /// Both need it: the popover moves focus into the panel on open and hands it
    /// back on close, and the menu needs it on the dispatch path for the arrow
    /// keys to reach its handlers.
    pub fn track_focus(mut self, focus: &gpui::FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(std::rc::Rc::new(handler));
        self
    }
}

impl RenderOnce for MenuDropdown {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut menu = self.menu;
        if let Some(focus) = self.focus.as_ref() {
            menu = menu.track_focus(focus);
        }

        let mut popover = crate::Popover::new(self.id)
            .default_open(self.default_open)
            .content(menu);
        if let Some(trigger) = self.trigger {
            popover = popover.trigger(trigger);
        }
        if let Some(open) = self.open {
            popover = popover.open(open);
        }
        if let Some(focus) = self.focus.as_ref() {
            popover = popover.track_focus(focus);
        }
        if let Some(handler) = self.on_open_change {
            popover = popover.on_open_change(move |next, window, cx| handler(next, window, cx));
        }

        popover.render(window, cx)
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
    /// Highlighted because the arrow keys are on this row.
    active: bool,
    on_select: Option<MenuHandler>,
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
            active: false,
            on_select: None,
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

    /// Runs when the row is chosen, by pointer or by the keyboard.
    ///
    /// The callback does not receive a click event: a menu row chosen with Enter
    /// has no pointer event to report, and callers act on "this row was chosen"
    /// rather than on the event that chose it.
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(std::rc::Rc::new(handler));
        self
    }

    /// Marks this row as the keyboard-active one. Set by [`Menu`].
    pub(crate) fn active(mut self, active: bool) -> Self {
        self.active = active;
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
        // The keyboard-active row uses the same fill as hover: Fluent has one
        // "this row is under the pointer or the cursor" look, not two.
        let background = if self.active && !self.disabled {
            spec.hover_background
        } else {
            spec.background
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
            .role(Role::MenuItem)
            .aria_selected(self.active && !self.disabled)
            .child(leading)
            .child(label)
            .refine_style(&self.style);

        // This GPUI version exposes no disabled state through accesskit, so a
        // disabled row is announced like any other and only refuses activation.
        // Recorded as a gap rather than worked around.

        if !self.disabled {
            el = el.cursor_pointer().hover(move |style| {
                style
                    .bg(spec.hover_background)
                    .text_color(spec.hover_foreground)
            });
            if let Some(handler) = self.on_select {
                el = el.on_click(move |_, window, cx| handler(window, cx));
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

#[cfg(test)]
mod keyboard_tests;
