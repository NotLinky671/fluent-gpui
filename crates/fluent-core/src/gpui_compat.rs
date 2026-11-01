//! The single point of contact with the GPUI snapshot.
//!
//! `gpui-base` pins `gpui-pre =0.3.10`, a republished snapshot of Zed's gpui
//! (ADR-0001). Snapshot bumps are expected to rename or reshape APIs, so every
//! Fluent crate imports GPUI through this module instead of naming `gpui`
//! directly. A bump then touches this file plus whatever the compiler reports,
//! rather than every component.
//!
//! Rules:
//! - Only re-export what the library actually uses; this is not a GPUI facade.
//! - Add a short note next to anything that is a workaround for a snapshot
//!   quirk, so the next bump knows what to re-check.

pub use gpui::{
    AnyElement, App, Application, BoxShadow, ClickEvent, Context, ElementId, FontWeight, Global,
    Hsla, InteractiveElement, IntoElement, ParentElement, Pixels, Render, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, point, px, rgba, svg,
};
pub use gpui_base::StyledExt;

/// Runs the base-layer initializer.
///
/// Wrapped so call sites never name `gpui_base::init` directly.
pub fn init_base(cx: &mut App) {
    gpui_base::init(cx);
}