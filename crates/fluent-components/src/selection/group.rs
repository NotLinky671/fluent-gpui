//! Group-owned selection and roving focus for Fluent radio buttons.
use super::*;
use gpui::{Axis, FocusHandle, StatefulInteractiveElement as _};
use std::rc::Rc;

type GroupChange = Rc<dyn Fn(&ElementId, &mut Window, &mut App)>;

/// A named set of mutually exclusive radios, keyed by each [Radio]'s id.
///
/// Use `value(Some(id.into()))` for controlled selection (`None` clears it),
/// or `default_value(id)` for an initial uncontrolled selection. Child checked
/// builders are ignored; the group owns selection. Arrow keys wrap over enabled
/// items and move both selection and focus. Tab visits the selected enabled item,
/// or the first enabled item when nothing selectable is selected.
#[derive(IntoElement)]
pub struct RadioGroup {
    id: ElementId,
    value: Option<Option<ElementId>>,
    default_value: Option<ElementId>,
    items: Vec<Radio>,
    label: SharedString,
    axis: Axis,
    size: Option<ControlSize>,
    disabled: bool,
    on_change: Option<GroupChange>,
    style: StyleRefinement,
}

impl RadioGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: None,
            default_value: None,
            items: Vec::new(),
            label: "Options".into(),
            axis: Axis::Vertical,
            size: None,
            disabled: false,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }
    pub fn value(mut self, value: Option<ElementId>) -> Self {
        self.value = Some(value);
        self
    }
    pub fn default_value(mut self, value: impl Into<ElementId>) -> Self {
        self.default_value = Some(value.into());
        self
    }
    pub fn radio(mut self, radio: Radio) -> Self {
        self.items.push(radio);
        self
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = Some(size);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn on_change(
        mut self,
        handler: impl Fn(&ElementId, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for RadioGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for RadioGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let stored = window.use_keyed_state(
            SharedString::from(format!("radio-group-value-{:?}", self.id)),
            cx,
            |_, _| self.default_value,
        );
        let controlled = self.value.is_some();
        let selected = self.value.unwrap_or_else(|| stored.read(cx).clone());
        let select: GroupChange = Rc::new(move |id, window, cx| {
            if !controlled {
                stored.update(cx, |value, cx| {
                    *value = Some(id.clone());
                    cx.notify();
                });
            }
            if let Some(handler) = &self.on_change {
                handler(id, window, cx);
            }
        });
        let focus: Vec<FocusHandle> = self
            .items
            .iter()
            .map(|item| {
                window
                    .use_keyed_state(
                        SharedString::from(format!(
                            "radio-group-focus-{:?}-{:?}",
                            self.id, item.id
                        )),
                        cx,
                        |_, cx| cx.focus_handle(),
                    )
                    .read(cx)
                    .clone()
            })
            .collect();
        let enabled: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| (!self.disabled && !item.disabled).then_some(i))
            .collect();
        let selected_index = self
            .items
            .iter()
            .position(|item| Some(&item.id) == selected.as_ref());
        let tab_index = selected_index
            .filter(|i| enabled.contains(i))
            .or_else(|| enabled.first().copied());
        let ids: Vec<_> = self.items.iter().map(|item| item.id.clone()).collect();
        let horizontal = self.axis == Axis::Horizontal;
        let mut group = gpui_base::RadioGroup::new(self.id)
            .axis(self.axis)
            .aria_label(self.label)
            .flex()
            .when(!horizontal, |el| el.flex_col())
            .gap(FluentTheme::global(cx).tokens().spacing.vertical.s)
            .refine_style(&self.style);
        let navigation_focus = focus.clone();
        let change = select.clone();
        let current_selected = selected.clone();
        group = group.on_key_down(
            move |event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App| {
                if event.keystroke.modifiers.control
                    || event.keystroke.modifiers.alt
                    || event.keystroke.modifiers.platform
                {
                    return;
                }
                let key = event.keystroke.key.as_str();
                let forward = match key {
                    "right" if horizontal => Some(true),
                    "left" if horizontal => Some(false),
                    "down" if !horizontal => Some(true),
                    "up" if !horizontal => Some(false),
                    _ => None,
                };
                let current = navigation_focus.iter().position(|f| f.is_focused(window));
                let next = if let Some(forward) = forward {
                    let current_position =
                        current.and_then(|i| enabled.iter().position(|v| *v == i));
                    match (current_position, forward, enabled.len()) {
                        (_, _, 0) => None,
                        (Some(i), true, n) => Some(enabled[(i + 1) % n]),
                        (Some(i), false, n) => Some(enabled[(i + n - 1) % n]),
                        (None, true, _) => enabled.first().copied(),
                        (None, false, _) => enabled.last().copied(),
                    }
                } else {
                    match key {
                        "home" => enabled.first().copied(),
                        "end" => enabled.last().copied(),
                        _ => return,
                    }
                };
                if let Some(i) = next {
                    navigation_focus[i].focus(window, cx);
                    if current_selected.as_ref() != Some(&ids[i]) {
                        change(&ids[i], window, cx);
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                }
            },
        );
        let count = self.items.len();
        for (i, mut item) in self.items.into_iter().enumerate() {
            item.checked = Some(&item.id) == selected.as_ref();
            item.controlled = true;
            item.disabled |= self.disabled;
            item.group_focus = Some(focus[i].clone());
            item.tab_stop = tab_index == Some(i);
            item.position_in_set = Some((i + 1, count));
            if let Some(size) = self.size {
                item.size = size;
            }
            let child_handler = item.on_change.take();
            let change = select.clone();
            let id = item.id.clone();
            item.on_change = Some(Box::new(move |value, window, cx| {
                if *value {
                    change(&id, window, cx);
                }
                if let Some(handler) = &child_handler {
                    handler(value, window, cx);
                }
            }));
            group = group.child(item);
        }
        group
    }
}
