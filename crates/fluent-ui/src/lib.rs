//! Microsoft Fluent 2 for Rust, rendered by GPUI.
//!
//! This is the only crate an application needs to depend on.
//!
//! # Getting started
//!
//! ```no_run
//! fluent_ui::application().run(|cx| {
//!     fluent_ui::init(cx);
//!     // open windows here
//! });
//! ```
//!
//! # Do not depend on `gpui` directly
//!
//! This crate is built against the `gpui-pre` snapshot that `gpui-base` pins.
//! Use the [`gpui`] re-export here instead of adding `gpui` to your own
//! `Cargo.toml`: two different gpui crates have incompatible types.

pub use fluent_components::{
    Button, FluentIcon, FluentRoot, Icon, IconStyle, InputAppearance, TextInput, Textarea,
    TextareaAppearance,
};
pub use fluent_core::spec::ControlSize;
pub use fluent_core::spec::button::{ButtonAppearance, ButtonShape, IconPosition};
pub use fluent_theme::{Appearance, FluentTheme, FontMode};
pub use fluent_tokens::Tokens;
pub use gpui;
pub use gpui_base;
pub use gpui_platform;

/// Common imports for building a Fluent application.
pub mod prelude {
    pub use gpui::prelude::*;
    pub use gpui::{div, hsla, point, px, rgb, rgba, size};
}

/// Creates the GPUI application for the current platform.
///
/// Thin re-export of [`gpui_platform::application`] so applications never need
/// to name the platform crate.
pub fn application() -> gpui::Application {
    gpui_platform::application()
}

/// Installs the Fluent design system into the application.
///
/// Call once, before opening windows. Installs `gpui-base` behavior
/// infrastructure and the global [`fluent_theme::FluentTheme`].
///
/// Later milestones extend this with assets, key bindings and system
/// appearance tracking.
pub fn init(cx: &mut gpui::App) {
    fluent_core::gpui_compat::init_base(cx);
    fluent_theme::init(cx);
}
