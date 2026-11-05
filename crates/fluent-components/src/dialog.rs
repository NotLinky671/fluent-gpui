//! Fluent 2 `Dialog` and `AlertDialog`.
//!
//! `gpui-base` owns the modal host: it renders itself through `deferred` at a
//! raised priority, paints the caller-supplied scrim under a centred popup,
//! registers a focus trap around it and dispatches `Cancel` / `Confirm` /
//! backdrop presses through one ordering. The base keeps no focus memory,
//! though, so the wrapper below records the previously focused handle in
//! window keyed state and hands focus back when the dialog closes — the same
//! edge-triggered handover `Popover` already performs.
//!
//! `AlertDialog` shares the surface; the only differences are the
//! `Role::AlertDialog` semantics, no Escape binding and no backdrop dismissal,
//! so the user must pick one of the supplied actions.

use fluent_core::gpui_compat::{
    AnyElement, App, ClickEvent, ElementId, FocusHandle, FontWeight, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, px,
};
use fluent_core::spec;
use fluent_theme::FluentTheme;
use gpui::prelude::FluentBuilder as _;
use gpui_base::{DialogBackdrop, DialogChangeReason, DialogPopup};

/// Called with the new open state and the reason whenever the dialog requests
/// a change (confirm, cancel, escape, backdrop, or uncontrolled toggle).
pub type DialogOpenChange =
    std::rc::Rc<dyn Fn(bool, DialogChangeReason, &mut Window, &mut App) + 'static>;

/// A confirm/cancel decision: returning `false` vetoes the close.
type Decision = std::rc::Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>;

/// Runs after a close was accepted.
type Closed = std::rc::Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Shared builder state for [`Dialog`] and [`AlertDialog`].
struct DialogConfig {
    id: ElementId,
    title: Option<AnyElement>,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<DialogOpenChange>,
    on_ok: Option<Decision>,
    on_cancel: Option<Decision>,
    on_close: Option<Closed>,
    children: Vec<AnyElement>,
    footer: Vec<AnyElement>,
}

impl DialogConfig {
    fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            title: None,
            open: None,
            default_open: false,
            on_open_change: None,
            on_ok: None,
            on_cancel: None,
            on_close: None,
            children: Vec::new(),
            footer: Vec::new(),
        }
    }
}

macro_rules! dialog_builders {
    () => {
        /// The title rendered at the top of the card.
        pub fn title(mut self, title: impl IntoElement) -> Self {
            self.config.title = Some(title.into_any_element());
            self
        }

        /// Controlled open state: the caller owns it.
        ///
        /// A close request still fires [`Self::on_open_change`]; the dialog
        /// stays open until the caller passes `open(false)`.
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

        /// Fires on every requested or actual open change with its reason.
        pub fn on_open_change(
            mut self,
            handler: impl Fn(bool, DialogChangeReason, &mut Window, &mut App) + 'static,
        ) -> Self {
            self.config.on_open_change = Some(std::rc::Rc::new(handler));
            self
        }

        /// Confirm decision: return `false` to veto the close.
        pub fn on_ok(
            mut self,
            handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
        ) -> Self {
            self.config.on_ok = Some(std::rc::Rc::new(handler));
            self
        }

        /// Cancel decision (Escape, backdrop where allowed): `false` vetoes.
        pub fn on_cancel(
            mut self,
            handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
        ) -> Self {
            self.config.on_cancel = Some(std::rc::Rc::new(handler));
            self
        }

        /// Runs after a close was accepted, whatever path triggered it.
        pub fn on_close(
            mut self,
            handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        ) -> Self {
            self.config.on_close = Some(std::rc::Rc::new(handler));
            self
        }

        /// Footer actions: buttons and labels supplied by the caller.
        ///
        /// Each child lands in a right-aligned row under the body. A button
        /// that should close the dialog updates the app's own state (or relies
        /// on the uncontrolled store) from its `on_click`.
        pub fn footer(mut self, children: impl IntoIterator<Item = AnyElement>) -> Self {
            self.config.footer.extend(children);
            self
        }

        /// Adds one footer child.
        ///
        /// The component does not choose appearances for you. Fluent's dialog
        /// contract is a **bordered secondary** button for the dismissing
        /// action and a primary one for the confirming action; a borderless
        /// button reads as a third, less important choice and is not what the
        /// footer should look like.
        pub fn footer_child(mut self, child: impl IntoElement) -> Self {
            self.config.footer.push(child.into_any_element());
            self
        }
    };
}

/// A Fluent 2 modal dialog: scrim, centred card, title/body/footer.
///
/// The card renders itself deferred above the window content; it needs no
/// overlay host beyond `FluentRoot`'s tab-walk context for the trap. Opening
/// moves focus into the card, Tab/Shift+Tab cycle inside it, and closing
/// returns focus to whatever held it before. Escape and a backdrop press
/// request a `Cancel` close; both go through `on_cancel` first.
#[derive(IntoElement)]
pub struct Dialog {
    config: DialogConfig,
}

impl Dialog {
    /// A dialog keyed by `id`: uncontrolled state lives in window keyed
    /// state, so the identity has to stay stable across redraws.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            config: DialogConfig::new(id),
        }
    }

    dialog_builders!();
}

impl ParentElement for Dialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.config.children.extend(elements);
    }
}

impl RenderOnce for Dialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_dialog(self.config, false, window, cx)
    }
}

/// A Fluent 2 alert dialog: the same card, but a choice is mandatory.
///
/// There is no Escape path and no backdrop dismissal, and callers should not
/// place a close button in the card; the only way out is one of the supplied
/// footer actions.
#[derive(IntoElement)]
pub struct AlertDialog {
    config: DialogConfig,
}

impl AlertDialog {
    /// An alert dialog keyed by `id`; see [`Dialog::new`].
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            config: DialogConfig::new(id),
        }
    }

    dialog_builders!();
}

impl ParentElement for AlertDialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.config.children.extend(elements);
    }
}

impl RenderOnce for AlertDialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_dialog(self.config, true, window, cx)
    }
}

/// Builds the gpui-base host with the Fluent scrim and card.
///
/// Focus handover mirrors `Popover`: the first frame the dialog reports open
/// records `window.focused` and focuses the host handle; the first closed
/// frame gives it back only while focus is still inside the dialog, so a
/// click that moved focus elsewhere is not undone. All of it lives in window
/// keyed state because the wrapper is `RenderOnce`.
fn render_dialog(
    config: DialogConfig,
    alert: bool,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let id = config.id;
    let store = window.use_keyed_state(
        SharedString::from(format!("dialog-open-{id:?}")),
        cx,
        |_, _| config.default_open,
    );
    let is_open = config.open.unwrap_or_else(|| *store.read(cx));
    let controlled = config.open.is_some();
    let surface = spec::dialog::dialog(&FluentTheme::global(cx));

    // Focus handover, mirroring `Popover`: opening records the previous focus
    // and moves it into the host; closing returns it only while focus is still
    // inside, so a click that moved focus elsewhere is not undone.
    let focus = window.use_keyed_state(
        SharedString::from(format!("dialog-focus-{id:?}")),
        cx,
        |_, cx| cx.focus_handle(),
    );
    let previous = window.use_keyed_state(
        SharedString::from(format!("dialog-previous-focus-{id:?}")),
        cx,
        |_, _| None::<FocusHandle>,
    );
    let was_open = window.use_keyed_state(
        SharedString::from(format!("dialog-was-open-{id:?}")),
        cx,
        |_, _| false,
    );
    let panel_focus = focus.read(cx).clone();
    let previously_open = *was_open.read(cx);
    if is_open && !previously_open {
        let current = window.focused(cx);
        previous.update(cx, |slot, _| *slot = current);
        panel_focus.focus(window, cx);
        was_open.update(cx, |open, _| *open = true);
    } else if !is_open && previously_open {
        let restore = previous.update(cx, |slot, _| slot.take());
        if panel_focus.contains_focused(window, cx)
            && let Some(handle) = restore
        {
            handle.focus(window, cx);
        }
        was_open.update(cx, |open, _| *open = false);
    }

    // Report every request through the caller's callback first, then drive the
    // uncontrolled store; a controlled caller decides itself by re-rendering
    // with a new `open` value. Callers that need the base's imperative handle
    // can drive `gpui_base::Dialog` themselves; the wrapper exposes no handle.

    let report = {
        let store = store.clone();
        let on_change = config.on_open_change.clone();
        move |open: bool, reason: DialogChangeReason, window: &mut Window, cx: &mut App| {
            if let Some(handler) = on_change.as_ref() {
                handler(open, reason, window, cx);
            }
            if !controlled {
                store.update(cx, |state, cx| {
                    *state = open;
                    cx.notify();
                });
            }
        }
    };

    let backdrop = DialogBackdrop::new()
        .absolute()
        .size_full()
        .bg(surface.scrim);
    let popup = DialogPopup::new()
        .flex()
        .flex_col()
        .gap(surface.content_gap)
        .w(surface.max_width)
        .max_w_full()
        .min_w(surface.min_width)
        .p(surface.padding)
        .rounded(surface.radius)
        .bg(surface.background)
        .text_color(surface.foreground)
        .shadow(surface.shadow.to_vec())
        .when_some(config.title, |card, title| {
            card.child(
                gpui_base::DialogTitle::new()
                    .text_size(surface.title_size)
                    .line_height(surface.title_line_height)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
        })
        .child(
            div()
                .text_size(surface.body_size)
                .line_height(surface.body_line_height)
                .children(config.children),
        )
        .when(!config.footer.is_empty(), |card| {
            card.child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_end()
                    .gap(surface.footer_gap)
                    .children(config.footer),
            )
        });

    // `AlertDialog` is the base's own specialization: `Role::AlertDialog`, no
    // backdrop dismissal. Escape is additionally cut because a forced choice
    // must come from one of the footer actions.
    macro_rules! finish {
        ($host:expr) => {{
            let mut host = $host;
            if let Some(on_ok) = config.on_ok {
                host = host.on_ok(move |event, window, cx| on_ok(event, window, cx));
            }
            if let Some(on_cancel) = config.on_cancel {
                host = host.on_cancel(move |event, window, cx| on_cancel(event, window, cx));
            }
            if let Some(on_close) = config.on_close {
                host = host.on_close(move |event, window, cx| on_close(event, window, cx));
            }
            host.into_any_element()
        }};
    }

    let host = if alert {
        finish!(
            gpui_base::AlertDialog::new(cx)
                .open(is_open)
                .on_open_change(report)
                .focus_handle(panel_focus)
                .backdrop(backdrop)
                .popup(popup)
                .close_on_escape(false)
        )
    } else {
        finish!(
            gpui_base::Dialog::new(cx)
                .open(is_open)
                .on_open_change(report)
                .focus_handle(panel_focus)
                .backdrop(backdrop)
                .popup(popup)
        )
    };

    // The base host renders `deferred(anchored(...))` while open and an empty
    // `div()` while closed. The open host is absolute and drops out of the
    // parent's flex flow, while the closed placeholder is in flow — opening a
    // dialog inside a flex column would otherwise remove one item and its gap,
    // moving the content below. Keep one zero-height in-flow slot in both
    // states, the same contract `Sheet` uses.
    div().h(px(0.)).child(host).into_any_element()
}

#[cfg(test)]
mod tests;
