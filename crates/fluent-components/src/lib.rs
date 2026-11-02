//! Fluent 2 components.
//!
//! Each component is a thin visual layer: behavior comes from `gpui-base`,
//! every color, radius and size comes from `fluent_core::spec`.

pub mod button;
pub mod field;
pub mod icon;
pub mod input;
pub mod misc;
pub mod root;
pub mod selection;
pub mod switch;
pub mod textarea;

pub use button::Button;
pub use field::{Field, Label, ValidationMessage};
pub use fluent_core::spec::ControlSize;
pub use fluent_core::spec::button::{ButtonAppearance, ButtonShape, IconPosition};
pub use fluent_core::spec::input::InputAppearance;
pub use fluent_core::spec::textarea::TextareaAppearance;
pub use fluent_icons::{FluentIcon, IconStyle};
pub use icon::Icon;
pub use input::TextInput;
pub use misc::{Divider, Link};
pub use root::FluentRoot;
pub use selection::{Checkbox, Radio};
pub use switch::Switch;
pub use textarea::Textarea;
