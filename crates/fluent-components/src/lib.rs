//! Fluent 2 components.
//!
//! Each component is a thin visual layer: behavior comes from `gpui-base`,
//! every color, radius and size comes from `fluent_core::spec`.

use gpui::actions;

actions!(fluent, [Dismiss]);

/// Key context an overlay sets on the element that should receive [`Dismiss`].
///
/// A binding scoped to a context only fires while focus is inside an element
/// carrying that context, which keeps Escape from closing an overlay the user
/// is not interacting with.
pub const OVERLAY_KEY_CONTEXT: &str = "FluentOverlay";

pub mod button;
pub mod feedback;
pub mod field;
pub mod icon;
pub mod input;
pub mod menu;
pub mod misc;
pub mod motion;
pub mod popover;
pub mod root;
pub mod selection;
pub mod switch;
pub mod textarea;
pub mod tooltip;

pub use button::Button;
pub use feedback::{Avatar, Badge, ProgressBar, TooltipSurface, TooltipView};
pub use field::{Field, Label, ValidationMessage};
pub use fluent_core::spec::ControlSize;
pub use fluent_core::spec::button::{ButtonAppearance, ButtonShape, IconPosition};
pub use fluent_core::spec::input::InputAppearance;
pub use fluent_core::spec::textarea::TextareaAppearance;
pub use fluent_icons::{FluentIcon, IconStyle};
pub use icon::Icon;
pub use input::TextInput;
pub use menu::{Menu, MenuDropdown, MenuHeader, MenuItem, MenuSeparator};
pub use misc::{Divider, Link};
pub use popover::Popover;
pub use root::FluentRoot;
pub use selection::{Checkbox, Radio};
pub use switch::Switch;
pub use textarea::Textarea;
pub use tooltip::TooltipService;
