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
    AlertDialog, Avatar, Badge, Button, Checkbox, Dialog, DialogOpenChange, Divider, Field,
    FluentIcon, FluentRoot, Icon, IconStyle, InputAppearance, Label, Link, Menu, MenuDropdown,
    MenuHeader, MenuItem, MenuSeparator, Popover, ProgressBar, Radio, Sheet, SheetOpenChange,
    Spinner, Switch, TabItem, TabList, TextInput, Textarea, TextareaAppearance, TooltipSurface,
    TooltipView, ValidationMessage, tooltip,
};
pub use fluent_core::spec::ControlSize;
pub use fluent_core::spec::button::{ButtonAppearance, ButtonShape, IconPosition};
pub use fluent_core::spec::feedback::{
    AvatarSize, BadgeAppearance, BadgeSize, ProgressAppearance, SpinnerSize,
};
pub use fluent_core::spec::misc::{DividerAppearance, LinkAppearance};
pub use fluent_core::spec::sheet::SheetSide;
pub use fluent_core::spec::tabs::TabsOrientation;
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
    // Dismissing an overlay with Escape needs a binding scoped to the overlay's
    // key context; GPUI ships none. Bound here so applications do not have to
    // reproduce it.
    cx.bind_keys([gpui::KeyBinding::new(
        "escape",
        fluent_components::Dismiss,
        Some(fluent_components::OVERLAY_KEY_CONTEXT),
    )]);
    // Menu navigation. Scoped to the menu's own context so arrow keys stay
    // available to text fields everywhere else in the window.
    cx.bind_keys([
        gpui::KeyBinding::new(
            "down",
            fluent_components::MenuNext,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "up",
            fluent_components::MenuPrev,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "home",
            fluent_components::MenuFirst,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "end",
            fluent_components::MenuLast,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
        // Submenus: Right opens the current row submenu, Left closes one level,
        // Escape closes one level or the whole menu at the root. Tab is bound
        // separately because it closes the entire chain.
        gpui::KeyBinding::new(
            "right",
            fluent_components::MenuSubmenuOpen,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "left",
            fluent_components::MenuSubmenuClose,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "escape",
            fluent_components::MenuEscape,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "enter",
            fluent_components::MenuActivate,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "space",
            fluent_components::MenuActivate,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
        // A menu closes when Tab leaves it. Scoping these to the menu context
        // shadows the root traversal binding, which would otherwise move focus out
        // and leave the panel open behind the pointer.
        // Tabs: arrows follow the strip's orientation; the component decides what
        // "next" means, so both axes map to the same actions.
        gpui::KeyBinding::new(
            "right",
            fluent_components::TabsNext,
            Some(fluent_components::TABS_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "down",
            fluent_components::TabsNext,
            Some(fluent_components::TABS_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "left",
            fluent_components::TabsPrev,
            Some(fluent_components::TABS_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "up",
            fluent_components::TabsPrev,
            Some(fluent_components::TABS_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "home",
            fluent_components::TabsFirst,
            Some(fluent_components::TABS_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "end",
            fluent_components::TabsLast,
            Some(fluent_components::TABS_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "tab",
            fluent_components::MenuTabOut,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "shift-tab",
            fluent_components::MenuTabOut,
            Some(fluent_components::MENU_KEY_CONTEXT),
        ),
    ]);
    // Tab order inside `FluentRoot`. gpui-base's own binding is scoped to its
    // private root context, so it never fires under a Fluent application.
    cx.bind_keys([
        gpui::KeyBinding::new(
            "tab",
            fluent_components::FocusNext,
            Some(fluent_components::ROOT_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "shift-tab",
            fluent_components::FocusPrev,
            Some(fluent_components::ROOT_KEY_CONTEXT),
        ),
    ]);
}
