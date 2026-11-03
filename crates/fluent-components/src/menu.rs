//! Fluent 2 `Menu` rendering.
//!
//! Metrics and colours come from [`fluent_core::spec::menu`]; this file paints
//! them and owns the keyboard model.
//!
//! Keyboard navigation keeps focus on the menu container and tracks which row is
//! current, rather than moving focus row by row. That is what lets the popover's
//! focus trap stay in charge of Tab while the arrow keys move inside the menu.
//!
//! The current row is one piece of state written by both the keyboard and the
//! pointer, so only one row can look current and Enter always agrees with it. It
//! is stored by row identity rather than by position, so rows being inserted,
//! removed or disabled above it do not silently retarget activation.
//!
//! The state also records which input wrote it: a row the pointer selected stops
//! looking current when the pointer leaves the menu, while a row the keyboard
//! selected survives that, so drifting the mouse away does not undo a key press.
//!
//! This model does not use `aria-activedescendant`: focus itself is on the
//! container, and the current row is reported through `aria-selected`.

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

/// Reports a mouse-move event over a row, so the menu can make it current.
type PointerHandler = std::rc::Rc<dyn Fn(&mut App)>;

/// Typed-prefix state for jumping to a row by its label.
///
/// A prefix older than the reset window is discarded, and typing the same
/// character again restarts from that character instead of building "aa", so
/// pressing a letter repeatedly cycles through the rows starting with it.
#[derive(Clone, Default)]
struct Typeahead {
    prefix: String,
    at: Option<std::time::Instant>,
}

const TYPEAHEAD_RESET: std::time::Duration = std::time::Duration::from_millis(600);

/// The row the menu points at, and which input put it there.
///
/// The source decides what happens when the pointer leaves the menu: a row the
/// pointer selected stops looking current, while a row the keyboard selected has
/// to survive the pointer passing over and out.
#[derive(Clone, PartialEq)]
struct CurrentRow {
    id: ElementId,
    from_pointer: bool,
}

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
        // Captured before the id is moved into the container element: non-selectable
        // rows need ids of their own, unique within this menu.
        let menu_id = SharedString::from(format!("{:?}", self.id));

        // Rows the arrow keys can land on, in render order, paired with the entry
        // they came from. Identity is the row's ElementId, not this position: the
        // caller may insert, remove or disable rows above the current one, and a
        // saved position would silently start pointing at a different action.
        let selectable: Vec<(usize, ElementId)> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.is_selectable())
            .map(|(index, entry)| {
                let id = match entry {
                    Entry::Item(item) => item.id.clone(),
                    // `is_selectable` only returns true for items.
                    _ => unreachable!("only items are selectable"),
                };
                (index, id)
            })
            .collect();

        // Labels in the same order as `selectable`, for typed-prefix matching.
        let labels: Vec<SharedString> = selectable
            .iter()
            .map(|(entry, _)| match &self.entries[*entry] {
                Entry::Item(item) => item.label.clone(),
                _ => unreachable!("only items are selectable"),
            })
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

        // The current row is stored by identity, so it survives rows being
        // inserted, removed or disabled above it.
        let active_state = window.use_keyed_state(
            SharedString::from(format!("menu-active-{:?}", self.id)),
            cx,
            |_, _| None::<CurrentRow>,
        );
        let count = selectable.len();

        // A stored row that is gone, or that has since been disabled, resolves to
        // nothing. Activating whatever slid into its position would run an action
        // the user never selected, so an unresolved current row simply has no
        // target until the next navigation key picks one.
        let resolve = {
            let active_state = active_state.clone();
            let selectable = selectable.clone();
            move |cx: &App| -> Option<usize> {
                let stored = active_state.read(cx).clone();
                stored.and_then(|current| {
                    selectable
                        .iter()
                        .position(|(_, candidate)| candidate == &current.id)
                })
            }
        };
        let active: Option<ElementId> =
            resolve(cx).and_then(|position| selectable.get(position).map(|(_, id)| id.clone()));

        let set_active = {
            let active_state = active_state.clone();
            move |cx: &mut App, next: Option<ElementId>, from_pointer: bool| {
                let next = next.map(|id| CurrentRow { id, from_pointer });
                active_state.update(cx, |active, cx| {
                    if *active != next {
                        *active = next;
                        cx.notify();
                    }
                });
            }
        };

        // Leaving the menu clears a pointer-selected row. It must not clear a
        // keyboard-selected one: the pointer would otherwise wipe out the key the
        // user just pressed by drifting across and off the panel.
        let clear_pointer_row: std::rc::Rc<dyn Fn(&mut App)> = {
            let active_state = active_state.clone();
            std::rc::Rc::new(move |cx: &mut App| {
                active_state.update(cx, |active, cx| {
                    if active.as_ref().is_some_and(|row| row.from_pointer) {
                        *active = None;
                        cx.notify();
                    }
                });
            })
        };

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
            .on_hover({
                let clear_pointer_row = clear_pointer_row.clone();
                move |hovered, _, cx| {
                    if !*hovered {
                        clear_pointer_row(cx);
                    }
                }
            })
            .refine_style(&self.style);

        if count > 0 {
            let go_to = {
                let set_active = set_active.clone();
                let selectable = selectable.clone();
                move |cx: &mut App, position: usize| {
                    if let Some((_, id)) = selectable.get(position) {
                        // Navigation is keyboard input, so the pointer no longer
                        // owns the current row.
                        set_active(cx, Some(id.clone()), false);
                    }
                }
            };

            let step_next = {
                let resolve = resolve.clone();
                let go_to = go_to.clone();
                move |cx: &mut App| {
                    let position = match resolve(cx) {
                        None => 0,
                        Some(position) => (position + 1) % count,
                    };
                    go_to(cx, position);
                }
            };
            container = container.on_action(move |_: &crate::MenuNext, _, cx| step_next(cx));

            let resolve_prev = resolve.clone();
            let go_to_prev = go_to.clone();
            container = container.on_action(move |_: &crate::MenuPrev, _, cx| {
                let position = match resolve_prev(cx) {
                    None | Some(0) => count - 1,
                    Some(position) => position - 1,
                };
                go_to_prev(cx, position);
            });

            let go_to_first = go_to.clone();
            container = container.on_action(move |_: &crate::MenuFirst, _, cx| go_to_first(cx, 0));

            let go_to_last = go_to.clone();
            container =
                container.on_action(move |_: &crate::MenuLast, _, cx| go_to_last(cx, count - 1));

            let resolve_activate = resolve.clone();
            let handlers = handlers.clone();
            let selectable = selectable.clone();
            container = container.on_action(move |_: &crate::MenuActivate, window, cx| {
                let Some(position) = resolve_activate(cx) else {
                    return;
                };
                let Some((entry, _)) = selectable.get(position) else {
                    return;
                };
                if let Some(Some(handler)) = handlers.get(*entry) {
                    handler(window, cx);
                }
            });
        }

        // Tab leaves the menu, so it closes. The binding is scoped to the menu
        // context, which shadows root traversal for this subtree.
        container = container.on_action(|_: &crate::MenuTabOut, window, cx| {
            window.dispatch_action(Box::new(crate::Dismiss), cx);
        });

        // Typed-prefix jump. Runs as a key listener rather than a binding because
        // the keys are arbitrary characters; `Enter`, `Tab` and the arrows resolve
        // to actions and never reach here as single characters.
        if count > 0 {
            let typeahead = window.use_keyed_state(
                SharedString::from(format!("menu-typeahead-{}", menu_id)),
                cx,
                |_, _| Typeahead::default(),
            );
            let labels = labels.clone();
            let selectable = selectable.clone();
            let set_active = set_active.clone();
            let resolve = resolve.clone();
            container = container.on_key_down(move |event, _, cx| {
                let Some(character) = typed_character(&event.keystroke) else {
                    return;
                };
                let now = std::time::Instant::now();
                let prefix = typeahead.update(cx, |state, _| {
                    let expired = state
                        .at
                        .is_none_or(|at| now.duration_since(at) > TYPEAHEAD_RESET);
                    // Repeating one character restarts from that character so it
                    // cycles through matches instead of building an unmatchable
                    // prefix like "oo".
                    let repeating = !expired && state.prefix.chars().all(|c| c == character);
                    if expired || repeating {
                        state.prefix.clear();
                    }
                    state.prefix.push(character);
                    state.at = Some(now);
                    state.prefix.clone()
                });

                // Search from the row after the current one, wrapping, so repeated
                // characters advance instead of staying on the first match.
                let start = resolve(cx).map_or(0, |position| position + 1);
                let mut target = None;
                for offset in 0..count {
                    let position = (start + offset) % count;
                    if labels[position].to_lowercase().starts_with(&prefix) {
                        target = Some(position);
                        break;
                    }
                }
                if let Some(position) = target
                    && let Some((_, id)) = selectable.get(position)
                {
                    set_active(cx, Some(id.clone()), false);
                }
            });
        }

        // Render rows, telling each item whether it is the current one. Inside a
        // menu the pointer writes the same state the keyboard does, so only one
        // row can ever look current and Enter always agrees with it.
        let children: Vec<AnyElement> = self
            .entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| match entry {
                Entry::Item(item) => {
                    let id = item.id.clone();
                    let is_active = active.as_ref() == Some(&id);
                    let hover: Option<PointerHandler> = if item.disabled {
                        // Not selectable, but still a place the pointer can be. Report
                        // "clear", so the row the pointer selected a moment ago stops
                        // looking current instead of lingering.
                        Some(clear_pointer_row.clone())
                    } else {
                        let set_active = set_active.clone();
                        Some(std::rc::Rc::new(move |cx: &mut App| {
                            set_active(cx, Some(id.clone()), true)
                        }) as PointerHandler)
                    };
                    (*item)
                        .active(is_active)
                        .pointer_activate(hover)
                        .into_any_element()
                }
                // Rows that cannot be chosen still have to report the pointer:
                // otherwise moving onto a header or a separator leaves the last
                // hovered row looking current, because nothing told the menu that
                // the pointer is now somewhere that is not a row.
                Entry::Separator => {
                    let mut wrapper =
                        filler(clear_pointer_row.clone(), filler_id(&menu_id, "sep", index));
                    wrapper = wrapper.child(MenuSeparator);
                    wrapper.into_any_element()
                }
                Entry::Header(header) => {
                    let mut wrapper = filler(
                        clear_pointer_row.clone(),
                        filler_id(&menu_id, "header", index),
                    );
                    wrapper = wrapper.child(header);
                    wrapper.into_any_element()
                }
                Entry::Custom(element) => element,
            })
            .collect();

        container.children(children)
    }
}

/// The single printable character a keystroke types, if it types one.
///
/// Named keys (`tab`, `enter`, `escape`) and shortcut combinations have no single
/// character and are left to the action bindings.
fn typed_character(keystroke: &gpui::Keystroke) -> Option<char> {
    let modifiers = &keystroke.modifiers;
    if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
        return None;
    }
    let text = keystroke
        .key_char
        .clone()
        .unwrap_or_else(|| keystroke.key.clone());
    let mut chars = text.chars();
    let first = chars.next()?;
    if chars.next().is_some() || first.is_control() {
        return None;
    }
    first.to_lowercase().next()
}

/// Stable identity for a non-selectable row inside one menu.
fn filler_id(menu: &SharedString, kind: &str, index: usize) -> SharedString {
    SharedString::from(format!("{menu}-{kind}-{index}"))
}

/// Wraps a non-selectable row so the pointer reports over it.
fn filler(clear: std::rc::Rc<dyn Fn(&mut App)>, id: SharedString) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .on_mouse_move(move |event, _, cx| {
            if event.pressed_button.is_none() && !cx.has_active_drag() {
                clear(cx);
            }
        })
}

/// A [`Menu`] that opens from a trigger.
///
/// Wraps the panel in a [`Popover`](crate::Popover) so the menu gets anchored
/// placement, outside-click and Escape dismissal. This is composition, not a
/// new interaction model: the popover owns open state and dismissal, the menu
/// owns the rows.
///
/// Keyboard navigation covers arrow keys, Home/End, Enter/Space activation and
/// skipping disabled rows. The pointer writes the same "current row" the keyboard
/// does, so only one row is ever current and activation always matches it.
///
/// Tab closes the menu. Still missing: submenus, and exposing a disabled state
/// through accesskit (this GPUI version has no API for it).
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
    /// Set by [`Menu`]: reports mouse movement over this row, so the menu can
    /// make it the one current row. `None` when the item is used outside a menu,
    /// which falls back to a purely local hover style.
    pointer_activate: Option<PointerHandler>,
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
            pointer_activate: None,
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

    /// Called on mouse movement over this row. Set by [`Menu`].
    pub(crate) fn pointer_activate(mut self, handler: Option<PointerHandler>) -> Self {
        self.pointer_activate = handler;
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

        let foreground = if self.disabled {
            spec.disabled_foreground
        } else {
            spec.foreground
        };

        let label = self.label.clone();
        let leading = self.leading_slot(&spec, foreground);
        // One row can look current, and this is the only input to that decision.
        let background = menu::row_background(&spec, self.active && !self.disabled);
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

        // Pointer reporting is installed even for disabled rows: moving onto one has
        // to tell the menu that the pointer is now over a row it cannot select, so a
        // pointer-selected row stops looking current. What the handler does is
        // decided by `Menu`, which passes "select me" or "clear the current row".
        match self.pointer_activate {
            // Inside a menu the pointer reports into the menu's own current-row
            // state. Painting hover locally as well would put a second row in
            // the "current" look while Enter still used the keyboard's row.
            Some(report) => {
                // on_hover also fires when layout moves a different row under
                // a stationary pointer. Only actual mouse-move events may take
                // over selection; on_mouse_move already checks the row hitbox.
                el = el.on_mouse_move(move |event, _, cx| {
                    if event.pressed_button.is_none() && !cx.has_active_drag() {
                        report(cx);
                    }
                });
            }
            // Standalone (no menu above it) keeps a local hover style.
            None => {
                if !self.disabled {
                    el = el.hover(move |style| {
                        style
                            .bg(spec.hover_background)
                            .text_color(spec.hover_foreground)
                    });
                }
            }
        }

        if !self.disabled {
            el = el.cursor_pointer();
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

#[cfg(test)]
mod input_mode_tests;

#[cfg(test)]
mod typeahead_tests;
