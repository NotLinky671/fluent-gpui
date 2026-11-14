//! Fluent 2 components.
//!
//! Each component is a thin visual layer: behavior comes from `gpui-base`,
//! every color, radius and size comes from `fluent_core::spec`.

use gpui::actions;

actions!(
    fluent,
    [
        Dismiss,
        DismissAndFocusNext,
        DismissAndFocusPrev,
        FocusNext,
        FocusPrev,
        MenuNext,
        MenuPrev,
        MenuFirst,
        MenuLast,
        MenuActivate,
        MenuTabOut,
        MenuTabBackward,
        MenuSubmenuOpen,
        MenuSubmenuClose,
        MenuEscape,
        TabsNext,
        TabsPrev,
        TabsFirst,
        TabsLast
    ]
);

/// Key context an overlay sets on the element that should receive [`Dismiss`].
///
/// A binding scoped to a context only fires while focus is inside an element
/// carrying that context, which keeps Escape from closing an overlay the user
/// is not interacting with.
pub const OVERLAY_KEY_CONTEXT: &str = "FluentOverlay";

/// Key context [`crate::FluentRoot`] sets so Tab order works inside it.
///
/// `gpui-base` binds `tab` to its own `Root` context, which only fires under its
/// own root view. Applications here mount [`crate::FluentRoot`] instead, so the
/// binding has to be repeated for a context that root actually carries.
pub const ROOT_KEY_CONTEXT: &str = "FluentRoot";

/// Key context a menu panel sets for its own navigation keys.
///
/// Arrow keys are only bound inside this context, so a menu does not take them
/// away from text fields elsewhere in the window.
pub const TABS_KEY_CONTEXT: &str = "FluentTabs";

pub const MENU_KEY_CONTEXT: &str = "FluentMenu";

/// Marker a Fluent text control sets when Tab should move focus instead of
/// editing the text. It is an ancestor context, so the binding must also name
/// gpui-base's deeper `Input` context to tie at that depth and win as the later
/// binding.
pub const TEXT_INPUT_NAVIGATE_KEY_CONTEXT: &str = "FluentTextInputNavigate";

/// Binding context for Tab/Shift+Tab on a text control in navigate mode.
pub const TEXT_INPUT_NAVIGATE_BINDING_CONTEXT: &str = "FluentTextInputNavigate > Input";

pub mod button;
mod circle;
pub mod dialog;
pub mod feedback;
pub mod field;
pub mod icon;
pub mod input;
mod interaction;
pub mod menu;
pub mod misc;
pub mod motion;
pub mod popover;
pub mod root;
pub mod selection;
pub mod sheet;
pub mod switch;
pub mod tabs;
pub mod textarea;
pub mod titlebar;
pub mod tooltip;

pub use button::Button;
pub use dialog::{AlertDialog, Dialog, DialogOpenChange};
pub use feedback::{Avatar, Badge, ProgressBar, Spinner, TooltipSurface, TooltipView};
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
pub use selection::{Checkbox, Radio, RadioGroup};
pub use sheet::{Sheet, SheetOpenChange};
pub use switch::Switch;
pub use tabs::{TabItem, TabList};
pub use textarea::{Textarea, TextareaTabBehavior};
pub use titlebar::{TitleBar, TitleBarLabels};

#[cfg(test)]
mod input_selection_tests;
