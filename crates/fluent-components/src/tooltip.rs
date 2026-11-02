//! Anchored tooltip service.
//!
//! GPUI's native `.tooltip()` paints at the mouse position, which is not how
//! Fluent tooltips behave: they anchor to the trigger and flip above/below.
//! This module hosts a `gpui_base::TooltipOverlay` at the root and lets
//! components request a tooltip with the trigger's bounds.

use fluent_core::gpui_compat::{App, Global};
use gpui::Entity;
use gpui_base::TooltipOverlay;

/// Global handle to the root-hosted tooltip overlay.
pub struct TooltipService {
    pub overlay: Entity<TooltipOverlay>,
}

impl Global for TooltipService {}

/// Returns the overlay when the app is wrapped in [`crate::FluentRoot`].
pub fn overlay(cx: &App) -> Option<Entity<TooltipOverlay>> {
    cx.try_global::<TooltipService>()
        .map(|service| service.overlay.clone())
}
