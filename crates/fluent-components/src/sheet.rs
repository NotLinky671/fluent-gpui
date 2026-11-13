//! Fluent 2 `Sheet`: an overlay drawer anchored to one window edge.
//!
//! `gpui-base` owns the modal mechanics: a full-viewport host that registers a
//! focus trap, answers Escape, and turns an overlay press into
//! `request_close` then `on_close`. Its host has no open/closed notion and no
//! motion, so this wrapper adds:
//!
//! - controlled/uncontrolled open state in window keyed state;
//! - a [`gpui_base::motion::Presence`] driven slide, so the panel and scrim
//!   animate in and out and stay mounted until the exit finishes; the edge is
//!   latched while open so an exit keeps the edge it entered from even if the
//!   caller clears its side state as soon as it requests a close;
//! - the focus handover `Dialog` uses: opening records the previously focused
//!   handle and moves focus into the host, and closing returns it **after** the
//!   exit finishes and only while focus is still inside.
//!
//! The surface and scrim come from [`fluent_core::spec::sheet`], which shares
//! the dialog's `colorBackgroundOverlay` scrim and neutral panel tokens.
//!
//! # Limitations
//!
//! - Escape and an overlay press both report `on_open_change(false)`; the base
//!   host does not distinguish them.
//! - Closing restores focus only when the sheet was allowed to finish its exit.
//!   A caller that unmounts the element in the same frame that it sets
//!   `open(false)` never gets a handover — keep it mounted and drive it with
//!   `open`, the same contract as `Dialog`.
//! - The panel is fixed-size and its content is clipped; there is no scroll
//!   region or drag-to-dismiss.

use fluent_core::gpui_compat::{
    AnyElement, App, ElementId, FocusHandle, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled,
    Window, deferred, div, px,
};
use fluent_core::spec;
use fluent_core::spec::sheet::SheetSide;
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui_base::Sheet as BaseSheet;
use gpui_base::motion::{Presence, TransitionId};

/// Called with the requested open state whenever the sheet asks to close.
pub type SheetOpenChange = std::rc::Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// Shared builder state for [`Sheet`].
struct SheetConfig {
    id: ElementId,
    side: SheetSide,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<SheetOpenChange>,
    size: Option<Pixels>,
    title: Option<SharedString>,
    title_element: Option<AnyElement>,
    a11y_label: Option<SharedString>,
    children: Vec<AnyElement>,
    footer: Vec<AnyElement>,
}

/// A Fluent overlay drawer: a modal panel sliding in from one window edge.
///
/// The element stays mounted while it animates out, but renders nothing once
/// the exit finishes. Mount it for the lifetime of the page and drive it with
/// [`Sheet::open`] (or let it keep its own state with [`Sheet::default_open`]).
#[derive(IntoElement)]
pub struct Sheet {
    config: SheetConfig,
}

impl Sheet {
    /// A sheet keyed by `id`: uncontrolled state lives in window keyed state,
    /// so the identity has to stay stable across redraws.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            config: SheetConfig {
                id: id.into(),
                side: SheetSide::default(),
                open: None,
                default_open: false,
                on_open_change: None,
                size: None,
                title: None,
                title_element: None,
                a11y_label: None,
                children: Vec::new(),
                footer: Vec::new(),
            },
        }
    }

    /// The edge the sheet slides in from. Defaults to [`SheetSide::Right`].
    pub fn side(mut self, side: SheetSide) -> Self {
        self.config.side = side;
        self
    }

    /// Controlled open state: the caller owns it.
    ///
    /// A close request still fires [`Self::on_open_change`]; the sheet stays
    /// open until the caller passes `open(false)`.
    pub fn open(mut self, open: bool) -> Self {
        self.config.open = Some(open);
        self
    }

    /// Uncontrolled initial state, kept in window keyed state.
    ///
    /// A redraw never resets it back to the default.
    pub fn default_open(mut self, open: bool) -> Self {
        self.config.default_open = open;
        self
    }

    /// Fires when Escape or an overlay press asks the sheet to close.
    ///
    /// The base host does not report which path triggered the request, so the
    /// value is always `false` and there is no reason argument.
    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.config.on_open_change = Some(std::rc::Rc::new(handler));
        self
    }

    /// Width (left/right) or height (top/bottom) of the panel.
    ///
    /// Defaults to the spec's 320px drawer. The value is also the slide
    /// distance, so a custom extent does not change the motion's feel.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.config.size = Some(size.into());
        self
    }

    /// The title text rendered at the top of the panel.
    ///
    /// The same text becomes the panel's accessible name. Use
    /// [`Self::title_element`] for custom content, or
    /// [`Self::accessibility_label`] when the announced name must differ.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.config.title = Some(title.into());
        self
    }

    /// A custom title element; it does not supply an accessible name.
    pub fn title_element(mut self, title: impl IntoElement) -> Self {
        self.config.title_element = Some(title.into_any_element());
        self
    }

    /// Overrides the accessible name derived from the title text.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.config.a11y_label = Some(label.into());
        self
    }

    /// Footer actions: buttons and labels supplied by the caller.
    pub fn footer(mut self, children: impl IntoIterator<Item = AnyElement>) -> Self {
        self.config.footer.extend(children);
        self
    }

    /// Adds one footer child.
    pub fn footer_child(mut self, child: impl IntoElement) -> Self {
        self.config.footer.push(child.into_any_element());
        self
    }
}

impl ParentElement for Sheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.config.children.extend(elements);
    }
}

impl RenderOnce for Sheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_sheet(self.config, window, cx)
    }
}

fn render_sheet(mut config: SheetConfig, window: &mut Window, cx: &mut App) -> AnyElement {
    let theme = FluentTheme::global(cx);
    let surface_spec = spec::sheet::sheet(&theme);
    let size = config.size.unwrap_or(surface_spec.size);
    let id = config.id;

    let store = window.use_keyed_state(
        SharedString::from(format!("sheet-open-{id:?}")),
        cx,
        |_, _| config.default_open,
    );
    let is_open = config.open.unwrap_or_else(|| *store.read(cx));
    let controlled = config.open.is_some();

    let was_rendered = window.use_keyed_state(
        SharedString::from(format!("sheet-was-rendered-{id:?}")),
        cx,
        |_, _| false,
    );
    let rendered_before = *was_rendered.read(cx);
    // A sheet that has never been open must not sample a presence: the first
    // sample of an all-zero transition reports `Exiting`/`Running` for one
    // duration, which would mount an invisible host, trap focus and unmount a
    // moment later.
    if !is_open && !rendered_before {
        return div().into_any_element();
    }

    // The exit has to use the edge the panel actually entered from. Callers
    // commonly clear their side state (or fall back to the default) as soon as
    // they request a close; reading the prop again during the exit would
    // teleport the panel to the other edge while it slides out.
    let side_state = window.use_keyed_state(
        SharedString::from(format!("sheet-side-{id:?}")),
        cx,
        |_, _| config.side,
    );
    if is_open && *side_state.read(cx) != config.side {
        side_state.update(cx, |side, _| *side = config.side);
    }
    let side = *side_state.read(cx);

    let focus = window.use_keyed_state(
        SharedString::from(format!("sheet-focus-{id:?}")),
        cx,
        |_, cx| cx.focus_handle(),
    );
    let previous = window.use_keyed_state(
        SharedString::from(format!("sheet-previous-focus-{id:?}")),
        cx,
        |_, _| None::<FocusHandle>,
    );

    // `Presence` owns the timeline. It keeps requesting animation frames while
    // entering or exiting, collapses to a stable sample under reduce-motion,
    // and shortens a reversal so a close-while-opening does not jump.
    let presence = Presence::new(
        TransitionId::from(ElementId::Name(SharedString::from(format!(
            "sheet-presence-{id:?}"
        )))),
        is_open,
    )
    .transition(spec::motion::sheet_policy());
    let sample = presence.sample(window, cx);
    let rendering = sample.should_render();
    let progress = sample.progress;

    let panel_focus = focus.read(cx).clone();
    if rendering && !rendered_before {
        let current = window.focused(cx);
        previous.update(cx, |slot, _| *slot = current);
        panel_focus.focus(window, cx);
        was_rendered.update(cx, |value, _| *value = true);
    } else if !rendering && rendered_before {
        // The exit has finished: hand focus back only if it never left the
        // sheet, so a pointer interaction elsewhere is not undone.
        let restore = previous.update(cx, |slot, _| slot.take());
        if panel_focus.contains_focused(window, cx)
            && let Some(handle) = restore
        {
            handle.focus(window, cx);
        }
        was_rendered.update(cx, |value, _| *value = false);
    }

    if !rendering {
        return div().into_any_element();
    }

    let offset = px(-size.as_f32() * (1.0 - progress));
    let scrim = div()
        .absolute()
        .size_full()
        // Painting over the page is not enough: without an occluding hitbox the
        // buttons underneath still receive hover edges through the scrim.
        .occlude()
        .bg(surface_spec.scrim.opacity(progress));

    let title_element = config.title_element.take().or_else(|| {
        config
            .title
            .clone()
            .map(|title| div().child(title).into_any_element())
    });
    let header = title_element.map(|title| {
        div()
            .w_full()
            .flex()
            .items_center()
            .text_size(surface_spec.title_size)
            .line_height(surface_spec.title_line_height)
            .font_weight(FontWeight::SEMIBOLD)
            .child(title)
    });
    let a11y_label = config.a11y_label.clone().or_else(|| config.title.clone());

    let body = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.))
        .text_size(surface_spec.body_size)
        .line_height(surface_spec.body_line_height)
        .children(config.children);

    let footer = (!config.footer.is_empty()).then(|| {
        div()
            .w_full()
            .flex()
            .flex_wrap()
            .justify_end()
            .gap(surface_spec.footer_gap)
            .children(config.footer)
    });

    let surface = div()
        .id(ElementId::Name(SharedString::from(format!(
            "sheet-surface-{id:?}"
        ))))
        .absolute()
        // Empty areas of the panel must not fall through to the base host's
        // overlay layer, which would read them as a dismiss press.
        .occlude()
        .overflow_hidden()
        // A 320px panel in a narrower/shorter window must not hang off the
        // opposite edge; the slide still starts at the nominal extent.
        .max_w_full()
        .max_h_full()
        .flex()
        .flex_col()
        .gap(surface_spec.content_gap)
        .p(surface_spec.padding)
        .bg(surface_spec.background)
        .text_color(surface_spec.foreground)
        .shadow(surface_spec.shadow.to_vec())
        .role(Role::Dialog)
        .aria_modal(true)
        .when_some(a11y_label, |this, label| this.aria_label(label))
        .children(header)
        .child(body)
        .children(footer);

    // Anchor to the requested edge, spanning the other axis. The offset moves
    // the panel out by its own extent, so `progress == 0` is fully offscreen and
    // `progress == 1` is flush with the window edge.
    let surface = match side {
        SheetSide::Right => surface.top(px(0.)).h_full().w(size).right(offset),
        SheetSide::Left => surface.top(px(0.)).h_full().w(size).left(offset),
        SheetSide::Top => surface.left(px(0.)).w_full().h(size).top(offset),
        SheetSide::Bottom => surface.left(px(0.)).w_full().h(size).bottom(offset),
    };
    // Round only the corners facing the page.
    let surface = match side {
        SheetSide::Right => surface
            .rounded_tl(surface_spec.radius)
            .rounded_bl(surface_spec.radius),
        SheetSide::Left => surface
            .rounded_tr(surface_spec.radius)
            .rounded_br(surface_spec.radius),
        SheetSide::Top => surface
            .rounded_bl(surface_spec.radius)
            .rounded_br(surface_spec.radius),
        SheetSide::Bottom => surface
            .rounded_tl(surface_spec.radius)
            .rounded_tr(surface_spec.radius),
    };

    let request_close = {
        let store = store.clone();
        let on_change = config.on_open_change.clone();
        move |window: &mut Window, cx: &mut App| {
            if let Some(handler) = on_change.as_ref() {
                handler(false, window, cx);
            }
            if !controlled {
                store.update(cx, |state, cx| {
                    *state = false;
                    cx.notify();
                });
            }
        }
    };

    // The base host is window-anchored but not deferred. Deferring it mirrors
    // `Dialog`: the modal layer paints after every ancestor, so a sheet mounted
    // inside a scrolling or clipped page still covers the whole window.
    //
    // The zero-height wrapper is load-bearing: a deferred host is absolute, so
    // without it the open state would drop the flex item (and its gap) and the
    // content below the sheet would jump. The closed state renders the same
    // zero-height slot, keeping the flow layout identical either way.
    div()
        .h(px(0.))
        .child(deferred(
            BaseSheet::new(cx)
                .focus_handle(panel_focus)
                .request_close(request_close)
                .overlay(scrim)
                .surface(surface),
        ))
        .into_any_element()
}

#[cfg(test)]
mod tests;
