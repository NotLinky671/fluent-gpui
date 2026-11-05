//! Fluent 2 `Tabs`.
//!
//! Metrics and colours come from [`fluent_core::spec::tabs`]; this file lays the
//! strip out and owns the keyboard model.
//!
//! The strip is a single focus stop that tracks the selected index rather than
//! giving every tab its own stop. Arrow keys move the selection (Fluent's
//! automatic activation) and the selected tab paints the focus ring while the
//! strip holds focus.
//!
//! The indicator is positioned from the tabs' painted bounds, recorded during the
//! previous frame's prepaint and fed through the same transition helper the other
//! controls use, so changing the selection slides it instead of jumping.

use fluent_core::gpui_compat::StyledExt as _;
use fluent_core::gpui_compat::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window,
    div, px,
};
use fluent_core::spec::tabs::{self, TabsOrientation};
use fluent_icons::FluentIcon;
use fluent_theme::FluentTheme;
use gpui::Bounds;
use gpui::prelude::FluentBuilder as _;

use crate::icon::Icon;

/// Selects a tab and tells the caller, from inside an event handler.
type SelectFn = std::rc::Rc<dyn Fn(&mut Window, &mut App, usize)>;

/// Called with the newly selected index.
pub type TabChangeHandler = std::rc::Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>;

/// One tab.
#[derive(Clone)]
pub struct TabItem {
    id: ElementId,
    label: SharedString,
    icon: Option<FluentIcon>,
    disabled: bool,
}

impl TabItem {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            disabled: false,
        }
    }

    pub fn icon(mut self, icon: FluentIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Painted geometry of the strip, captured for the following frame.
#[derive(Default)]
struct Geometry {
    row: Option<Bounds<Pixels>>,
    tabs: Vec<Bounds<Pixels>>,
}

/// A horizontal or vertical tab strip.
///
/// Controlled by default: pass [`TabList::selected`] and update it from
/// [`TabList::on_change`]. [`TabList::default_selected`] gives an uncontrolled
/// strip that keeps its own index.
#[derive(IntoElement)]
pub struct TabList {
    id: ElementId,
    orientation: TabsOrientation,
    items: Vec<TabItem>,
    controlled: bool,
    selected: usize,
    default_selected: usize,
    on_change: Option<TabChangeHandler>,
    focus: Option<FocusHandle>,
    style: StyleRefinement,
}

impl TabList {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            orientation: TabsOrientation::Horizontal,
            items: Vec::new(),
            controlled: false,
            selected: 0,
            default_selected: 0,
            on_change: None,
            focus: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn vertical(mut self) -> Self {
        self.orientation = TabsOrientation::Vertical;
        self
    }

    pub fn orientation(mut self, orientation: TabsOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn tab(mut self, tab: TabItem) -> Self {
        self.items.push(tab);
        self
    }

    /// Controlled selection: the caller owns the index.
    pub fn selected(mut self, index: usize) -> Self {
        self.controlled = true;
        self.selected = index;
        self
    }

    /// Uncontrolled initial selection.
    pub fn default_selected(mut self, index: usize) -> Self {
        self.default_selected = index;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(std::rc::Rc::new(handler));
        self
    }

    /// Focus handle for keyboard navigation.
    ///
    /// The strip holds focus and the selected tab paints the ring.
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
}

impl ParentElement for TabList {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        // Only typed tabs take part in navigation, so anything pushed through the
        // parent API is dropped rather than rendered unreachable.
        drop(elements);
    }
}

/// The next selectable index after `from`, wrapping around once.
fn next_selectable(items: &[TabItem], from: usize, forward: bool) -> Option<usize> {
    let count = items.len();
    if count == 0 {
        return None;
    }
    for step in 1..=count {
        let index = if forward {
            (from + step) % count
        } else {
            (from + count - (step % count)) % count
        };
        if !items[index].disabled {
            return Some(index);
        }
    }
    None
}

fn edge_selectable(items: &[TabItem], last: bool) -> Option<usize> {
    if last {
        items.iter().rposition(|tab| !tab.disabled)
    } else {
        items.iter().position(|tab| !tab.disabled)
    }
}

impl RenderOnce for TabList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        // Consumed by the container element below; the keyed state and the
        // indicator ids still need it.
        let element_id = self.id.clone();
        let spec = tabs::list(&theme, self.orientation);
        let m = spec.metrics;
        let horizontal = self.orientation.is_horizontal();

        let selected_state = window.use_keyed_state(
            SharedString::from(format!("tabs-selected-{:?}", element_id)),
            cx,
            |_, _| self.default_selected,
        );
        let stored = *selected_state.read(cx);
        let selected = if self.controlled {
            self.selected
        } else {
            stored
        };
        // Two different fallbacks, on purpose:
        // - an index past the end is a caller mistake, and clamping to the last
        //   usable tab keeps the selection adjacent instead of jumping to the top;
        // - an index that lands on a disabled tab falls back to the first usable
        //   tab, because a disabled tab can never be the selection.
        let selected = match self.items.get(selected) {
            Some(tab) if !tab.disabled => selected,
            Some(_) => edge_selectable(&self.items, false).unwrap_or(0),
            None => edge_selectable(&self.items, true).unwrap_or(0),
        };

        let geometry = window.use_keyed_state(
            SharedString::from(format!("tabs-geometry-{:?}", element_id)),
            cx,
            |_, _| std::rc::Rc::new(std::cell::RefCell::new(Geometry::default())),
        );
        let painted = geometry.read(cx).clone();
        let painted_row = painted.borrow().row;
        let painted_tab = painted.borrow().tabs.get(selected).copied();

        let select: SelectFn = {
            let selected_state = selected_state.clone();
            let on_change = self.on_change.clone();
            let controlled = self.controlled;
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App, index: usize| {
                if !controlled {
                    selected_state.update(cx, |current, cx| {
                        if *current != index {
                            *current = index;
                            cx.notify();
                        }
                    });
                }
                if let Some(handler) = on_change.as_ref() {
                    handler(&index, window, cx);
                }
            })
        };

        let mut container = div()
            .id(self.id)
            .flex()
            .when(horizontal, |el| el.flex_row())
            .when(!horizontal, |el| el.flex_col())
            .gap(m.tab_gap)
            .when(!horizontal, |el| el.w(m.vertical_width))
            .key_context(crate::TABS_KEY_CONTEXT)
            .role(Role::TabList)
            .when_some(self.focus.clone(), |el, focus| el.track_focus(&focus))
            .refine_style(&self.style);

        if !self.items.is_empty() {
            let items = self.items.clone();
            let next = {
                let items = items.clone();
                let select = select.clone();
                move |window: &mut Window, cx: &mut App| {
                    if let Some(index) = next_selectable(&items, selected, true) {
                        select(window, cx, index);
                    }
                }
            };
            let previous = {
                let items = items.clone();
                let select = select.clone();
                move |window: &mut Window, cx: &mut App| {
                    if let Some(index) = next_selectable(&items, selected, false) {
                        select(window, cx, index);
                    }
                }
            };
            let first = {
                let items = items.clone();
                let select = select.clone();
                move |window: &mut Window, cx: &mut App| {
                    if let Some(index) = edge_selectable(&items, false) {
                        select(window, cx, index);
                    }
                }
            };
            let last = {
                let items = items.clone();
                let select = select.clone();
                move |window: &mut Window, cx: &mut App| {
                    if let Some(index) = edge_selectable(&items, true) {
                        select(window, cx, index);
                    }
                }
            };
            container = container
                .on_action(move |_: &crate::TabsNext, window, cx| next(window, cx))
                .on_action(move |_: &crate::TabsPrev, window, cx| previous(window, cx))
                .on_action(move |_: &crate::TabsFirst, window, cx| first(window, cx))
                .on_action(move |_: &crate::TabsLast, window, cx| last(window, cx));
        }

        let focused = self
            .focus
            .as_ref()
            .map(|focus| focus.is_focused(window))
            .unwrap_or(false);
        let ring_color = theme.tokens().color.neutral_foreground_1;

        let mut row = div()
            .relative()
            .flex()
            .when(horizontal, |el| el.flex_row())
            .when(!horizontal, |el| el.flex_col())
            .gap(m.tab_gap);

        for (index, item) in self.items.iter().enumerate() {
            let colors = tabs::tab_colors(&spec, index == selected, false, false, item.disabled);
            let is_selected = index == selected;
            let hover = spec.hover;

            let mut tab = div()
                .id(item.id.clone())
                .flex()
                .items_center()
                // Horizontal tabs centre their label; a vertical strip reads as a
                // list, so its labels start at the leading edge.
                .when(horizontal, |el| el.justify_center())
                .when(!horizontal, |el| el.justify_start().w_full())
                .gap(m.gap)
                .h(m.height)
                .px(m.padding_x)
                .rounded(m.radius)
                .text_size(m.font_size)
                .line_height(m.line_height)
                .text_color(colors.foreground)
                .bg(colors.background)
                .role(Role::Tab)
                .aria_selected(is_selected)
                .when(is_selected && focused, |el| {
                    el.shadow(vec![gpui::BoxShadow {
                        color: ring_color,
                        offset: gpui::point(px(0.), px(0.)),
                        blur_radius: px(0.),
                        spread_radius: px(1.),
                        inset: true,
                    }])
                })
                .when(!item.disabled, |el| {
                    let select = select.clone();
                    el.cursor_pointer()
                        .hover(move |style| style.bg(hover.background).text_color(hover.foreground))
                        .on_click(move |_, window, cx| select(window, cx, index))
                });

            if let Some(icon) = item.icon {
                tab = tab.child(Icon::new(icon).size(px(16.)));
            }
            row = row.child(tab.child(item.label.clone()));
        }

        // Indicator, positioned from the previous frame's recorded bounds.
        if let (Some(row_bounds), Some(tab_bounds)) = (painted_row, painted_tab) {
            let thickness = m.indicator_thickness;
            let mut indicator = div().absolute().rounded(thickness / 2.).bg(spec.indicator);
            if horizontal {
                let target = tab_bounds.origin.x - row_bounds.origin.x;
                let left = crate::motion::length(
                    SharedString::from(format!("tabs-indicator-x-{:?}", element_id)),
                    target,
                    fluent_core::spec::motion::motion_policy(),
                    window,
                    cx,
                );
                // The width has to animate with the left edge. Snapping it to the
                // target tab while the left edge still slides makes the trailing
                // edge jump for a frame — visible when a wide tab hands over to a
                // narrower one.
                let width = crate::motion::length(
                    SharedString::from(format!("tabs-indicator-w-{:?}", element_id)),
                    tab_bounds.size.width,
                    fluent_core::spec::motion::motion_policy(),
                    window,
                    cx,
                );
                indicator = indicator
                    .left(left)
                    .top(tab_bounds.size.height - thickness)
                    .w(width)
                    .h(thickness);
                indicator = indicator.top(tab_bounds.size.height - thickness);
            } else {
                let target = tab_bounds.origin.y - row_bounds.origin.y;
                let top = crate::motion::length(
                    SharedString::from(format!("tabs-indicator-y-{:?}", element_id)),
                    target,
                    fluent_core::spec::motion::motion_policy(),
                    window,
                    cx,
                );
                let height = crate::motion::length(
                    SharedString::from(format!("tabs-indicator-h-{:?}", element_id)),
                    tab_bounds.size.height,
                    fluent_core::spec::motion::motion_policy(),
                    window,
                    cx,
                );
                indicator = indicator.left(px(0.)).top(top).h(height).w(thickness);
            }
            row = row.child(indicator);
        }

        let tabs_len = self.items.len();
        let row_out = painted.clone();
        let row_recorder = row.on_children_prepainted(move |children, _, _| {
            let tabs: Vec<Bounds<Pixels>> = children.iter().take(tabs_len).copied().collect();
            let mut state = row_out.borrow_mut();
            state.tabs = tabs;
        });

        let outer_out = painted.clone();
        container.child(
            div()
                .on_children_prepainted(move |children, _, _| {
                    if let Some(bounds) = children.first() {
                        outer_out.borrow_mut().row = Some(*bounds);
                    }
                })
                .child(row_recorder),
        )
    }
}

#[cfg(test)]
mod tests;
