//! Fluent 2 `Accordion`.
//!
//! Metrics and colours come from [`fluent_core::spec::accordion`]; this file lays
//! the rows out and owns the open set, the pointer and the keyboard model.
//!
//! The open set follows `useAccordion.js` exactly: one panel at a time unless
//! [`Accordion::multiple`], and the last open panel only closes when
//! [`Accordion::collapsible`]. Every header is its own tab stop, the way the web
//! component renders a `<button>` per header; arrow keys move focus without
//! toggling, Enter and Space toggle.
//!
//! Two upstream details are adapted, both recorded in
//! `docs/validation/accordion-2026-11-17.md`: the chevron is swapped between
//! `ChevronRight` and `ChevronDown` instead of being rotated, because this GPUI
//! snapshot has no element transform, and a panel appears without the 200ms
//! height transition, because there is no height animation either.

use std::rc::Rc;

use fluent_core::gpui_compat::{
    AnyElement, App, ElementId, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window,
    div, px,
};
use fluent_core::spec::accordion::{self, AccordionSize};
use fluent_icons::FluentIcon;
use fluent_theme::FluentTheme;
use gpui::{BoxShadow, KeyDownEvent, point};

use fluent_core::gpui_compat::StyledExt as _;

use crate::icon::Icon;

/// Called with the new open set after a header was activated.
pub type AccordionToggleHandler = Rc<dyn Fn(&AccordionToggle, &mut Window, &mut App) + 'static>;

/// Flips one item and reports the new set.
type Toggle = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// What a toggle produced: the activated item and the resulting open set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AccordionToggle {
    pub index: usize,
    pub open: Vec<usize>,
}

/// One header and its (optional) panel.
pub struct AccordionItem {
    id: ElementId,
    label: SharedString,
    icon: Option<FluentIcon>,
    content: Option<AnyElement>,
    disabled: bool,
}

impl AccordionItem {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            content: None,
            disabled: false,
        }
    }

    /// Leading icon, shown after the chevron and before the label.
    pub fn icon(mut self, icon: FluentIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The panel. An item without content still toggles, it just has nothing to
    /// show.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A stack of collapsible sections.
///
/// Uncontrolled by default: [`Accordion::default_open_items`] sets the starting
/// set and the component keeps it. Pass [`Accordion::open_items`] to own the set
/// instead, and update it from [`Accordion::on_toggle`].
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    items: Vec<AccordionItem>,
    size: AccordionSize,
    multiple: bool,
    collapsible: bool,
    open: Option<Vec<usize>>,
    default_open: Vec<usize>,
    on_toggle: Option<AccordionToggleHandler>,
    style: StyleRefinement,
}

impl Accordion {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            size: AccordionSize::Medium,
            multiple: false,
            collapsible: false,
            open: None,
            default_open: Vec::new(),
            on_toggle: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn size(mut self, size: AccordionSize) -> Self {
        self.size = size;
        self
    }

    /// Lets more than one panel stay open.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Lets the last open panel close. Without it a single accordion always keeps
    /// exactly one panel open, which is Fluent's default.
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// Controlled open set: the caller owns it.
    pub fn open_items(mut self, open: impl Into<Vec<usize>>) -> Self {
        self.open = Some(open.into());
        self
    }

    /// Uncontrolled starting set.
    pub fn default_open_items(mut self, open: impl Into<Vec<usize>>) -> Self {
        self.default_open = open.into();
        self
    }

    pub fn on_toggle(
        mut self,
        handler: impl Fn(&AccordionToggle, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    pub fn item(mut self, item: AccordionItem) -> Self {
        self.items.push(item);
        self
    }

    /// Caller styling, applied to the column that holds the rows.
    pub fn refine_style(mut self, style: StyleRefinement) -> Self {
        self.style = style;
        self
    }
}

/// The next enabled row in `step` direction, without wrapping: Fluent's arrow
/// navigation stops at the ends.
fn neighbour(disabled: &[bool], from: usize, step: isize) -> Option<usize> {
    let count = disabled.len() as isize;
    let mut index = from as isize;
    loop {
        index += step;
        if index < 0 || index >= count {
            return None;
        }
        if !disabled[index as usize] {
            return Some(index as usize);
        }
    }
}

fn edge(disabled: &[bool], last: bool) -> Option<usize> {
    if last {
        disabled.iter().rposition(|item| !item)
    } else {
        disabled.iter().position(|item| !item)
    }
}

impl RenderOnce for Accordion {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let spec = accordion::spec(&theme, self.size);
        let element_id = self.id.clone();

        let open_state = window.use_keyed_state(
            SharedString::from(format!("accordion-open-{element_id:?}")),
            cx,
            |_, _| accordion::normalize_default(&self.default_open, self.multiple),
        );
        let controlled = self.open.is_some();
        let open = match self.open.as_ref() {
            Some(items) => accordion::normalize_default(items, self.multiple),
            None => open_state.read(cx).clone(),
        };

        // One focus stop per header, created before the rows so a key handler on
        // any row can move focus to another one.
        let handles: Vec<FocusHandle> = (0..self.items.len())
            .map(|index| {
                window
                    .use_keyed_state(
                        SharedString::from(format!("accordion-focus-{element_id:?}-{index}")),
                        cx,
                        |_, cx| cx.focus_handle().tab_stop(true),
                    )
                    .read(cx)
                    .clone()
            })
            .collect();
        let disabled: Vec<bool> = self.items.iter().map(|item| item.disabled).collect();

        let toggle: Toggle = {
            let open_state = open_state.clone();
            let on_toggle = self.on_toggle.clone();
            let multiple = self.multiple;
            let collapsible = self.collapsible;
            let baseline = open.clone();
            Rc::new(move |index: usize, window: &mut Window, cx: &mut App| {
                let current = if controlled {
                    baseline.clone()
                } else {
                    open_state.read(cx).clone()
                };
                let next = accordion::toggled(&current, index, multiple, collapsible);
                if !controlled && next != current {
                    open_state.update(cx, |slot, cx| {
                        if *slot != next {
                            *slot = next.clone();
                            cx.notify();
                        }
                    });
                }
                if let Some(handler) = on_toggle.as_ref() {
                    handler(&AccordionToggle { index, open: next }, window, cx);
                }
            })
        };

        let mut container = div()
            .id(self.id)
            .flex()
            .flex_col()
            .refine_style(&self.style);

        for (index, item) in self.items.into_iter().enumerate() {
            let is_open = accordion::is_open(&open, index);
            let colors = accordion::header_colors(&spec, false, false, item.disabled);
            let hover = spec.hover.background;
            let pressed = spec.pressed.background;
            let focus_handle = handles[index].clone();
            let handles_for_keys = handles.clone();
            let disabled_for_keys = disabled.clone();

            let chevron = if is_open {
                FluentIcon::ChevronDown
            } else {
                FluentIcon::ChevronRight
            };

            let mut header = div()
                .id(item.id.clone())
                .relative()
                .flex()
                .items_center()
                .gap(spec.header.gap)
                .h(spec.header.height)
                .pl(spec.header.padding_start)
                .pr(spec.header.padding_end)
                .rounded(spec.header.radius)
                .text_size(spec.header.font_size)
                .line_height(spec.header.line_height)
                .text_color(colors.foreground)
                .bg(colors.background)
                .role(Role::Button)
                .aria_expanded(is_open)
                .child(
                    Icon::new(chevron)
                        .size(spec.header.chevron)
                        .color(colors.foreground),
                );

            if let Some(icon) = item.icon {
                header = header.child(Icon::new(icon).size(spec.header.chevron));
            }

            header = header.child(div().flex_1().child(item.label.clone()));

            if !item.disabled {
                let click_toggle = toggle.clone();
                let key_toggle = toggle.clone();
                header = header
                    .tab_stop(true)
                    .track_focus(&focus_handle)
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .active(move |style| style.bg(pressed))
                    .on_click(move |_, window, cx| click_toggle(index, window, cx))
                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                        let key = event.keystroke.key.as_str();
                        match key {
                            "enter" | "space" => {
                                cx.stop_propagation();
                                key_toggle(index, window, cx);
                            }
                            "down" | "up" | "home" | "end" => {
                                let target = match key {
                                    "down" => neighbour(&disabled_for_keys, index, 1),
                                    "up" => neighbour(&disabled_for_keys, index, -1),
                                    "home" => edge(&disabled_for_keys, false),
                                    _ => edge(&disabled_for_keys, true),
                                };
                                cx.stop_propagation();
                                if let Some(target) = target {
                                    handles_for_keys[target].focus(window, cx);
                                }
                            }
                            _ => {}
                        }
                    });
            }

            let ring_colour = spec.focus_ring;
            let ring_thickness = spec.header.focus_thickness;
            header = header.focus_visible(move |style| {
                style
                    .border_1()
                    .border_color(ring_colour)
                    .shadow(vec![BoxShadow {
                        color: ring_colour,
                        offset: point(px(0.), px(0.)),
                        blur_radius: px(0.),
                        spread_radius: ring_thickness,
                        inset: true,
                    }])
            });

            container = container.child(header);

            if is_open && let Some(content) = item.content {
                container = container.child(
                    div()
                        .id(ElementId::Name(
                            format!("{element_id:?}-panel-{index}").into(),
                        ))
                        .ml(spec.panel.margin_start)
                        .mr(spec.panel.margin_end)
                        .p(spec.panel.padding)
                        .role(Role::Group)
                        .child(content),
                );
            }
        }

        container
    }
}

#[cfg(test)]
mod tests;
