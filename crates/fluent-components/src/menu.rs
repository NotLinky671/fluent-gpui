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
use gpui::prelude::FluentBuilder as _;
use gpui::{Bounds, Pixels, Role};

/// What a menu item does when chosen, by pointer or by keyboard.
pub type MenuHandler = std::rc::Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// Reports that the pointer is over `ElementId`, so its level can make that row
/// current.
type RowHover = std::rc::Rc<dyn Fn(&mut App, ElementId)>;

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
    /// Level the prefix belongs to. Typing is per level: a prefix started on the
    /// root must not keep matching after the keyboard moves into a submenu.
    depth: usize,
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

/// One open submenu level: the row that opened it, and that level's current row.
#[derive(Clone, PartialEq)]
struct OpenLevel {
    parent: ElementId,
    active: Option<ElementId>,
}

/// Navigation state for a menu tree.
///
/// The path is stored by row identity, not by position: the caller may insert,
/// remove or disable rows and a saved index would start pointing at a different
/// action. Invalid segments are dropped by `normalize` before use.
/// A submenu the pointer asked for, waiting out the hover delay.
#[derive(Clone, PartialEq)]
struct PendingOpen {
    row: ElementId,
    deadline: std::time::Instant,
}

#[derive(Clone, Default, PartialEq)]
struct MenuState {
    root: Option<CurrentRow>,
    open: Vec<OpenLevel>,
    /// Set while the pointer rests on a submenu row; replaced or cleared by
    /// any other hover, so a stale deadline can never open an old branch.
    pending: Option<PendingOpen>,
}

/// Painted bounds of the panel and of the row that opens its submenu.
#[derive(Default)]
struct Anchor {
    panel: Option<Bounds<Pixels>>,
    row: Option<Bounds<Pixels>>,
    /// Size of the submenu panel from the previous frame, used to decide whether
    /// it fits on the right and how far it may slide vertically.
    submenu_size: Option<(Pixels, Pixels)>,
}

/// Drops path segments whose parent row is gone, disabled, or has no submenu.
///
/// A stored path is a set of identities, so inserting or removing unrelated rows
/// keeps it valid. Anything that does invalidate a segment truncates from there:
/// keeping the deeper levels would attach them to a row the user never opened.
fn normalize_state(state: &mut MenuState, entries: &[Entry]) {
    if let Some(row) = &state.root {
        let exists = entries
            .iter()
            .any(|entry| matches!(entry, Entry::Item(item) if item.id == row.id && !item.disabled));
        if !exists {
            state.root = None;
        }
    }
    let mut current = entries;
    let mut depth = 0;
    while depth < state.open.len() {
        let parent = state.open[depth].parent.clone();
        let child = current.iter().find_map(|entry| match entry {
            Entry::Item(item) if item.id == parent && !item.disabled => item.submenu.as_deref(),
            _ => None,
        });
        let Some(child) = child else {
            state.open.truncate(depth);
            break;
        };
        if let Some(active) = state.open[depth].active.clone() {
            let still_there = child.entries.iter().any(
                |entry| matches!(entry, Entry::Item(item) if item.id == active && !item.disabled),
            );
            if !still_there {
                state.open[depth].active = None;
            }
        }
        current = &child.entries;
        depth += 1;
    }
}

/// The rows of the level the keyboard is currently on.
fn level_entries<'a>(entries: &'a [Entry], open: &[OpenLevel]) -> &'a [Entry] {
    let mut current = entries;
    for level in open {
        let Some(child) = current.iter().find_map(|entry| match entry {
            Entry::Item(item) if item.id == level.parent => item.submenu.as_deref(),
            _ => None,
        }) else {
            return current;
        };
        current = &child.entries;
    }
    current
}

/// Rows the arrow keys can land on, in render order.
fn selectable_ids(entries: &[Entry]) -> Vec<ElementId> {
    entries
        .iter()
        .filter(|entry| entry.is_selectable())
        .filter_map(|entry| match entry {
            Entry::Item(item) => Some(item.id.clone()),
            _ => None,
        })
        .collect()
}

/// Where a submenu panel goes, relative to the menu panel that owns it.
///
/// The submenu sits flush against the right edge of the row that opened it and
/// level with that row's top, which is what Fluent does for a right-opening
/// submenu. Kept as a pure function so the arithmetic is testable without a
/// window: the renderer only feeds it measured bounds.
fn submenu_offset(row: Bounds<Pixels>, panel: Bounds<Pixels>) -> (Pixels, Pixels) {
    (
        row.origin.x - panel.origin.x + row.size.width,
        row.origin.y - panel.origin.y,
    )
}
/// Where the submenu actually goes: right of the row unless that would leave the
/// window, then flipped to the row's left edge, and clamped vertically so it
/// stays inside the viewport.
///
/// The size comes from the previous frame, because a panel cannot know its own
/// measured size before it is laid out. A zero size means it has not been
/// measured yet, in which case the unflipped position is used.
fn submenu_placement(
    row: Bounds<Pixels>,
    panel: Bounds<Pixels>,
    submenu: (Pixels, Pixels),
    viewport: (Pixels, Pixels),
) -> (Pixels, Pixels) {
    let (base_left, base_top) = submenu_offset(row, panel);
    let (width, height) = submenu;
    let right_edge = panel.origin.x + base_left + width;
    let left = if width > px(0.) && right_edge > viewport.0 {
        row.origin.x - panel.origin.x - width
    } else {
        base_left
    };
    let top = if height > px(0.) {
        let wanted = panel.origin.y + base_top;
        let max_top = (viewport.1 - height).max(px(0.));
        wanted.clamp(px(0.), max_top) - panel.origin.y
    } else {
        base_top
    };
    (left, top)
}
/// Renders a submenu panel: the same chrome and rows, without its own key
/// context, actions or reveal. The parent menu drives it.
fn render_submenu(
    menu: Menu,
    id: ElementId,
    active: Option<ElementId>,
    theme: &FluentTheme,
    shadow: Vec<gpui::BoxShadow>,
    hover: Option<RowHover>,
) -> AnyElement {
    let surface = menu::surface(theme);
    let mut container = div()
        .id(id)
        .flex()
        .flex_col()
        .gap(px(2.))
        .p(surface.padding)
        .min_w(surface.min_width)
        .rounded(surface.radius)
        .bg(surface.background)
        .text_color(surface.foreground)
        .shadow(shadow)
        .role(Role::Menu);

    for entry in menu.entries {
        container = match entry {
            Entry::Item(item) => {
                let is_active = active.as_ref() == Some(&item.id);
                // Rows inside the panel write the same tree state the keyboard
                // does, so the highlighted row and Enter agree at every level.
                let pointer: Option<PointerHandler> = match (&hover, item.disabled) {
                    (Some(report), false) => {
                        let report = report.clone();
                        let row = item.id.clone();
                        Some(std::rc::Rc::new(move |cx: &mut App| {
                            report(cx, row.clone())
                        }))
                    }
                    _ => None,
                };
                container.child((*item).active(is_active).pointer_activate(pointer))
            }
            Entry::Separator => container.child(MenuSeparator),
            Entry::Header(header) => container.child(header),
            Entry::Custom(element) => container.child(element),
        };
    }
    container.into_any_element()
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
    /// Bumped by [`MenuDropdown`] each time the menu opens; a new generation
    /// replays the reveal. Standalone menus keep generation 0 and reveal once.
    reveal: Option<u64>,
    style: StyleRefinement,
}

impl Menu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            focus: None,
            reveal: None,
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

    /// Restarts the reveal animation when the generation changes.
    pub(crate) fn reveal_generation(mut self, generation: u64) -> Self {
        self.reveal = Some(generation);
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
        let menu_id = SharedString::from(format!("{:?}", self.id));

        // Whole-tree navigation state: the root row plus the open submenu path.
        // Stored by row identity, so inserting or removing rows keeps it valid.
        let tree = window.use_keyed_state(
            SharedString::from(format!("menu-tree-{}", menu_id)),
            cx,
            |_, _| MenuState::default(),
        );
        let mut snapshot = tree.read(cx).clone();
        normalize_state(&mut snapshot, &self.entries);

        // A hover that has waited out the delay opens its submenu now. This runs
        // during render rather than in a timer: the deadline lives in state, and
        // any other hover replaces or clears it, so a late deadline cannot open a
        // branch the pointer has already left.
        if let Some(pending) = snapshot.pending.clone() {
            let now = cx.background_executor().now();
            if now >= pending.deadline {
                let first = snapshot
                    .open
                    .last()
                    .map(|open| level_entries(&self.entries, std::slice::from_ref(open)))
                    .unwrap_or(&self.entries)
                    .iter()
                    .find_map(|entry| match entry {
                        Entry::Item(item) if item.id == pending.row => item
                            .submenu
                            .as_deref()
                            .and_then(|child| selectable_ids(&child.entries).into_iter().next()),
                        _ => None,
                    });
                let row = pending.row.clone();
                tree.update(cx, |state, cx| {
                    state.pending = None;
                    if state.open.last().map(|open| &open.parent) != Some(&row) {
                        state.open.push(OpenLevel {
                            parent: row,
                            active: first,
                        });
                    }
                    cx.notify();
                });
                snapshot = tree.read(cx).clone();
                normalize_state(&mut snapshot, &self.entries);
            } else {
                window.request_animation_frame();
            }
        }
        // The level the keyboard is on: the deepest open submenu, else the root.
        let level = level_entries(&self.entries, &snapshot.open);
        let selectable = selectable_ids(level);
        let count = selectable.len();
        let find_item = |id: &ElementId| -> Option<&MenuItem> {
            level.iter().find_map(|entry| match entry {
                Entry::Item(item) if &item.id == id => Some(item.as_ref()),
                _ => None,
            })
        };
        let labels: Vec<SharedString> = selectable
            .iter()
            .map(|id| {
                find_item(id)
                    .map(|item| item.label.clone())
                    .unwrap_or_default()
            })
            .collect();
        let handlers: Vec<Option<MenuHandler>> = selectable
            .iter()
            .map(|id| find_item(id).and_then(|item| item.on_select.clone()))
            .collect();
        let opens: Vec<bool> = selectable
            .iter()
            .map(|id| find_item(id).is_some_and(|item| item.submenu.is_some()))
            .collect();
        // Where a submenu lands when it opens: its first selectable row.
        let targets: Vec<Option<ElementId>> = selectable
            .iter()
            .map(|id| {
                find_item(id)
                    .and_then(|item| item.submenu.as_deref())
                    .and_then(|child| selectable_ids(&child.entries).into_iter().next())
            })
            .collect();

        let set_active = {
            let tree = tree.clone();
            move |cx: &mut App, next: Option<ElementId>, from_pointer: bool| {
                tree.update(cx, |state, cx| {
                    let changed = match state.open.last_mut() {
                        Some(open) => {
                            if open.active == next {
                                false
                            } else {
                                open.active = next;
                                true
                            }
                        }
                        None => {
                            let next = next.map(|id| CurrentRow { id, from_pointer });
                            if state.root == next {
                                false
                            } else {
                                state.root = next;
                                true
                            }
                        }
                    };
                    if changed {
                        cx.notify();
                    }
                });
            }
        };

        // Pointer hover: make the row current, and either schedule its submenu
        // or close the deeper levels when the pointer moves to a sibling row.
        // Moving into the submenu panel is not a hover on a sibling, so it does
        // not close anything.
        let pointer_hover = {
            let tree = tree.clone();
            let set_active = set_active.clone();
            move |cx: &mut App, id: ElementId, opens: bool| {
                set_active(cx, Some(id.clone()), true);
                let now = cx.background_executor().now();
                tree.update(cx, |state, cx| {
                    let mut changed = false;
                    let already_pending =
                        state.pending.as_ref().map(|pending| &pending.row) == Some(&id);
                    if !already_pending && state.pending.take().is_some() {
                        changed = true;
                    }
                    if opens {
                        let already_open = state.open.last().map(|open| &open.parent) == Some(&id);
                        if !already_open && !already_pending {
                            state.pending = Some(PendingOpen {
                                row: id.clone(),
                                deadline: now + fluent_core::spec::motion::DURATION_MENU_HOVER_OPEN,
                            });
                            changed = true;
                        }
                    } else if !state.open.is_empty() {
                        state.open.clear();
                        changed = true;
                    }
                    if changed {
                        cx.notify();
                    }
                });
            }
        };
        // Leaving the menu clears a pointer-selected row. It must not clear a
        // keyboard-selected one: the pointer would otherwise wipe out the key the
        // user just pressed by drifting across and off the panel.
        let clear_pointer_row: std::rc::Rc<dyn Fn(&mut App)> = {
            let tree = tree.clone();
            std::rc::Rc::new(move |cx: &mut App| {
                tree.update(cx, |state, cx| {
                    let changed = match state.open.last_mut() {
                        Some(open) => open.active.take().is_some(),
                        None => match &state.root {
                            Some(row) if row.from_pointer => {
                                state.root = None;
                                true
                            }
                            _ => false,
                        },
                    };
                    if changed {
                        cx.notify();
                    }
                });
            })
        };

        let open_submenu = {
            let tree = tree.clone();
            move |cx: &mut App, row: ElementId, first: Option<ElementId>| {
                tree.update(cx, |state, cx| {
                    if state.open.last().map(|open| &open.parent) == Some(&row) {
                        return;
                    }
                    state.open.push(OpenLevel {
                        parent: row,
                        active: first,
                    });
                    cx.notify();
                });
            }
        };

        let close_level = {
            let tree = tree.clone();
            move |cx: &mut App| {
                tree.update(cx, |state, cx| {
                    if state.open.pop().is_some() {
                        cx.notify();
                    }
                });
            }
        };

        let any_level_open = {
            let tree = tree.clone();
            move |cx: &App| !tree.read(cx).open.is_empty()
        };

        let mut container = div()
            .id(self.id)
            .relative()
            .flex()
            .flex_col()
            .gap(px(2.))
            .p(surface.padding)
            .min_w(surface.min_width)
            .rounded(surface.radius)
            .bg(surface.background)
            .text_color(surface.foreground)
            .shadow(shadow.clone())
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
                    if let Some(id) = selectable.get(position) {
                        // Navigation is keyboard input, so the pointer no longer
                        // owns the current row.
                        set_active(cx, Some(id.clone()), false);
                    }
                }
            };
            let resolve = {
                let tree = tree.clone();
                let selectable = selectable.clone();
                move |cx: &App| -> Option<usize> {
                    let state = tree.read(cx);
                    let active = match state.open.last() {
                        None => state.root.as_ref().map(|row| row.id.clone()),
                        Some(open) => open.active.clone(),
                    };
                    active.and_then(|id| selectable.iter().position(|candidate| candidate == &id))
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

            // Enter, Space and Right all open a submenu; only Enter runs a plain
            // row. Opening is decided by the row, not by the key.
            let open_current = {
                let resolve = resolve.clone();
                let open_submenu = open_submenu.clone();
                let selectable = selectable.clone();
                let targets = targets.clone();
                move |cx: &mut App| {
                    if let Some(position) = resolve(cx)
                        && let Some(id) = selectable.get(position)
                    {
                        open_submenu(cx, id.clone(), targets[position].clone());
                    }
                }
            };

            let open_for_activate = open_current.clone();
            let resolve_activate = resolve.clone();
            let handlers = handlers.clone();
            let opens = opens.clone();
            container = container.on_action(move |_: &crate::MenuActivate, window, cx| {
                let Some(position) = resolve_activate(cx) else {
                    return;
                };
                if opens[position] {
                    open_for_activate(cx);
                    return;
                }
                if let Some(Some(handler)) = handlers.get(position) {
                    handler(window, cx);
                }
            });

            let open_for_right = open_current.clone();
            container =
                container.on_action(move |_: &crate::MenuSubmenuOpen, _, cx| open_for_right(cx));

            let typeahead = window.use_keyed_state(
                SharedString::from(format!("menu-typeahead-{}", menu_id)),
                cx,
                |_, _| Typeahead::default(),
            );
            let tree_for_depth = tree.clone();
            let labels = labels.clone();
            let selectable_typing = selectable.clone();
            let set_active_typing = set_active.clone();
            let resolve_typing = resolve.clone();
            container = container.on_key_down(move |event, _, cx| {
                let Some(character) = typed_character(&event.keystroke) else {
                    return;
                };
                let now = std::time::Instant::now();
                let depth = tree_for_depth.read(cx).open.len();
                let prefix = typeahead.update(cx, |state, _| {
                    // Typing is per level: a prefix started elsewhere must not
                    // keep matching after the keyboard changes level.
                    if state.depth != depth {
                        state.prefix.clear();
                        state.at = None;
                        state.depth = depth;
                    }
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
                let start = resolve_typing(cx).map_or(0, |position| position + 1);
                let mut target = None;
                for offset in 0..count {
                    let position = (start + offset) % count;
                    if labels[position].to_lowercase().starts_with(&prefix) {
                        target = Some(position);
                        break;
                    }
                }
                if let Some(position) = target
                    && let Some(id) = selectable_typing.get(position)
                {
                    set_active_typing(cx, Some(id.clone()), false);
                }
            });
        }

        let close_for_left = close_level.clone();
        container =
            container.on_action(move |_: &crate::MenuSubmenuClose, _, cx| close_for_left(cx));

        // Escape closes one level; at the root it closes the whole menu. Tab is
        // separate: it closes the entire chain whatever level it is pressed on.
        let close_for_escape = close_level.clone();
        let any_level_open_for_escape = any_level_open.clone();
        container = container.on_action(move |_: &crate::MenuEscape, window, cx| {
            if any_level_open_for_escape(cx) {
                close_for_escape(cx);
            } else {
                window.dispatch_action(Box::new(crate::Dismiss), cx);
            }
        });

        container = container.on_action(|_: &crate::MenuTabOut, window, cx| {
            window.dispatch_action(Box::new(crate::Dismiss), cx);
        });

        // Rows of the root level. The row that opened a submenu hands its content
        // over: the panel is rendered once its painted bounds are known.
        let root_active = snapshot.root.as_ref().map(|row| row.id.clone());
        let anchor_row = snapshot.open.first().map(|open| open.parent.clone());
        let anchor_state = window.use_keyed_state(
            SharedString::from(format!("menu-anchor-{}", menu_id)),
            cx,
            |_, _| std::rc::Rc::new(std::cell::RefCell::new(Anchor::default())),
        );
        let anchor = anchor_state.read(cx).clone();
        let mut open_child: Option<Box<Menu>> = None;

        let children: Vec<AnyElement> = self
            .entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| match entry {
                Entry::Item(item) => {
                    let mut item = *item;
                    let id = item.id.clone();
                    let is_active = root_active.as_ref() == Some(&id);
                    let is_anchor = anchor_row.as_ref() == Some(&id);
                    if is_anchor {
                        let child = item.submenu.take();
                        item = item.submenu_marker(true);
                        open_child = child;
                    }
                    let opens_submenu = item.submenu.is_some() || item.submenu_marker;
                    let hover: Option<PointerHandler> = if item.disabled {
                        // Not selectable, but still a place the pointer can be. Report
                        // "clear", so the row the pointer selected a moment ago stops
                        // looking current instead of lingering.
                        Some(clear_pointer_row.clone())
                    } else {
                        let pointer_hover = pointer_hover.clone();
                        Some(std::rc::Rc::new(move |cx: &mut App| {
                            pointer_hover(cx, id.clone(), opens_submenu)
                        }) as PointerHandler)
                    };
                    let element = item
                        .active(is_active)
                        .pointer_activate(hover)
                        .into_any_element();
                    if is_anchor {
                        let slot = anchor.clone();
                        div()
                            .on_children_prepainted(move |children, _, _| {
                                if let Some(bounds) = children.first() {
                                    slot.borrow_mut().row = Some(*bounds);
                                }
                            })
                            .child(element)
                            .into_any_element()
                    } else {
                        element
                    }
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

        let mut container = container.children(children);

        // The submenu panel hangs off the row that opened it. Its bounds come from
        // the previous frame: on the first frame there is nothing to anchor to, so
        // it is not painted rather than flashing at the panel origin.
        if let Some(child) = open_child {
            let (row_bounds, panel_bounds) = {
                let anchor = anchor.borrow();
                (anchor.row, anchor.panel)
            };
            if let (Some(row_bounds), Some(panel_bounds)) = (row_bounds, panel_bounds) {
                let (submenu_width, submenu_height) = {
                    let anchor = anchor.borrow();
                    anchor.submenu_size.unwrap_or((px(0.), px(0.)))
                };
                let viewport = window.viewport_size();
                let (left, top) = submenu_placement(
                    row_bounds,
                    panel_bounds,
                    (submenu_width, submenu_height),
                    (viewport.width, viewport.height),
                );
                let submenu_active = snapshot.open.first().and_then(|open| open.active.clone());
                let hover_set_active = set_active.clone();
                let hover: Option<RowHover> =
                    Some(std::rc::Rc::new(move |cx: &mut App, row: ElementId| {
                        hover_set_active(cx, Some(row), true);
                    }));
                let panel = render_submenu(
                    *child,
                    ElementId::from(SharedString::from(format!("{}-submenu", menu_id))),
                    submenu_active,
                    &theme,
                    shadow.clone(),
                    hover,
                );
                let size_slot = anchor.clone();
                container = container.child(
                    div()
                        .absolute()
                        .left(left)
                        .top(top)
                        .on_children_prepainted(move |children, _, _| {
                            if let Some(bounds) = children.first() {
                                size_slot.borrow_mut().submenu_size =
                                    Some((bounds.size.width, bounds.size.height));
                            }
                        })
                        .child(panel),
                );
            }
        }

        // Record this panel's own bounds for the submenu positioning above.
        let panel_slot = anchor.clone();
        let container = div()
            .on_children_prepainted(move |children, _, _| {
                if let Some(bounds) = children.first() {
                    panel_slot.borrow_mut().panel = Some(*bounds);
                }
            })
            .child(container);

        // Reveal: the panel appears at its final size and fades in. Measuring the
        // Fluent docs menu shows width, edges and top staying constant while only
        // the opacity ramps, so this is a fade and not a grow.
        //
        // Only a menu that a dropdown opens reveals. A menu sitting in a page is
        // already there; fading it in on mount would flash every time its host
        // rebuilt the tree.
        let Some(generation) = self.reveal else {
            return container.into_any_element();
        };
        if cx.reduce_motion() {
            return container.into_any_element();
        }

        let reveal_state = window.use_keyed_state(
            SharedString::from(format!("menu-reveal-{}", menu_id)),
            cx,
            |_, _| (u64::MAX, None::<std::time::Instant>),
        );
        let now = cx.background_executor().now();
        let (stored_generation, stored_start) = *reveal_state.read(cx);
        let started = match stored_start {
            Some(start) if stored_generation == generation => start,
            _ => {
                reveal_state.update(cx, |state, _| {
                    state.0 = generation;
                    state.1 = Some(now);
                });
                now
            }
        };
        let elapsed = now.saturating_duration_since(started).as_secs_f32();
        let duration = fluent_core::spec::motion::DURATION_MENU_REVEAL.as_secs_f32();
        let progress = (elapsed / duration).clamp(0.0, 1.0);
        if progress >= 1.0 {
            return container.into_any_element();
        }
        window.request_animation_frame();
        let opacity = fluent_core::spec::motion::menu_reveal_opacity(progress);
        // Measured from the same recording: the content also settles downward by a
        // few pixels, so it starts slightly closer to the trigger and drops into
        // place while the opacity ramps.
        let rise = fluent_core::spec::motion::menu_reveal_offset(progress);
        container
            .relative()
            .top(px(rise))
            .opacity(opacity)
            .into_any_element()
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
        // Each open bumps the generation so the reveal replays; the menu is
        // unmounted while closed, so a stable generation would animate only once
        // per window.
        let generation = window.use_keyed_state(
            SharedString::from(format!("menu-reveal-generation-{:?}", self.id)),
            cx,
            |_, _| 0_u64,
        );

        let mut menu = self.menu.reveal_generation(*generation.read(cx));
        if let Some(focus) = self.focus.as_ref() {
            menu = menu.track_focus(focus);
        }

        let mut popover = crate::Popover::new(self.id)
            .default_open(self.default_open)
            // The menu IS the surface: a panel chrome around it would stack a
            // second card and inset every row.
            .draw_panel_surface(false)
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
        let bump = generation.clone();
        match self.on_open_change {
            Some(handler) => {
                popover = popover.on_open_change(move |next, window, cx| {
                    if next {
                        bump.update(cx, |value, _| *value += 1);
                    }
                    handler(next, window, cx);
                });
            }
            None => {
                popover = popover.on_open_change(move |next, _, cx| {
                    if next {
                        bump.update(cx, |value, _| *value += 1);
                    }
                });
            }
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
    /// Nested menu content. The parent menu owns navigation for it: the child
    /// contributes rows and styling only, and carries no state of its own.
    submenu: Option<Box<Menu>>,
    /// Set when the menu took the submenu out to render it, so the chevron stays.
    submenu_marker: bool,
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
            submenu: None,
            submenu_marker: false,
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

    /// Nested menu opened by this row.
    ///
    /// The child is content, not a callback: the parent menu walks its rows to
    /// navigate, activate and close it. Only one level of nesting is rendered in
    /// this slice; deeper rows still show the chevron but do not open.
    pub fn submenu(mut self, menu: Menu) -> Self {
        self.submenu = Some(Box::new(menu));
        self
    }

    /// Whether this row opens a submenu.
    pub fn has_submenu(&self) -> bool {
        self.submenu.is_some()
    }

    /// Keeps the chevron once the menu has taken the submenu out to render it.
    pub(crate) fn submenu_marker(mut self, marked: bool) -> Self {
        self.submenu_marker = marked;
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

        // A row that opens a submenu ends with a chevron, pushed to the trailing
        // edge so labels stay left-aligned whatever their length.
        if self.submenu.is_some() || self.submenu_marker {
            el = el
                .child(div().flex_1())
                .child(Icon::new(FluentIcon::ChevronRight).size(px(12.)));
        }

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

#[cfg(test)]
mod surface_tests;

#[cfg(test)]
mod reveal_tests;

#[cfg(test)]
mod submenu_tests;
