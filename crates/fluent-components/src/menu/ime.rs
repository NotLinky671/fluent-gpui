//! IME input for menu typeahead.
//!
//! A menu is a focusable container, not a text field, so the platform has no
//! `EntityInputHandler` for it and committed CJK text would be dropped. The
//! wrapper below registers one for the focused menu container. Composition is
//! held locally; only the committed replacement text reaches the typeahead
//! matcher, so there is no pinyin special-casing and no hidden editable field.

use super::MenuImeCommit;
use gpui::{
    AnyElement, App, Bounds, Context, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels,
    UTF16Selection, Window,
};
use std::ops::Range;

#[derive(Default)]
pub(super) struct MenuImeSink {
    handler: Option<MenuImeCommit>,
    /// Session + level visit this handler was installed for. Composition is
    /// dropped when either changes (close, reopen, child-menu switch).
    generation: Option<(Option<u64>, u64)>,
    /// Text the input method is currently composing. Kept only so the platform
    /// can query the marked range; it never drives menu matches.
    marked: String,
    /// Selection reported by the input method within `marked` (UTF-16 offsets).
    marked_selection: Option<Range<usize>>,
    /// A printable key already applied through KeyDown. The platform follows a
    /// key with `insertText`; that duplicate commit must not run typeahead a
    /// second time.
    last_key: Option<String>,
}

impl MenuImeSink {
    pub(super) fn set_handler(&mut self, handler: MenuImeCommit, generation: (Option<u64>, u64)) {
        if self.generation != Some(generation) {
            self.marked.clear();
            self.marked_selection = None;
            self.last_key = None;
        }
        self.generation = Some(generation);
        self.handler = Some(handler);
    }

    /// Called by the KeyDown typeahead path before it applies `text`.
    pub(super) fn note_key(&mut self, text: &str) {
        self.last_key = Some(text.to_string());
    }

    /// Clears the marker on the next platform dispatch. A key-only path that
    /// never produces `insertText` must not swallow a later independent commit
    /// of the same character.
    pub(super) fn clear_key(&mut self, text: &str) {
        if self.last_key.as_deref() == Some(text) {
            self.last_key = None;
        }
    }

    pub(super) fn has_marked(&self) -> bool {
        !self.marked.is_empty()
    }

    fn commit(&mut self, text: &str, window: &mut Window, cx: &mut App) {
        self.marked.clear();
        self.marked_selection = None;
        if self.last_key.take().as_deref() == Some(text) {
            return;
        }
        if let Some(handler) = self.handler.as_ref() {
            handler(text, window, cx);
        }
    }
}

impl EntityInputHandler for MenuImeSink {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        if self.marked.is_empty() {
            return None;
        }
        let utf16: Vec<u16> = self.marked.encode_utf16().collect();
        let end = range_utf16.end.min(utf16.len());
        let start = range_utf16.start.min(end);
        adjusted_range.replace(start..end);
        String::from_utf16(&utf16[start..end]).ok()
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        // This is not an editable document; the only selection the platform can
        // ask about is the input method's own marked range.
        self.marked_selection.clone().map(|range| UTF16Selection {
            range,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        (!self.marked.is_empty()).then(|| 0..self.marked.encode_utf16().count())
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked.clear();
        self.marked_selection = None;
    }

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit(text, window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // Composition preview must not move the menu's current row. Keep the
        // platform's selected range so candidate-window queries see the real
        // composition selection, not an invented one.
        self.marked = new_text.to_string();
        self.marked_selection = new_selected_range;
    }

    fn bounds_for_range(
        &mut self,
        _range: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(element_bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

/// Registers the menu's IME sink while painting the focused container. The
/// element is purely a pass-through for layout/prepaint/paint otherwise.
pub(super) struct MenuImeElement {
    pub(super) child: AnyElement,
    pub(super) focus: gpui::FocusHandle,
    pub(super) sink: Entity<MenuImeSink>,
}

impl Element for MenuImeElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn a11y_role(&self) -> Option<gpui::Role> {
        self.child.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        self.child.write_a11y_info(node)
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let _ = self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _state: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.handle_input(
            &self.focus,
            ElementInputHandler::new(bounds, self.sink.clone()),
            cx,
        );
        self.child.paint(window, cx)
    }
}

impl IntoElement for MenuImeElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}
