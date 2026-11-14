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
use crate::popover::{InteriorRegions, OpenChangeHandler};
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
use gpui::AppContext as _;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Bounds, Display, Element, GlobalElementId, InspectorElementId, LayoutId, Pixels, Point,
    Position, Role, Size, Style,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// What a menu item does when chosen, by pointer or by keyboard.
pub type MenuHandler = std::rc::Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// Reports a mouse-move event over a row, so the menu can make it current.
type PointerHandler = Rc<dyn Fn(&mut App)>;

/// Reports pointer movement over a selectable row of one level.
type LevelHover = Rc<dyn Fn(&mut App, Vec<ElementId>, ElementId, bool)>;

/// Reports pointer movement over a non-selectable row of one level.
type LevelClear = Rc<dyn Fn(&mut App, Vec<ElementId>)>;

/// Reports that the pointer left one panel of the chain.
type PanelLeave = Rc<dyn Fn(&mut Window, &mut App, Vec<ElementId>)>;

/// Reports an actual pointer move inside one panel, with that panel's rows.
type PanelMove = Rc<dyn Fn(&mut App, Vec<ElementId>, RowBounds, Point<Pixels>)>;

/// Receives text committed by the platform input method.
pub(super) type MenuImeCommit = Rc<dyn Fn(&str, &mut Window, &mut App)>;
/// Measured bounds of the rows drawn in one panel of the menu chain.
///
/// The map is rebuilt into a fresh frame-local value on every render, so a row
/// that moves, disappears or is reused in another level cannot report the bounds
/// of a different row.
type RowBounds = Rc<RefCell<HashMap<ElementId, Bounds<Pixels>>>>;

/// Navigation state for the menu chain, stored in window keyed state.
type MenuTree = gpui::Entity<MenuState>;

/// Typed-prefix state for jumping to a row by its label.
///
/// A prefix older than the reset window is discarded, and typing the same
/// character again restarts from that character instead of building "aa", so
/// pressing a letter repeatedly cycles through the rows starting with it.
#[derive(Clone, Default)]
struct Typeahead {
    prefix: String,
    at: Option<std::time::Instant>,
    /// Path of the level the prefix belongs to: the rows that open it, from the
    /// root inwards. Depth alone is not an identity — two branches at the same
    /// depth are different levels and must not share a prefix.
    path: Vec<ElementId>,
    /// Menu session the prefix belongs to. A reopened menu starts a new session.
    session: Option<u64>,
    /// Level-visit counter. Unlike `path`, this changes on every open, close or
    /// truncation, even when the pointer/keyboard returns to the same path.
    level_epoch: u64,
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
    active: Option<CurrentRow>,
}

/// A submenu the pointer asked for, waiting out the hover delay.
#[derive(Clone, PartialEq)]
struct PendingOpen {
    /// Path of the level containing `row` (the rows that open it, from the root).
    path: Vec<ElementId>,
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
    /// Reveal generation of the dropdown that hosts this menu. A new session
    /// (close, then reopen) must not revive the old open path or hover request.
    session: Option<u64>,
    /// Incremented whenever the open path changes, so typeahead can tell "still
    /// on the same visit" from "left and came back to the same path".
    level_epoch: u64,
}

fn bump_level_epoch(state: &mut MenuState) {
    state.level_epoch = state.level_epoch.wrapping_add(1);
}

fn reset_typeahead(typeahead: &gpui::Entity<Typeahead>, cx: &mut App) {
    typeahead.update(cx, |typeahead, _| {
        typeahead.prefix.clear();
        typeahead.at = None;
    });
}

/// One item row as read from the live data tree for this frame.
#[derive(Clone)]
struct RowSnapshot {
    id: ElementId,
    label: SharedString,
    disabled: bool,
    has_submenu: bool,
    /// First selectable row of the submenu, to land on when it opens.
    first_child: Option<ElementId>,
    on_select: Option<MenuHandler>,
}

/// All rows of one level, in render order. `path` is the chain of rows that
/// opens this level; the root level's path is empty.
#[derive(Clone)]
struct LevelSnapshot {
    path: Vec<ElementId>,
    rows: Vec<RowSnapshot>,
}

impl LevelSnapshot {
    fn row(&self, id: &ElementId) -> Option<&RowSnapshot> {
        self.rows.iter().find(|row| &row.id == id)
    }

    fn selectable(&self) -> impl Iterator<Item = &RowSnapshot> {
        self.rows.iter().filter(|row| !row.disabled)
    }
}

/// Reads the whole data tree once, so every level can be resolved by path.
fn snapshot_levels(entries: &[Entry], path: &[ElementId], out: &mut Vec<LevelSnapshot>) {
    let mut rows = Vec::new();
    for entry in entries {
        if let Entry::Item(item) = entry {
            let first_child = item
                .submenu
                .as_deref()
                .and_then(|child| first_selectable(&child.entries));
            rows.push(RowSnapshot {
                id: item.id.clone(),
                label: item.label.clone(),
                disabled: item.disabled,
                has_submenu: item.submenu.is_some(),
                first_child,
                on_select: item.on_select.clone(),
            });
        }
    }
    out.push(LevelSnapshot {
        path: path.to_vec(),
        rows,
    });

    for entry in entries {
        if let Entry::Item(item) = entry
            && let Some(child) = item.submenu.as_deref()
        {
            let mut child_path = path.to_vec();
            child_path.push(item.id.clone());
            snapshot_levels(&child.entries, &child_path, out);
        }
    }
}

fn first_selectable(entries: &[Entry]) -> Option<ElementId> {
    entries.iter().find_map(|entry| match entry {
        Entry::Item(item) if !item.disabled => Some(item.id.clone()),
        _ => None,
    })
}

fn level_by_path<'a>(levels: &'a [LevelSnapshot], path: &[ElementId]) -> Option<&'a LevelSnapshot> {
    levels.iter().find(|level| level.path == path)
}

fn level_path(state: &MenuState) -> Vec<ElementId> {
    state
        .open
        .iter()
        .map(|level| level.parent.clone())
        .collect()
}

fn deepest_active(state: &MenuState) -> Option<ElementId> {
    match state.open.last() {
        None => state.root.as_ref().map(|row| row.id.clone()),
        Some(level) => level.active.as_ref().map(|row| row.id.clone()),
    }
}

/// Drops path segments whose parent row is gone, disabled, or has no submenu.
///
/// A stored path is a set of identities, so inserting or removing unrelated rows
/// keeps it valid. Anything that does invalidate a segment truncates from there:
/// keeping the deeper levels would attach them to a row the user never opened.
/// The result is written back by the caller, so rendering, keys, pointer and the
/// pending hover request all read the same state.
fn normalize_state(state: &mut MenuState, levels: &[LevelSnapshot]) -> bool {
    let before = state.clone();

    if let Some(row) = &state.root {
        let valid = level_by_path(levels, &[])
            .and_then(|level| level.row(&row.id))
            .is_some_and(|row| !row.disabled);
        if !valid {
            state.root = None;
        }
    }

    let mut path: Vec<ElementId> = Vec::new();
    let mut depth = 0;
    while depth < state.open.len() {
        let parent = state.open[depth].parent.clone();
        let Some(level) = level_by_path(levels, &path) else {
            state.open.truncate(depth);
            break;
        };
        let Some(row) = level.row(&parent) else {
            state.open.truncate(depth);
            break;
        };
        if row.disabled || !row.has_submenu {
            state.open.truncate(depth);
            break;
        }
        path.push(parent);
        let Some(child) = level_by_path(levels, &path) else {
            state.open.truncate(depth);
            break;
        };
        if let Some(active) = &state.open[depth].active {
            let valid = child.row(&active.id).is_some_and(|row| !row.disabled);
            if !valid {
                state.open[depth].active = None;
            }
        }
        depth += 1;
    }

    if let Some(pending) = &state.pending {
        // A pending request belongs to the deepest open level. A keyboard action,
        // a closed level or a truncated branch makes it stale.
        let same_path = pending.path.len() == state.open.len()
            && pending
                .path
                .iter()
                .zip(&state.open)
                .all(|(id, level)| id == &level.parent);
        let valid = same_path
            && level_by_path(levels, &pending.path)
                .and_then(|level| level.row(&pending.row))
                .is_some_and(|row| !row.disabled && row.has_submenu);
        if !valid {
            state.pending = None;
        }
    }

    // Any path change made by normalization (a removed/disabled parent, a lost
    // submenu) also ends the previous level visit for typeahead purposes.
    if state.open != before.open {
        bump_level_epoch(state);
    }

    *state != before
}

/// Closes the whole chain on Tab and asks the host to hand focus to the next
/// (or previous) control. The direction travels as a distinct action: a unit
/// `Dismiss` cannot tell Shift+Tab from a click or Escape.
fn tab_out(
    tree: &MenuTree,
    levels: &[LevelSnapshot],
    typeahead: &gpui::Entity<Typeahead>,
    forward: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let (mut state, mut changed) = read_normalized(tree, cx, levels);
    changed |= state.pending.take().is_some();
    reset_typeahead(typeahead, cx);
    commit(tree, cx, &state, changed);
    let action: Box<dyn gpui::Action> = if forward {
        Box::new(crate::DismissAndFocusNext)
    } else {
        Box::new(crate::DismissAndFocusPrev)
    };
    window.dispatch_action(action, cx);
}

/// Applies one typed character (keyboard) or one committed string (IME) to the
/// typeahead prefix. The committed string is matched as-is; there is no pinyin
/// or candidate special-casing, because the platform already resolved it.
fn apply_typeahead(
    tree: &MenuTree,
    levels: &[LevelSnapshot],
    typeahead: &gpui::Entity<Typeahead>,
    text: &str,
    cx: &mut App,
) {
    let (mut state, mut changed) = read_normalized(tree, cx, levels);
    // Typing is keyboard input: it cancels a hover that has not fired.
    changed |= state.pending.take().is_some();
    let path = level_path(&state);
    let session = state.session;
    let level_epoch = state.level_epoch;
    let now = std::time::Instant::now();
    let prefix = typeahead.update(cx, |typeahead, _| {
        // Typing belongs to one *visit* of a level: the full path is not
        // enough, because leaving and re-entering the same branch ends at the
        // same path. The epoch changes on every open, close or truncation, so
        // such a re-entry starts fresh.
        if typeahead.path != path
            || typeahead.session != session
            || typeahead.level_epoch != level_epoch
        {
            typeahead.prefix.clear();
            typeahead.at = None;
            typeahead.path = path.clone();
            typeahead.session = session;
            typeahead.level_epoch = level_epoch;
        }
        let expired = typeahead
            .at
            .is_none_or(|at| now.duration_since(at) > TYPEAHEAD_RESET);
        // Repeating one character restarts from that character so it cycles
        // through matches instead of building an unmatchable prefix like "oo".
        let mut characters = text.chars();
        let first = characters.next();
        let single = first.is_some() && characters.next().is_none();
        let repeating = !expired && single && typeahead.prefix.chars().all(|c| Some(c) == first);
        if expired || repeating {
            typeahead.prefix.clear();
        }
        typeahead.prefix.push_str(text);
        typeahead.at = Some(now);
        typeahead.prefix.clone()
    });

    if let Some(level) = level_by_path(levels, &path) {
        let rows: Vec<&RowSnapshot> = level.selectable().collect();
        if !rows.is_empty() {
            // Search from the row after the current one, wrapping, so
            // repeated characters advance instead of staying on the first
            // match.
            let current = deepest_active(&state);
            let start = current
                .and_then(|id| rows.iter().position(|row| row.id == id))
                .map_or(0, |position| position + 1);
            let mut target = None;
            for offset in 0..rows.len() {
                let position = (start + offset) % rows.len();
                if rows[position].label.to_lowercase().starts_with(&prefix) {
                    target = Some(rows[position].id.clone());
                    break;
                }
            }
            if let Some(id) = target {
                changed |= set_level_active(&mut state, levels, &path, Some(id), false);
            }
        }
    }
    commit(tree, cx, &state, changed);
}

fn read_normalized(tree: &MenuTree, cx: &App, levels: &[LevelSnapshot]) -> (MenuState, bool) {
    let mut state = tree.read(cx).clone();
    let changed = normalize_state(&mut state, levels);
    (state, changed)
}

fn commit(tree: &MenuTree, cx: &mut App, state: &MenuState, changed: bool) {
    if changed {
        let next = state.clone();
        tree.update(cx, |slot, cx| {
            *slot = next;
            cx.notify();
        });
    }
}

/// Sets one level's current row, without touching the levels above or below it.
///
/// The level is addressed by its path, so a pointer report from a submenu can
/// never write the root row (which is what made Enter act on a different row
/// than the one that looked current).
fn set_level_active(
    state: &mut MenuState,
    levels: &[LevelSnapshot],
    path: &[ElementId],
    next: Option<ElementId>,
    from_pointer: bool,
) -> bool {
    let Some(level) = level_by_path(levels, path) else {
        return false;
    };
    if let Some(id) = &next
        && level.row(id).is_none_or(|row| row.disabled)
    {
        return false;
    }
    let next = next.map(|id| CurrentRow { id, from_pointer });
    if path.is_empty() {
        if state.root == next {
            false
        } else {
            state.root = next;
            true
        }
    } else if let Some(open) = state.open.get_mut(path.len() - 1) {
        if open.active == next {
            false
        } else {
            open.active = next;
            true
        }
    } else {
        false
    }
}

/// Opens the next level below the current row, if that row actually has one.
///
/// Only the deepest open level can be extended, and only a live, enabled row
/// with a submenu can extend it. This is what keeps Right on a leaf from
/// creating an invisible, activatable phantom level.
fn open_below(
    state: &mut MenuState,
    path: &[ElementId],
    row: &ElementId,
    first_child: Option<ElementId>,
    from_pointer: bool,
) -> bool {
    // Identity is the full path: the row is in level `path.len()`, so extending
    // it always appends one level. Comparing the bare id against
    // `state.open.last()` would compare rows from different levels, which may
    // legitimately share an id. (If this level were already extended, the length
    // check above would have failed.)
    if path.len() != state.open.len() {
        return false;
    }
    state.open.push(OpenLevel {
        parent: row.clone(),
        active: first_child.map(|id| CurrentRow { id, from_pointer }),
    });
    state.pending = None;
    bump_level_epoch(state);
    true
}

/// Pops one level and hands the keyboard back to the row that opened it.
fn close_level(state: &mut MenuState) -> bool {
    let Some(level) = state.open.pop() else {
        return false;
    };
    let current = CurrentRow {
        id: level.parent,
        from_pointer: false,
    };
    match state.open.last_mut() {
        Some(parent) => parent.active = Some(current),
        None => state.root = Some(current),
    }
    state.pending = None;
    bump_level_epoch(state);
    true
}

enum NavStep {
    Next,
    Prev,
    First,
    Last,
}

fn navigate(tree: &MenuTree, levels: &[LevelSnapshot], cx: &mut App, step: NavStep) {
    let (mut state, mut changed) = read_normalized(tree, cx, levels);
    // Any keyboard navigation supersedes a hover that has not fired yet.
    changed |= state.pending.take().is_some();
    let path = level_path(&state);
    if let Some(level) = level_by_path(levels, &path) {
        let ids: Vec<ElementId> = level.selectable().map(|row| row.id.clone()).collect();
        if !ids.is_empty() {
            let current = deepest_active(&state);
            let position = current.and_then(|id| ids.iter().position(|candidate| candidate == &id));
            let next = match step {
                NavStep::Next => position.map_or(0, |position| (position + 1) % ids.len()),
                NavStep::Prev => position.map_or(ids.len() - 1, |position| {
                    if position == 0 {
                        ids.len() - 1
                    } else {
                        position - 1
                    }
                }),
                NavStep::First => 0,
                NavStep::Last => ids.len() - 1,
            };
            changed |= set_level_active(&mut state, levels, &path, Some(ids[next].clone()), false);
        }
    }
    commit(tree, cx, &state, changed);
}

/// Runs the current row, or opens its submenu when it has one.
fn activate_row(tree: &MenuTree, levels: &[LevelSnapshot], window: &mut Window, cx: &mut App) {
    let (mut state, mut changed) = read_normalized(tree, cx, levels);
    changed |= state.pending.take().is_some();
    let path = level_path(&state);
    let row = level_by_path(levels, &path).and_then(|level| {
        deepest_active(&state)
            .and_then(|id| level.row(&id))
            .cloned()
    });
    let Some(row) = row else {
        commit(tree, cx, &state, changed);
        return;
    };
    if row.has_submenu {
        changed |= open_below(&mut state, &path, &row.id, row.first_child.clone(), false);
        commit(tree, cx, &state, changed);
        return;
    }
    commit(tree, cx, &state, changed);
    if let Some(handler) = row.on_select {
        handler(window, cx);
    }
}

fn open_current_row(tree: &MenuTree, levels: &[LevelSnapshot], cx: &mut App) {
    let (mut state, mut changed) = read_normalized(tree, cx, levels);
    changed |= state.pending.take().is_some();
    let path = level_path(&state);
    let row = level_by_path(levels, &path).and_then(|level| {
        deepest_active(&state)
            .and_then(|id| level.row(&id))
            .cloned()
    });
    if let Some(row) = row
        && row.has_submenu
    {
        changed |= open_below(&mut state, &path, &row.id, row.first_child.clone(), false);
    }
    commit(tree, cx, &state, changed);
}

/// Where a submenu panel goes in window coordinates.
///
/// The panel hangs off the right edge of the row that opened it, flips to the
/// row's left edge when it would leave the window, and is clamped into the
/// viewport on both axes. The size is this frame's measured panel size, so the
/// first frame of a newly opened level is already placed correctly.
fn submenu_origin(
    row: Bounds<Pixels>,
    size: Size<Pixels>,
    viewport: Size<Pixels>,
) -> Point<Pixels> {
    let right = row.origin.x + row.size.width;
    let left = if right + size.width > viewport.width {
        row.origin.x - size.width
    } else {
        right
    };
    let left = left.clamp(px(0.), (viewport.width - size.width).max(px(0.)));
    let max_top = (viewport.height - size.height).max(px(0.));
    let top = row.origin.y.clamp(px(0.), max_top);
    gpui::point(left, top)
}

/// Places one submenu panel in the same frame it is laid out.
///
/// The parent's rows are prepainted before this element (it is appended after
/// them), so the anchor row's bounds are available here; the panel's own size
/// comes from its layout. That removes the previous one-frame guess where the
/// first paint used an unknown size and the panel could flash at the origin.
struct SubmenuPositioner {
    panel: AnyElement,
    row_bounds: RowBounds,
    anchor: ElementId,
    depth: usize,
    regions: InteriorRegions,
}

impl IntoElement for SubmenuPositioner {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SubmenuPositioner {
    type RequestLayoutState = LayoutId;
    type PrepaintState = bool;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, LayoutId) {
        let panel_layout = self.panel.request_layout(window, cx);
        let layout = window.request_layout(
            Style {
                position: Position::Absolute,
                display: Display::Flex,
                ..Style::default()
            },
            [panel_layout],
            cx,
        );
        (layout, panel_layout)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        panel_layout: &mut LayoutId,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let panel = window.layout_bounds(*panel_layout);
        // Layout bounds already include the current absolute element offset, so
        // the row bounds recorded this frame are in window coordinates too.
        let base = window.element_offset();
        let Some(row) = self.row_bounds.borrow().get(&self.anchor).copied() else {
            // The anchor row is not measured this frame. Never leave the level
            // invisible while it stays activatable: paint it where it already
            // lays out and ask for another frame, where the row will be measured.
            window.request_animation_frame();
            window.with_absolute_element_offset(base, |window| self.panel.prepaint(window, cx));
            return true;
        };
        let origin = submenu_origin(row, panel.size, window.viewport_size());
        // `bounds.origin` is layout origin + `base`; convert back to the raw
        // layout origin before adding the absolute offset that places the panel.
        let offset = origin - bounds.origin + base;
        {
            let mut regions = self.regions.borrow_mut();
            if self.depth < regions.len() {
                regions[self.depth] = Some(Bounds::new(origin, panel.size));
            }
        }
        window.with_absolute_element_offset(offset, |window| self.panel.prepaint(window, cx));
        true
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _panel_layout: &mut LayoutId,
        placed: &mut bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        if *placed {
            self.panel.paint(window, cx);
        }
    }
}

/// Per-frame inputs shared by every panel in the menu chain.
struct PanelContext {
    /// String form of the root id, used for stable non-row identities.
    root_name: SharedString,
    theme: FluentTheme,
    shadow: Vec<gpui::BoxShadow>,
    regions: InteriorRegions,
    hover: LevelHover,
    clear: LevelClear,
    leave: PanelLeave,
    moved: PanelMove,
}

fn level_active_id(state: &MenuState, depth: usize) -> Option<ElementId> {
    if depth == 0 {
        state.root.as_ref().map(|row| row.id.clone())
    } else {
        state
            .open
            .get(depth - 1)
            .and_then(|level| level.active.as_ref())
            .map(|row| row.id.clone())
    }
}

/// Builds one level's rows, taking the anchor row's submenu out for rendering.
fn build_rows(
    entries: Vec<Entry>,
    depth: usize,
    path: &[ElementId],
    state: &MenuState,
    ctx: &PanelContext,
) -> (Vec<AnyElement>, Option<Box<Menu>>, RowBounds) {
    let active_id = level_active_id(state, depth);
    let expanded_id = state.open.get(depth).map(|level| level.parent.clone());
    let row_bounds: RowBounds = Rc::default();
    let mut child: Option<Box<Menu>> = None;
    let mut rows = Vec::with_capacity(entries.len());

    for (index, entry) in entries.into_iter().enumerate() {
        match entry {
            Entry::Item(item) => {
                let mut item = *item;
                let id = item.id.clone();
                let is_active = active_id.as_ref() == Some(&id);
                let is_expanded = expanded_id.as_ref() == Some(&id);
                if is_expanded {
                    child = item.submenu.take();
                    item = item.submenu_marker(true);
                }
                let opens_submenu = item.submenu.is_some() || item.submenu_marker;
                let pointer: Option<PointerHandler> = if item.disabled {
                    // Not selectable, but still a place the pointer can be. Report
                    // "clear", so the row the pointer selected stops looking
                    // current instead of lingering.
                    let clear = ctx.clear.clone();
                    let level_path = path.to_vec();
                    Some(Rc::new(move |cx: &mut App| clear(cx, level_path.clone())))
                } else {
                    let hover = ctx.hover.clone();
                    let level_path = path.to_vec();
                    let row = id.clone();
                    Some(Rc::new(move |cx: &mut App| {
                        hover(cx, level_path.clone(), row.clone(), opens_submenu)
                    }) as PointerHandler)
                };
                let element = item
                    .active(is_active && !is_expanded)
                    .expanded(is_expanded)
                    .pointer_activate(pointer)
                    .into_any_element();

                // Record every row's painted bounds in this frame. The submenu
                // positioner for the next level runs after this row and reads it,
                // so first open, branch switches and layout changes place the
                // panel on their first frame instead of waiting for an input.
                let measure = row_bounds.clone();
                let measured_id = id.clone();
                let element = div()
                    .on_children_prepainted(move |children, _, _| {
                        if let Some(bounds) = children.first() {
                            measure.borrow_mut().insert(measured_id.clone(), *bounds);
                        }
                    })
                    .child(element)
                    .into_any_element();
                rows.push(element);
            }
            // Rows that cannot be chosen still have to report the pointer:
            // otherwise moving onto a header or a separator leaves the last
            // hovered row looking current, because nothing told the menu that
            // the pointer is now somewhere that is not a row.
            Entry::Separator => {
                let mut wrapper = filler(
                    ctx.clear.clone(),
                    path.to_vec(),
                    filler_id(&ctx.root_name, "sep", index),
                );
                wrapper = wrapper.child(MenuSeparator);
                rows.push(wrapper.into_any_element());
            }
            Entry::Header(header) => {
                let mut wrapper = filler(
                    ctx.clear.clone(),
                    path.to_vec(),
                    filler_id(&ctx.root_name, "header", index),
                );
                wrapper = wrapper.child(header);
                rows.push(wrapper.into_any_element());
            }
            Entry::Custom(element) => rows.push(element),
        }
    }

    (rows, child, row_bounds)
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

/// A Fluent 2 menu panel.
///
/// The id is required because the current row has to survive redraws. A menu may
/// carry nested menus through [`MenuItem::submenu`]; the root menu owns the open
/// path, every level's current row and the pointer/keyboard routing for the whole
/// tree. Submenus are content, not separate stateful menu views.
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
    /// Window-space bounds of every panel in the chain, published each frame for
    /// the hosting popover's outside-press test. A submenu panel extends beyond
    /// the popover's own bounds, so the bounds of the popover alone cannot tell
    /// "inside this menu" from "outside the overlay".
    interior: Option<InteriorRegions>,
    a11y_label: Option<SharedString>,
    style: StyleRefinement,
}

impl Menu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            focus: None,
            reveal: None,
            interior: None,
            a11y_label: None,
            style: StyleRefinement::default(),
        }
    }

    /// Accessible name for the menu surface.
    ///
    /// Defaults to “Menu”; callers with a meaningful trigger (“Sort”, “File”)
    /// should pass it so assistive technology announces the menu's purpose.
    /// MenuDropdown forwards its own label when one is supplied.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.a11y_label = Some(label.into());
        self
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

    /// Shares the chain's painted panel bounds with the hosting popover.
    pub(crate) fn interior_regions(mut self, regions: InteriorRegions) -> Self {
        self.interior = Some(regions);
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
        let Menu {
            id,
            entries,
            focus,
            reveal,
            interior,
            a11y_label,
            style,
        } = self;
        let theme = FluentTheme::global(cx);
        let surface = menu::surface(&theme);
        let shadow = feedback::tooltip(&theme).shadow.to_vec();
        let menu_id = SharedString::from(format!("{:?}", id));

        let levels = Rc::new({
            let mut levels = Vec::new();
            snapshot_levels(&entries, &[], &mut levels);
            levels
        });

        // Whole-tree navigation state: the root row plus the open submenu path.
        // Stored by row identity, so inserting or removing rows keeps it valid.
        let tree = window.use_keyed_state(
            SharedString::from(format!("menu-tree-{}", menu_id)),
            cx,
            |_, _| MenuState::default(),
        );

        let state = render::prepare_state(&tree, &levels, reveal, window, cx);

        // The regions the whole menu chain occupies, shared with the hosting
        // popover so a press inside any open submenu is not treated as "outside".
        let regions: InteriorRegions = interior.unwrap_or_default();
        {
            let mut regions = regions.borrow_mut();
            regions.clear();
            regions.resize(state.open.len() + 1, None);
        }

        let typeahead = window.use_keyed_state(
            SharedString::from(format!("menu-typeahead-{}", menu_id)),
            cx,
            |_, _| Typeahead::default(),
        );

        // Platform input methods have no text field here: the menu container
        // owns focus. Register an input handler for it so committed CJK text
        // reaches the same matcher as printable keys.
        let ime_sink_state = window.use_keyed_state(
            SharedString::from(format!("menu-ime-{}", menu_id)),
            cx,
            |_, cx| cx.new(|_| ime::MenuImeSink::default()),
        );
        // `use_keyed_state` stores the entity handle itself; unwrap it once so
        // the rest of the frame talks to the sink, not Entity<Entity<_>>.
        let ime_sink = ime_sink_state.read(cx).clone();
        {
            let tree = tree.clone();
            let levels = levels.clone();
            let typeahead = typeahead.clone();
            let ime_sink = ime_sink.clone();
            let generation = (state.session, state.level_epoch);
            ime_sink.update(cx, |sink, _| {
                sink.set_handler(
                    Rc::new(move |text, _window, cx| {
                        apply_typeahead(&tree, &levels, &typeahead, text, cx);
                    }),
                    generation,
                );
            });
        }

        let hover: LevelHover = {
            let tree = tree.clone();
            let levels = levels.clone();
            Rc::new(move |cx, path, row, opens| {
                pointer_hover(&tree, &levels, cx, &path, &row, opens)
            })
        };
        let clear: LevelClear = {
            let tree = tree.clone();
            let levels = levels.clone();
            Rc::new(move |cx, path| clear_pointer_level(&tree, &levels, cx, &path))
        };
        let leave: PanelLeave = {
            let tree = tree.clone();
            let levels = levels.clone();
            let regions = regions.clone();
            let typeahead = typeahead.clone();
            Rc::new(move |window, cx, path| {
                leave_panel(&tree, &levels, &regions, &typeahead, &path, window, cx)
            })
        };
        let moved: PanelMove = {
            let tree = tree.clone();
            let levels = levels.clone();
            Rc::new(move |cx, path, row_bounds, position| {
                pointer_moved_in_level(&tree, &levels, cx, &path, &row_bounds, position)
            })
        };

        let ctx = PanelContext {
            root_name: menu_id.clone(),
            theme,
            shadow: shadow.clone(),
            regions: regions.clone(),
            hover: hover.clone(),
            clear: clear.clone(),
            leave: leave.clone(),
            moved: moved.clone(),
        };

        let (children, root_child, root_row_bounds) = build_rows(entries, 0, &[], &state, &ctx);

        let mut container = div()
            .id(id.clone())
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
            .aria_label(a11y_label.unwrap_or_else(|| SharedString::from("Menu")))
            .when_some(focus.clone(), |el, focus| el.track_focus(&focus))
            .on_hover({
                let leave = leave.clone();
                move |hovered, window, cx| {
                    if !*hovered {
                        leave(window, cx, Vec::new());
                    }
                }
            })
            .on_mouse_move({
                let moved = moved.clone();
                let row_bounds = root_row_bounds.clone();
                move |event, _, cx| {
                    if event.pressed_button.is_none() && !cx.has_active_drag() {
                        moved(cx, Vec::new(), row_bounds.clone(), event.position);
                    }
                }
            })
            .refine_style(&style)
            .children(children);

        container = render::input_handlers(container, &tree, &levels, &typeahead, &ime_sink);

        // The first submenu hangs off the row that opened it. Its panel is built
        // from the same state the keyboard and pointer read, and placed by the
        // same-frame positioner below.
        if let Some(child) = root_child
            && !state.open.is_empty()
        {
            let anchor = state.open[0].parent.clone();
            let child_path = vec![anchor.clone()];
            let panel = build_submenu_panel(*child, 1, child_path, &state, &ctx);
            container = container.child(SubmenuPositioner {
                panel,
                row_bounds: root_row_bounds.clone(),
                anchor,
                depth: 1,
                regions: regions.clone(),
            });
        }

        let container: AnyElement = match focus.as_ref() {
            Some(focus) => ime::MenuImeElement {
                child: container.into_any_element(),
                focus: focus.clone(),
                sink: ime_sink,
            }
            .into_any_element(),
            None => container.into_any_element(),
        };

        // Publish this panel's window bounds so pointer leave from a submenu can
        // tell "moved back onto the root panel" from "left the menu chain".
        let root_regions = regions.clone();
        let container = div()
            .on_children_prepainted(move |children, _, _| {
                if let Some(bounds) = children.first() {
                    let mut regions = root_regions.borrow_mut();
                    if !regions.is_empty() {
                        regions[0] = Some(*bounds);
                    }
                }
            })
            .child(container);

        render::reveal(container, reveal, &menu_id, window, cx)
    }
}

/// Pointer movement over a selectable row of the level identified by `path`.
///
/// The pointer writes the level it is actually on, then truncates the branches
/// below it unless the row is the one already anchoring them. Hovering the row
/// that opened an open submenu therefore keeps that branch, while moving to a
/// sibling closes it — and Enter always agrees with the row that looks current.
fn pointer_hover(
    tree: &MenuTree,
    levels: &[LevelSnapshot],
    cx: &mut App,
    path: &[ElementId],
    row: &ElementId,
    opens: bool,
) {
    let (mut state, mut changed) = read_normalized(tree, cx, levels);
    if path.len() > state.open.len()
        || !path
            .iter()
            .zip(&state.open)
            .all(|(id, level)| id == &level.parent)
    {
        // A pointer report from a level that was closed meanwhile is stale.
        commit(tree, cx, &state, changed);
        return;
    }
    let depth = path.len();
    let anchors_open = state.open.get(depth).map(|level| &level.parent) == Some(row);
    if depth < state.open.len() && !anchors_open {
        state.open.truncate(depth);
        bump_level_epoch(&mut state);
        changed = true;
    }
    changed |= set_level_active(&mut state, levels, path, Some(row.clone()), true);

    let already_pending = state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.path == path && &pending.row == row);
    let should_pend = opens && !anchors_open && depth == state.open.len();
    if should_pend {
        if !already_pending {
            let now = cx.background_executor().now();
            state.pending = Some(PendingOpen {
                path: path.to_vec(),
                row: row.clone(),
                deadline: now + fluent_core::spec::motion::DURATION_MENU_HOVER_OPEN,
            });
            changed = true;
        }
    } else if state.pending.take().is_some() {
        changed = true;
    }
    commit(tree, cx, &state, changed);
}

/// Pointer movement onto a row that cannot be chosen (disabled item, header,
/// separator) in the level identified by `path`.
fn clear_pointer_level(
    tree: &MenuTree,
    levels: &[LevelSnapshot],
    cx: &mut App,
    path: &[ElementId],
) {
    let (mut state, mut changed) = read_normalized(tree, cx, levels);
    if path.len() > state.open.len()
        || !path
            .iter()
            .zip(&state.open)
            .all(|(id, level)| id == &level.parent)
    {
        commit(tree, cx, &state, changed);
        return;
    }
    let depth = path.len();
    if depth < state.open.len() {
        state.open.truncate(depth);
        bump_level_epoch(&mut state);
        changed = true;
    }
    if depth == 0 {
        if state.root.as_ref().is_some_and(|row| row.from_pointer) {
            state.root = None;
            changed = true;
        }
    } else if let Some(level) = state.open.get_mut(depth - 1)
        && level.active.as_ref().is_some_and(|row| row.from_pointer)
    {
        level.active = None;
        changed = true;
    }
    if state.pending.take().is_some() {
        changed = true;
    }
    commit(tree, cx, &state, changed);
}

/// The pointer left one panel. A pending request that belongs to this panel's
/// level is stale as soon as the pointer leaves it, even when another panel of
/// the chain is still under the pointer. Only a move outside the whole chain
/// clears the pointer-owned current rows and any remaining hover; moving between
/// panels of the chain is a legal move and must not cancel an open branch.
fn leave_panel(
    tree: &MenuTree,
    levels: &[LevelSnapshot],
    regions: &InteriorRegions,
    typeahead: &gpui::Entity<Typeahead>,
    path: &[ElementId],
    window: &mut Window,
    cx: &mut App,
) {
    let position = window.mouse_position();
    let outside = !regions
        .borrow()
        .iter()
        .flatten()
        .any(|bounds| bounds.contains(&position));

    let (mut state, mut changed) = read_normalized(tree, cx, levels);
    if state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.path == path)
    {
        state.pending = None;
        changed = true;
    }
    if outside {
        if state.pending.take().is_some() {
            changed = true;
        }
        if state.root.as_ref().is_some_and(|row| row.from_pointer) {
            state.root = None;
            changed = true;
        }
        for level in &mut state.open {
            if level.active.as_ref().is_some_and(|row| row.from_pointer) {
                level.active = None;
                changed = true;
            }
        }
        // Leaving the menu ends the typing session: returning must not inherit a
        // prefix typed before the leave even if the 600ms window has not elapsed.
        reset_typeahead(typeahead, cx);
    }
    commit(tree, cx, &state, changed);
}

/// An actual pointer move inside one panel. Mouse movement is the only thing
/// that can cancel a hover request: layout changes under a stationary pointer
/// must not take over the keyboard's current row.
///
/// The request is cancelled when the pointer is no longer inside the measured
/// bounds of the row that asked for it — including panel padding, row gaps and
/// custom non-selectable content, which have no row callback of their own. Only
/// the panel that owns the request's path may cancel it; ancestor panels also
/// receive the bubbled event and must not clear a deeper level's request.
fn pointer_moved_in_level(
    tree: &MenuTree,
    levels: &[LevelSnapshot],
    cx: &mut App,
    path: &[ElementId],
    row_bounds: &RowBounds,
    position: Point<Pixels>,
) {
    let (mut state, mut changed) = read_normalized(tree, cx, levels);
    let Some(pending) = state.pending.clone() else {
        commit(tree, cx, &state, changed);
        return;
    };
    if pending.path != path {
        commit(tree, cx, &state, changed);
        return;
    }
    let inside = row_bounds
        .borrow()
        .get(&pending.row)
        .copied()
        .is_some_and(|bounds| bounds.contains(&position));
    if !inside {
        state.pending = None;
        changed = true;
    }
    commit(tree, cx, &state, changed);
}

/// Builds one submenu panel, recursing into the levels below it.
fn build_submenu_panel(
    menu: Menu,
    depth: usize,
    path: Vec<ElementId>,
    state: &MenuState,
    ctx: &PanelContext,
) -> AnyElement {
    let Menu {
        id: menu_id,
        entries,
        style,
        ..
    } = menu;
    let (rows, child, row_bounds) = build_rows(entries, depth, &path, state, ctx);
    let surface = menu::surface(&ctx.theme);
    // Identity follows the full path, so switching to a sibling branch gets a
    // fresh panel even if both submenus use the same `Menu` id and row ids.
    let panel_id = ElementId::from(SharedString::from(format!(
        "{}-submenu-{}-{:?}-{:?}",
        ctx.root_name, depth, path, menu_id
    )));

    // Each panel owns its rows and its own `role=menu`, but only the root panel
    // is a focus scope: the keyboard model stays on the root container.
    let mut container = div()
        .id(panel_id)
        // Painting above the page is not enough to stop a press from reaching
        // the control behind it; the panel needs an occluding hitbox too.
        .occlude()
        .flex()
        .flex_col()
        .gap(px(2.))
        .p(surface.padding)
        .min_w(surface.min_width)
        .rounded(surface.radius)
        .bg(surface.background)
        .text_color(surface.foreground)
        .shadow(ctx.shadow.clone())
        .role(Role::Menu)
        .on_hover({
            let leave = ctx.leave.clone();
            let path = path.clone();
            move |hovered, window, cx| {
                if !*hovered {
                    leave(window, cx, path.clone());
                }
            }
        })
        .on_mouse_move({
            let moved = ctx.moved.clone();
            let row_bounds = row_bounds.clone();
            let path = path.clone();
            move |event, _, cx| {
                if event.pressed_button.is_none() && !cx.has_active_drag() {
                    moved(cx, path.clone(), row_bounds.clone(), event.position);
                }
            }
        })
        .refine_style(&style)
        .children(rows);

    if let Some(child) = child
        && depth < state.open.len()
    {
        let anchor = state.open[depth].parent.clone();
        let mut child_path = path.clone();
        child_path.push(anchor.clone());
        let panel = build_submenu_panel(*child, depth + 1, child_path, state, ctx);
        container = container.child(SubmenuPositioner {
            panel,
            row_bounds,
            anchor,
            depth: depth + 1,
            regions: ctx.regions.clone(),
        });
    }

    container.into_any_element()
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

/// Wraps a non-selectable row so the pointer reports over it, in its own level.
fn filler(clear: LevelClear, path: Vec<ElementId>, id: SharedString) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .on_mouse_move(move |event, _, cx| {
            if event.pressed_button.is_none() && !cx.has_active_drag() {
                clear(cx, path.clone());
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
/// does; each open level keeps its own current row, and the keyboard always acts
/// on the deepest one.
///
/// Submenus up to any depth are supported. Tab closes the whole chain, Escape and
/// Left step one level back, and a press on any submenu panel counts as inside
/// the dropdown. Still missing: exposing a disabled state through accesskit (this
/// GPUI version has no API for it).
#[derive(IntoElement)]
pub struct MenuDropdown {
    id: ElementId,
    trigger: Option<AnyElement>,
    menu: Menu,
    open: Option<bool>,
    default_open: bool,
    focus: Option<gpui::FocusHandle>,
    on_open_change: Option<OpenChangeHandler>,
    a11y_label: Option<SharedString>,
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
            a11y_label: None,
        }
    }

    /// Accessible name for the menu surface; overrides `Menu::accessibility_label`.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.a11y_label = Some(label.into());
        self
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
        // Each open bumps the generation so the reveal replays and the menu starts
        // a new session (old submenu paths and hover requests must not revive).
        // Detect the rising edge here rather than in `on_open_change`: a
        // controlled caller changes `open` directly and never calls the handler.
        let open_store = window.use_keyed_state(
            SharedString::from(format!("menu-dropdown-open-{:?}", self.id)),
            cx,
            |_, _| self.default_open,
        );
        let was_open = window.use_keyed_state(
            SharedString::from(format!("menu-dropdown-was-open-{:?}", self.id)),
            cx,
            |_, _| false,
        );
        let generation = window.use_keyed_state(
            SharedString::from(format!("menu-reveal-generation-{:?}", self.id)),
            cx,
            |_, _| 0_u64,
        );
        let effective_open = self.open.unwrap_or_else(|| *open_store.read(cx));
        if effective_open && !*was_open.read(cx) {
            generation.update(cx, |value, _| *value += 1);
            was_open.update(cx, |slot, _| *slot = true);
        } else if !effective_open && *was_open.read(cx) {
            was_open.update(cx, |slot, _| *slot = false);
        }
        let generation_value = *generation.read(cx);

        let interior: InteriorRegions = InteriorRegions::default();
        let mut menu = self
            .menu
            .reveal_generation(generation_value)
            .interior_regions(interior.clone());
        if let Some(label) = self.a11y_label.as_ref() {
            menu = menu.accessibility_label(label.clone());
        }
        if let Some(focus) = self.focus.as_ref() {
            menu = menu.track_focus(focus);
        }

        let mut popover = crate::Popover::new(self.id)
            .default_open(self.default_open)
            // The menu IS the surface: a panel chrome around it would stack a
            // second card and inset every row.
            .draw_panel_surface(false)
            .interior_regions(interior)
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
        match self.on_open_change {
            Some(handler) => {
                let store = open_store.clone();
                popover = popover.on_open_change(move |next, window, cx| {
                    store.update(cx, |slot, _| *slot = next);
                    handler(next, window, cx);
                });
            }
            None => {
                let store = open_store.clone();
                popover = popover.on_open_change(move |next, _, cx| {
                    store.update(cx, |slot, _| *slot = next);
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
    /// Styled as the path to an open submenu. This is deliberately distinct from
    /// `active`: a parent row is a path hint, not a second activation target.
    expanded: bool,
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
            expanded: false,
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
    /// The child is content, not a callback: the parent menu owns the whole tree
    /// and walks its rows to navigate, activate and close it. Nesting can be
    /// arbitrary: every level of an open path is rendered, and a row inside a
    /// submenu may open another level. Row ids only have to be unique within
    /// their own menu; the same id may appear at different levels, including two
    /// adjacent opener rows, because opening is resolved by the full level path.
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

    /// Marks this row as the path to an open submenu. Set by [`Menu`].
    pub(crate) fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
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
        // An expanded parent row uses a separate path-hint fill, so the deepest
        // current row is unambiguous.
        let background = if self.expanded && !self.disabled {
            menu::path_hint_background(&spec)
        } else {
            menu::row_background(&spec, self.active && !self.disabled)
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
            .aria_label(label.clone())
            .aria_selected(self.active && !self.disabled)
            .when(self.active && !self.disabled, |el| {
                // Focus stays on the menu container; this child is the active
                // descendant assistive technology should announce.
                el.aria_active_descendant()
            })
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

mod ime;
mod render;

#[cfg(test)]
mod ax_tests;

#[cfg(test)]
mod ime_tests;

#[cfg(test)]
mod cjk_consumer_tests;

#[cfg(test)]
mod keyboard_tests;

#[cfg(test)]
mod input_mode_tests;

#[cfg(test)]
mod typeahead_tests;

#[cfg(test)]
mod tab_handoff_tests;

#[cfg(test)]
mod surface_tests;

#[cfg(test)]
mod reveal_tests;

#[cfg(test)]
mod submenu_tests;

#[cfg(test)]
mod submenu_regression_tests;
