//! Fluent 2 components.
//!
//! Each component is a thin visual layer: behavior comes from `gpui-base`,
//! every color, radius and size comes from `fluent_core::spec`.

pub mod button;
pub mod icon;
pub mod root;

pub use button::Button;
pub use fluent_core::spec::ControlSize;
pub use fluent_core::spec::button::{ButtonAppearance, ButtonShape, IconPosition};
pub use fluent_icons::{FluentIcon, IconStyle};
pub use icon::Icon;
pub use root::FluentRoot;
