//! Single-component fixtures for pixel baselines.
//!
//! Each fixture renders exactly one component on a fixed canvas so the resulting
//! PNG can be compared against a stored baseline. Keep the canvas small: a diff
//! should point at the component, not at surrounding chrome.

use fluent_ui::gpui::AppContext as _;
use fluent_ui::gpui::{
    AnyElement, App, FocusHandle, IntoElement, ParentElement, Styled, Window, div, px,
};
use fluent_ui::gpui_base::input::InputState;
use fluent_ui::{
    Avatar, AvatarSize, Badge, BadgeAppearance, Button, ButtonAppearance, Checkbox, Dialog,
    Divider, Field, Link, Menu, MenuHeader, MenuItem, ProgressBar, Radio, Sheet, SheetSide, Switch,
    TabItem, TabList, TextInput,
};

/// One baseline fixture.
pub struct Fixture {
    /// Stable name; it is also the baseline file name.
    pub name: &'static str,
    /// Logical canvas width.
    pub width: f32,
    /// Logical canvas height.
    pub height: f32,
}

const W: f32 = 240.0;

/// Every fixture, in baseline order.
pub const FIXTURES: &[Fixture] = &[
    Fixture {
        name: "button-primary",
        width: W,
        height: 60.0,
    },
    Fixture {
        name: "button-secondary",
        width: W,
        height: 60.0,
    },
    Fixture {
        name: "button-subtle",
        width: W,
        height: 60.0,
    },
    Fixture {
        name: "checkbox-unchecked",
        width: W,
        height: 50.0,
    },
    Fixture {
        name: "checkbox-checked",
        width: W,
        height: 50.0,
    },
    Fixture {
        name: "radio-checked",
        width: W,
        height: 50.0,
    },
    Fixture {
        name: "switch-on",
        width: W,
        height: 50.0,
    },
    Fixture {
        name: "switch-off",
        width: W,
        height: 50.0,
    },
    Fixture {
        name: "input-outline",
        width: W,
        height: 56.0,
    },
    Fixture {
        name: "field-with-hint",
        width: W,
        height: 96.0,
    },
    Fixture {
        name: "link-default",
        width: W,
        height: 40.0,
    },
    Fixture {
        name: "divider-horizontal",
        width: W,
        height: 40.0,
    },
    Fixture {
        name: "badge-danger",
        width: W,
        height: 40.0,
    },
    Fixture {
        name: "progress-40",
        width: W,
        height: 40.0,
    },
    Fixture {
        name: "tabs-horizontal",
        width: 340.0,
        height: 60.0,
    },
    Fixture {
        name: "tabs-vertical",
        width: 220.0,
        height: 140.0,
    },
    Fixture {
        name: "menu-panel",
        width: 240.0,
        height: 220.0,
    },
    Fixture {
        name: "dialog-open",
        width: 420.0,
        height: 260.0,
    },
    Fixture {
        name: "avatar-medium",
        width: W,
        height: 60.0,
    },
    Fixture {
        name: "sheet-right-open",
        width: 460.0,
        height: 300.0,
    },
];

/// Canvas size of a fixture, in logical pixels.
pub fn size(name: &str) -> Option<(f32, f32)> {
    FIXTURES
        .iter()
        .find(|fixture| fixture.name == name)
        .map(|fixture| (fixture.width, fixture.height))
}

/// A tab-stop focus handle that survives redraws.
///
/// Fixtures are rendered as `RenderOnce` trees, so a fresh `cx.focus_handle()`
/// each frame would replace the strip's stop every paint. The keyed state keeps
/// one handle for the fixture's lifetime.
fn tab_focus(window: &mut Window, cx: &mut App, key: &'static str) -> FocusHandle {
    let state = window.use_keyed_state(key, cx, |_, cx| cx.focus_handle().tab_stop(true));
    state.read(cx).clone()
}

/// Renders one fixture, or `None` when the name is unknown.
pub fn render(name: &str, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    let element = match name {
        "button-primary" => Button::new("fx-button-primary")
            .appearance(ButtonAppearance::Primary)
            .accessibility_label("Save")
            .child("Save")
            .into_any_element(),
        "button-secondary" => Button::new("fx-button-secondary")
            .appearance(ButtonAppearance::Secondary)
            .accessibility_label("Cancel")
            .child("Cancel")
            .into_any_element(),
        "button-subtle" => Button::new("fx-button-subtle")
            .appearance(ButtonAppearance::Subtle)
            .accessibility_label("Subtle")
            .child("Subtle")
            .into_any_element(),
        "checkbox-unchecked" => Checkbox::new("fx-checkbox-unchecked")
            .label("Send updates")
            .into_any_element(),
        "checkbox-checked" => Checkbox::new("fx-checkbox-checked")
            .checked(true)
            .label("Send updates")
            .into_any_element(),
        "radio-checked" => Radio::new("fx-radio-checked")
            .checked(true)
            .label("Option B")
            .into_any_element(),
        "switch-on" => Switch::new("fx-switch-on")
            .checked(true)
            .label("Wi-Fi")
            .into_any_element(),
        "switch-off" => Switch::new("fx-switch-off")
            .checked(false)
            .label("Wi-Fi")
            .into_any_element(),
        "input-outline" => {
            let state = cx.new(|cx| InputState::new(window, cx).placeholder("Placeholder"));
            div()
                .w(px(200.))
                .child(TextInput::new(&state).accessibility_label("Input"))
                .into_any_element()
        }
        "field-with-hint" => {
            let state = cx.new(|cx| InputState::new(window, cx));
            div()
                .w(px(200.))
                .child(
                    Field::new("Display name")
                        .required(true)
                        .hint("Shown to other people.")
                        .child(TextInput::new(&state).accessibility_label("Display name")),
                )
                .into_any_element()
        }
        "link-default" => Link::new("fx-link", "Learn more").into_any_element(),
        "divider-horizontal" => div().w(px(200.)).child(Divider::new()).into_any_element(),
        "badge-danger" => Badge::new("Danger")
            .appearance(BadgeAppearance::Danger)
            .into_any_element(),
        "progress-40" => div()
            .w(px(200.))
            .child(ProgressBar::new(40.))
            .into_any_element(),
        "avatar-medium" => Avatar::new("Ada Lovelace")
            .size(AvatarSize::Medium)
            .into_any_element(),

        "tabs-horizontal" => TabList::new("fx-tabs")
            .track_focus(&tab_focus(window, cx, "fx-tabs-focus"))
            .default_selected(0)
            .tab(TabItem::new("fx-tab-a", "Overview"))
            .tab(TabItem::new("fx-tab-b", "Activity"))
            .tab(TabItem::new("fx-tab-c", "Archived").disabled(true))
            .into_any_element(),
        "tabs-vertical" => TabList::new("fx-tabs-vertical")
            .track_focus(&tab_focus(window, cx, "fx-tabs-vertical-focus"))
            .vertical()
            .default_selected(1)
            .tab(TabItem::new("fx-vtab-a", "Overview"))
            .tab(TabItem::new("fx-vtab-b", "Activity"))
            .tab(TabItem::new("fx-vtab-c", "Settings"))
            .into_any_element(),
        "menu-panel" => Menu::new("fx-menu")
            .header(MenuHeader::new("Actions"))
            .item(MenuItem::new("fx-menu-rename", "Rename"))
            .item(MenuItem::new("fx-menu-copy", "Copy"))
            .separator()
            .item(MenuItem::new("fx-menu-archive", "Archive").disabled(true))
            .item(MenuItem::new("fx-menu-delete", "Delete").danger(true))
            .into_any_element(),
        "dialog-open" => Dialog::new("fx-dialog")
            .title("Edit details")
            .open(true)
            .child("Dialogs dim the page and take focus.")
            .footer_child(
                Button::new("fx-dialog-cancel")
                    .appearance(ButtonAppearance::Secondary)
                    .accessibility_label("Cancel")
                    .child("Cancel"),
            )
            .into_any_element(),
        "sheet-right-open" => Sheet::new("fx-sheet-right")
            .side(SheetSide::Right)
            .default_open(true)
            .title("Filters")
            .child("Narrow the list to what you need.")
            .footer_child(
                Button::new("fx-sheet-cancel")
                    .appearance(ButtonAppearance::Secondary)
                    .accessibility_label("Cancel")
                    .child("Cancel"),
            )
            .footer_child(
                Button::new("fx-sheet-apply")
                    .appearance(ButtonAppearance::Primary)
                    .accessibility_label("Apply")
                    .child("Apply"),
            )
            .into_any_element(),
        _ => return None,
    };
    Some(element)
}
