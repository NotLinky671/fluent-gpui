//! Anchored tooltip service.
//!
//! GPUI's native `.tooltip()` paints at the mouse position, which is not how
//! Fluent tooltips behave: they anchor to the trigger and flip above/below.
//! This module hosts a `gpui_base::TooltipOverlay` at the root and lets
//! components request a tooltip with the trigger's bounds.
//!
//! The overlay itself is a per-window keyed state, but it is *registered* in a
//! table keyed by window id. A single App global holding one overlay was wrong:
//! a second window's root replaced the first entry and tooltips crossed windows.
//! The table also keeps lookups possible from event handlers, where GPUI forbids
//! creating keyed state (it may only happen during layout, prepaint or paint).

use fluent_core::gpui_compat::{App, Global, Window};
use gpui::{AnyWindowHandle, Entity};
use gpui_base::TooltipOverlay;

/// Key of the per-window tooltip overlay.
const TOOLTIP_OVERLAY_KEY: &str = "fluent-tooltip-overlay";

/// Overlays by the window that hosts them.
#[derive(Default)]
pub struct TooltipService {
    overlays:
        std::cell::RefCell<std::collections::HashMap<AnyWindowHandle, Entity<TooltipOverlay>>>,
}

impl Global for TooltipService {}

/// Creates this window's overlay if needed and records it for later lookups.
///
/// Called by [`crate::FluentRoot`] during render, which is the only phase where
/// keyed state may be created.
pub fn register(window: &mut Window, cx: &mut App) -> Entity<TooltipOverlay> {
    let overlay = window.use_keyed_state(TOOLTIP_OVERLAY_KEY, cx, |_, _| TooltipOverlay::new());
    if cx.try_global::<TooltipService>().is_none() {
        cx.set_global(TooltipService::default());
    }
    cx.global::<TooltipService>()
        .overlays
        .borrow_mut()
        .insert(window.window_handle(), overlay.clone());
    overlay
}

/// The overlay hosted by `window`, when its root has been rendered.
pub fn overlay(window: &Window, cx: &App) -> Option<Entity<TooltipOverlay>> {
    cx.try_global::<TooltipService>()?
        .overlays
        .borrow()
        .get(&window.window_handle())
        .cloned()
}

#[cfg(test)]
mod tests;
