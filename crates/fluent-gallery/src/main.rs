//! Component gallery: the manual review surface for `fluent-ui`.
//!
//! - `FLUENT_GALLERY_SMOKE=1` opens the window and quits, for CI boot checks.
//! - `FLUENT_GALLERY_SCREENSHOT=<path>` (with `--features screenshot`) writes a
//!   PNG of the first frame.

mod fixtures;

use std::time::Duration;

use fluent_ui::gpui::{
    AnyElement, App, Bounds, ClickEvent, Context, FocusHandle, Hsla, Render,
    StatefulInteractiveElement, Subscription, Window, WindowBounds, WindowOptions, div, px, size,
};
use fluent_ui::gpui_base::input::{InputState, TextareaState};
use fluent_ui::prelude::*;
use fluent_ui::{
    AlertDialog, Appearance, Avatar, AvatarSize, Badge, BadgeAppearance, BadgeSize, Button,
    ButtonAppearance, ButtonShape, Checkbox, ControlSize, Dialog, Divider, Field, FluentIcon,
    FluentRoot, FluentTheme, FontMode, InputAppearance, Link, Menu, MenuDropdown, MenuHeader,
    MenuItem, Popover, ProgressAppearance, ProgressBar, Radio, Sheet, SheetSide, Spinner,
    SpinnerSize, Switch, TabItem, TabList, TextInput, Textarea, TextareaAppearance, TitleBar,
    TooltipSurface, ValidationMessage,
};

const APPEARANCES: [ButtonAppearance; 5] = [
    ButtonAppearance::Secondary,
    ButtonAppearance::Outline,
    ButtonAppearance::Primary,
    ButtonAppearance::Subtle,
    ButtonAppearance::Transparent,
];

const SIZES: [ControlSize; 3] = [ControlSize::Small, ControlSize::Medium, ControlSize::Large];

fn appearance_name(appearance: ButtonAppearance) -> &'static str {
    match appearance {
        ButtonAppearance::Secondary => "secondary",
        ButtonAppearance::Outline => "outline",
        ButtonAppearance::Primary => "primary",
        ButtonAppearance::Subtle => "subtle",
        ButtonAppearance::Transparent => "transparent",
    }
}

fn size_name(size: ControlSize) -> &'static str {
    match size {
        ControlSize::Small => "small",
        ControlSize::Medium => "medium",
        ControlSize::Large => "large",
    }
}

/// Root view: theme switcher, font mode, and the button matrix.
struct Gallery {
    appearance: Appearance,
    /// Set by `FLUENT_GALLERY_FIXTURE`: render one component on a fixed canvas.
    fixture: Option<String>,
    clicks: usize,
    inputs: Vec<gpui::Entity<InputState>>,
    textareas: Vec<gpui::Entity<TextareaState>>,
    checkbox_a: bool,
    radio_choice: usize,
    switch_a: bool,
    switch_b: bool,
    dialog_open: bool,
    tab_index: usize,
    /// Focus for the horizontal strip. The strip is one Tab stop; arrows move
    /// the selection. Without `track_focus` the strip is unreachable by Tab.
    tab_focus: FocusHandle,
    vertical_tab_index: usize,
    /// Focus for the vertical strip, kept separate so each strip is its own
    /// stop and the two never share selection state.
    vertical_tab_focus: FocusHandle,
    alert_open: bool,
    /// The edge of the open sheet, or `None` when it is closed.
    sheet_side: Option<SheetSide>,
    /// Keeps the OS appearance observer alive for the window's lifetime.
    _appearance_sub: Option<Subscription>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let fixture = std::env::var("FLUENT_GALLERY_FIXTURE").ok();
        // A forced appearance (screenshots, CI baselines) must not be
        // overwritten by the OS observer, which fires immediately.
        let forced = std::env::var_os("FLUENT_GALLERY_APPEARANCE").is_some();
        let sub = if forced {
            None
        } else {
            Some(FluentTheme::follow_system(window))
        };
        // 4 appearances x 3 sizes, plus disabled / invalid / slotted fields.
        let inputs: Vec<_> = (0..18)
            .map(|index| {
                cx.new(|cx| InputState::new(window, cx).placeholder(format!("Placeholder {index}")))
            })
            .collect();
        inputs[0].update(cx, |state, cx| {
            state.set_value("Hello 世界", window, cx);
            state.focus(window, cx);
        });

        let textareas: Vec<_> = (0..11)
            .map(|index| {
                cx.new(|cx| {
                    TextareaState::new(window, cx).placeholder(format!("Multi-line {index}"))
                })
            })
            .collect();

        Self {
            appearance: FluentTheme::global(cx).appearance(),
            clicks: 0,
            inputs,
            textareas,
            checkbox_a: true,
            radio_choice: 1,
            switch_a: true,
            switch_b: false,
            dialog_open: false,
            tab_index: 0,
            tab_focus: cx.focus_handle().tab_stop(true),
            vertical_tab_index: 0,
            vertical_tab_focus: cx.focus_handle().tab_stop(true),
            alert_open: false,
            sheet_side: None,
            fixture,
            _appearance_sub: sub,
        }
    }

    fn apply(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        self.appearance = appearance;
        FluentTheme::global(cx)
            .with_appearance(appearance)
            .install(cx);
        cx.notify();
    }

    fn toggle_font(&mut self, cx: &mut Context<Self>) {
        let current = FluentTheme::global(cx);
        let next = match current.font_mode() {
            FontMode::SegoeCompat => FontMode::System,
            _ => FontMode::SegoeCompat,
        };
        current.with_font_mode(next).install(cx);
        cx.notify();
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = FluentTheme::global(cx);

        // Baseline mode: one fixture, centred on a canvas the window is sized to.
        if let Some(element) = self
            .fixture
            .as_deref()
            .and_then(|name| fixtures::render(name, window, cx))
        {
            return FluentRoot::new()
                .flex()
                .items_center()
                .justify_center()
                .child(element)
                .into_any_element();
        }

        let t = theme.tokens();

        let swatch = |c: Hsla| div().w(px(44.)).h(px(28.)).rounded(px(4.)).bg(c);

        let weak = cx.entity().downgrade();
        let handler = {
            let weak = weak.clone();
            move || {
                let weak = weak.clone();
                move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                    weak.update(cx, |this, cx| {
                        this.clicks += 1;
                        cx.notify();
                    })
                    .ok();
                }
            }
        };

        let mut matrix = div().flex().flex_col().gap(px(12.)).p(px(16.));
        let mut index = 0usize;

        for appearance in APPEARANCES {
            let mut row = div().flex().items_center().gap(px(12.)).child(
                div()
                    .w(px(96.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(appearance_name(appearance)),
            );
            for size in SIZES {
                row = row.child(
                    Button::new(("btn", index))
                        .appearance(appearance)
                        .size(size)
                        .on_click(handler())
                        .accessibility_label(size_name(size))
                        .child(size_name(size)),
                );
                index += 1;
            }
            matrix = matrix.child(row);
        }

        let mut extra = div().flex().items_center().gap(px(12.)).child(
            div()
                .w(px(96.))
                .text_color(t.color.neutral_foreground_3)
                .child("disabled"),
        );
        for appearance in APPEARANCES {
            extra = extra.child(
                Button::new(("disabled", index))
                    .appearance(appearance)
                    .disabled(true)
                    .on_click(handler())
                    .accessibility_label("disabled")
                    .child("disabled"),
            );
            index += 1;
        }
        matrix = matrix.child(extra);

        let mut shapes = div().flex().items_center().gap(px(12.)).child(
            div()
                .w(px(96.))
                .text_color(t.color.neutral_foreground_3)
                .child("shape"),
        );
        for shape in [
            ButtonShape::Rounded,
            ButtonShape::Circular,
            ButtonShape::Square,
        ] {
            let shape_label = match shape {
                ButtonShape::Rounded => "rounded",
                ButtonShape::Circular => "circular",
                ButtonShape::Square => "square",
            };
            shapes = shapes.child(
                Button::new(("shape", index))
                    .shape(shape)
                    .on_click(handler())
                    .accessibility_label(shape_label)
                    .child(shape_label),
            );
            index += 1;
        }
        matrix = matrix.child(shapes);

        let mut icon_row = div().flex().items_center().gap(px(12.)).child(
            div()
                .w(px(96.))
                .text_color(t.color.neutral_foreground_3)
                .child("icon"),
        );
        for (icon, label) in [
            (FluentIcon::Add, "Add"),
            (FluentIcon::Search, "Search"),
            (FluentIcon::Settings, "Settings"),
        ] {
            icon_row = icon_row.child(
                Button::new(("icon", index))
                    .icon(icon)
                    .on_click(handler())
                    .accessibility_label(label)
                    .child(label),
            );
            index += 1;
        }
        icon_row = icon_row.child(
            Button::new(("icon", index))
                .appearance(ButtonAppearance::Primary)
                .icon_after(FluentIcon::ChevronDown)
                .on_click(handler())
                .accessibility_label("Options")
                .child("Options"),
        );
        index += 1;
        for (icon, label) in [
            (FluentIcon::Add, "Add"),
            (FluentIcon::MoreHorizontal, "More actions"),
            (FluentIcon::Dismiss, "Dismiss"),
        ] {
            icon_row = icon_row.child(
                Button::new(("icon-only", index))
                    .appearance(ButtonAppearance::Subtle)
                    .icon(icon)
                    .icon_only()
                    .on_click(handler())
                    .accessibility_label(label),
            );
            index += 1;
        }
        matrix = matrix.child(icon_row);

        let mut input_matrix = div().flex().flex_col().gap(px(12.)).p(px(16.));
        let input_appearances = [
            InputAppearance::Outline,
            InputAppearance::Underline,
            InputAppearance::FilledDarker,
            InputAppearance::FilledLighter,
        ];
        let mut input_index = 0usize;
        for appearance in input_appearances {
            let appearance_label = match appearance {
                InputAppearance::Outline => "outline",
                InputAppearance::Underline => "underline",
                InputAppearance::FilledDarker => "filled-darker",
                InputAppearance::FilledLighter => "filled-lighter",
            };
            let mut row = div().flex().items_center().gap(px(12.)).child(
                div()
                    .w(px(96.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(appearance_label),
            );
            if matches!(appearance, InputAppearance::FilledLighter) {
                row = row.bg(FluentTheme::global(cx).tokens().color.neutral_background_2);
            }
            for size in SIZES {
                row = row.child(
                    div().w(px(220.)).child(
                        TextInput::new(&self.inputs[input_index])
                            .appearance(appearance)
                            .size(size)
                            .accessibility_label(format!(
                                "{appearance_label} input, {}",
                                size_name(size)
                            )),
                    ),
                );
                input_index += 1;
            }
            input_matrix = input_matrix.child(row);
        }

        let mut input_extras = div().flex().items_center().gap(px(12.)).child(
            div()
                .w(px(96.))
                .text_color(t.color.neutral_foreground_3)
                .child("states"),
        );
        input_extras = input_extras
            .child(
                div().w(px(220.)).child(
                    TextInput::new(&self.inputs[12])
                        .disabled(true)
                        .size(ControlSize::Medium)
                        .accessibility_label("disabled input"),
                ),
            )
            .child(
                div().w(px(220.)).child(
                    TextInput::new(&self.inputs[13])
                        .invalid(true)
                        .size(ControlSize::Medium)
                        .accessibility_label("invalid input"),
                ),
            )
            .child(
                div().w(px(220.)).child(
                    TextInput::new(&self.inputs[14])
                        .content_before(fluent_ui::Icon::new(FluentIcon::Search).size(px(16.)))
                        .size(ControlSize::Medium)
                        .accessibility_label("search input"),
                ),
            );
        input_matrix = input_matrix.child(input_extras);
        let mut textarea_matrix = div().flex().flex_col().gap(px(12.)).p(px(16.));
        let textarea_appearances = [
            TextareaAppearance::Outline,
            TextareaAppearance::FilledDarker,
            TextareaAppearance::FilledLighter,
        ];
        let mut textarea_index = 0usize;
        for appearance in textarea_appearances {
            let appearance_label = match appearance {
                TextareaAppearance::Outline => "outline",
                TextareaAppearance::Filled => "filled",
                TextareaAppearance::FilledDarker => "filled-darker",
                TextareaAppearance::FilledLighter => "filled-lighter",
            };
            let mut row = div().flex().items_start().gap(px(12.)).child(
                div()
                    .w(px(96.))
                    .pt(px(8.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(appearance_label),
            );
            if matches!(appearance, TextareaAppearance::FilledLighter) {
                row = row.bg(FluentTheme::global(cx).tokens().color.neutral_background_2);
            }
            for size in SIZES {
                row = row.child(
                    div().w(px(220.)).child(
                        Textarea::new(&self.textareas[textarea_index])
                            .appearance(appearance)
                            .size(size)
                            .accessibility_label(format!(
                                "{appearance_label} textarea, {}",
                                size_name(size)
                            )),
                    ),
                );
                textarea_index += 1;
            }
            textarea_matrix = textarea_matrix.child(row);
        }
        textarea_matrix = textarea_matrix.child(
            div()
                .flex()
                .items_start()
                .gap(px(12.))
                .child(
                    div()
                        .w(px(96.))
                        .pt(px(8.))
                        .text_color(t.color.neutral_foreground_3)
                        .child("states"),
                )
                .child(
                    div().w(px(220.)).child(
                        Textarea::new(&self.textareas[9])
                            .disabled(true)
                            .size(ControlSize::Medium)
                            .accessibility_label("disabled textarea"),
                    ),
                )
                .child(
                    div().w(px(220.)).child(
                        Textarea::new(&self.textareas[10])
                            .invalid(true)
                            .size(ControlSize::Medium)
                            .accessibility_label("invalid textarea"),
                    ),
                ),
        );

        let field_section = div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .p(px(16.))
            .child(
                div().w(px(300.)).child(
                    Field::new("Display name")
                        .required(true)
                        .hint("Shown to other people.")
                        .child(
                            TextInput::new(&self.inputs[15]).accessibility_label("Display name"),
                        ),
                ),
            )
            .child(
                div().w(px(300.)).child(
                    Field::new("Email")
                        .required(true)
                        .validation_message(ValidationMessage::Error(
                            "Enter a valid email address".into(),
                        ))
                        .child(
                            TextInput::new(&self.inputs[16])
                                .invalid(true)
                                .accessibility_label("Email"),
                        ),
                ),
            )
            .child(
                div().w(px(460.)).child(
                    Field::new("Storage")
                        .horizontal()
                        .hint("Used across all workspaces.")
                        .child(TextInput::new(&self.inputs[17]).accessibility_label("Storage")),
                ),
            )
            .child(
                div().w(px(300.)).child(
                    Field::new("Disabled field").disabled(true).child(
                        TextInput::new(&self.inputs[2])
                            .disabled(true)
                            .accessibility_label("Disabled field"),
                    ),
                ),
            );

        let header = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .p(px(16.))
            .border_b_1()
            .border_color(t.color.neutral_stroke_1)
            .child(
                div()
                    .id("theme-light")
                    .px(px(12.))
                    .py(px(5.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(t.color.neutral_stroke_1)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.apply(Appearance::Light, cx)))
                    .child("Light"),
            )
            .child(
                div()
                    .id("theme-dark")
                    .px(px(12.))
                    .py(px(5.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(t.color.neutral_stroke_1)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.apply(Appearance::Dark, cx)))
                    .child("Dark"),
            )
            .child(
                div()
                    .id("theme-hc")
                    .px(px(12.))
                    .py(px(5.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(t.color.neutral_stroke_1)
                    .cursor_pointer()
                    .on_click(
                        cx.listener(|this, _, _, cx| this.apply(Appearance::HighContrast, cx)),
                    )
                    .child("HC"),
            )
            .child(
                div()
                    .id("font-mode")
                    .px(px(12.))
                    .py(px(5.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(t.color.neutral_stroke_1)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_font(cx)))
                    .child("Font"),
            )
            .child(
                div()
                    .ml(px(8.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(format!(
                        "appearance: {:?}   clicks: {}   font: {} ({:?})",
                        theme.appearance(),
                        self.clicks,
                        theme.font_family(),
                        theme.font_mode()
                    )),
            );

        let selection_section = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(16.))
            .child(
                Checkbox::new("cb-a")
                    .checked(self.checkbox_a)
                    .label("Send me product updates")
                    .on_change(cx.listener(|this, value, _, cx| {
                        this.checkbox_a = *value;
                        cx.notify();
                    })),
            )
            .child(
                // Uncontrolled: keeps its own state, no caller state needed.
                Checkbox::new("cb-b")
                    .default_checked(true)
                    .label("Uncontrolled (starts checked)"),
            )
            .child(
                Checkbox::new("cb-c")
                    .checked(true)
                    .disabled(true)
                    .label("Disabled and checked"),
            )
            .child(
                Radio::new("radio-0")
                    .checked(self.radio_choice == 0)
                    .label("Option A")
                    .on_change(cx.listener(|this, _, _, cx| {
                        this.radio_choice = 0;
                        cx.notify();
                    })),
            )
            .child(
                Radio::new("radio-1")
                    .checked(self.radio_choice == 1)
                    .label("Option B (selected)")
                    .on_change(cx.listener(|this, _, _, cx| {
                        this.radio_choice = 1;
                        cx.notify();
                    })),
            )
            .child(
                Radio::new("radio-2")
                    .checked(false)
                    .disabled(true)
                    .label("Option C (disabled)"),
            );

        let switch_section = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(16.))
            .child(
                Switch::new("sw-a")
                    .checked(self.switch_a)
                    .label("Enable notifications")
                    .on_change(cx.listener(|this, value, _, cx| {
                        this.switch_a = *value;
                        cx.notify();
                    })),
            )
            .child(
                Switch::new("sw-b")
                    .checked(self.switch_b)
                    .label("Compact mode")
                    .on_change(cx.listener(|this, value, _, cx| {
                        this.switch_b = *value;
                        cx.notify();
                    })),
            )
            .child(
                // Uncontrolled: the switch keeps its own state, no caller state needed.
                Switch::new("sw-small")
                    .size(ControlSize::Small)
                    .default_checked(true)
                    .label("Small switch"),
            )
            .child(
                Switch::new("sw-disabled")
                    .checked(true)
                    .disabled(true)
                    .label("Disabled switch"),
            );

        let misc_section = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(16.))
            .child(
                div()
                    .flex()
                    .gap(px(16.))
                    .child(Link::new("link-1", "Default link"))
                    .child(
                        Link::new("link-2", "Subtle link")
                            .appearance(fluent_ui::LinkAppearance::Subtle),
                    )
                    .child(Link::new("link-3", "Disabled link").disabled(true)),
            )
            .child(Divider::new())
            .child(
                Divider::new()
                    .appearance(fluent_ui::DividerAppearance::Brand)
                    .inset(px(24.)),
            )
            .child(Divider::new().appearance(fluent_ui::DividerAppearance::Subtle))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .child(
                        Button::new("tooltip-btn")
                            .tooltip("Saves the current document")
                            .accessibility_label("Hover for tooltip")
                            .child("Hover for tooltip"),
                    )
                    // Static preview of the surface, since screenshots cannot hover.
                    .child(TooltipSurface::new("Saves the current document")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    // Opened by default so a screenshot can show the panel.
                    .child(
                        Popover::new("popover-demo")
                            .default_open(true)
                            .trigger(
                                Button::new("popover-trigger")
                                    .accessibility_label("Open popover")
                                    .child("Open popover"),
                            )
                            .content("Popover content"),
                    )
                    // Reproduction for popover positioning inside a scroll
                    // container: scroll this column to the middle row.
                    .child(
                        div()
                            .id("popover-scroll-probe")
                            .h(px(120.))
                            .w(px(260.))
                            .overflow_y_scroll()
                            .border_1()
                            .border_color(Hsla::black().opacity(0.12))
                            .rounded(px(4.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.))
                                    .p(px(8.))
                                    .child(div().h(px(36.)).child("spacer 1"))
                                    .child(
                                        Popover::new("popover-scroll-demo")
                                            .default_open(true)
                                            .trigger(
                                                Button::new("popover-scroll-trigger")
                                                    .accessibility_label("Scroll me")
                                                    .child("Scroll me"),
                                            )
                                            .content("Anchored panel"),
                                    )
                                    .child(div().h(px(60.)).child("spacer 2")),
                            ),
                    )
                    .child(
                        Menu::new("menu-panel")
                            .header(MenuHeader::new("Actions"))
                            .item(MenuItem::new("menu-edit", "Edit").icon(FluentIcon::Settings))
                            .item(MenuItem::new("menu-dup", "Duplicate"))
                            .separator()
                            .header(MenuHeader::new("View"))
                            .item(MenuItem::new("menu-wrap", "Word wrap").checked(true))
                            .item(MenuItem::new("menu-minimap", "Minimap").checked(false))
                            .separator()
                            .header(MenuHeader::new("Sort by"))
                            .item(MenuItem::new("menu-sort-name", "Name").selected(true))
                            .item(MenuItem::new("menu-sort-date", "Date").selected(false))
                            .separator()
                            .item(MenuItem::new("menu-arch", "Archive").disabled(true))
                            .item(MenuItem::new("menu-del", "Delete").danger(true)),
                    ),
            )
            .child(
                div().p(px(16.)).child(
                    MenuDropdown::new(
                        "menu-dropdown",
                        Button::new("menu-dropdown-trigger")
                            .accessibility_label("Open menu")
                            .child("Open menu"),
                        Menu::new("menu-dropdown-panel")
                            .item(MenuItem::new("dd-rename", "Rename").icon(FluentIcon::Settings))
                            .item(MenuItem::new("dd-copy", "Copy"))
                            .separator()
                            .item(MenuItem::new("dd-delete", "Delete").danger(true)),
                    )
                    .default_open(true),
                ),
            );

        // Tabs: one horizontal strip with a disabled tab, one vertical strip. The
        // selection is controlled so the demo exercises the caller-owned path.
        let tabs_section = div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .p(px(16.))
            .child(
                TabList::new("gallery-tabs")
                    .track_focus(&self.tab_focus)
                    .selected(self.tab_index)
                    .on_change(cx.listener(|this, index, _, cx| {
                        this.tab_index = *index;
                        cx.notify();
                    }))
                    .tab(TabItem::new("gallery-tab-overview", "Overview"))
                    .tab(TabItem::new("gallery-tab-activity", "Activity"))
                    .tab(TabItem::new("gallery-tab-archived", "Archived").disabled(true))
                    .tab(TabItem::new("gallery-tab-settings", "Settings")),
            )
            .child(
                div().w(px(200.)).child(
                    // The two strips have different tab counts, so they keep
                    // separate selections; sharing one index made the shorter
                    // strip jump back to its first tab.
                    TabList::new("gallery-tabs-vertical")
                        .track_focus(&self.vertical_tab_focus)
                        .vertical()
                        .selected(self.vertical_tab_index)
                        .on_change(cx.listener(|this, index, _, cx| {
                            this.vertical_tab_index = *index;
                            cx.notify();
                        }))
                        .tab(TabItem::new("gallery-vtab-overview", "Overview"))
                        .tab(TabItem::new("gallery-vtab-activity", "Activity"))
                        .tab(TabItem::new("gallery-vtab-settings", "Settings")),
                ),
            );

        let feedback_section = div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .p(px(16.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(Badge::new("Brand"))
                    .child(Badge::new("Danger").appearance(BadgeAppearance::Danger))
                    .child(Badge::new("Success").appearance(BadgeAppearance::Success))
                    .child(Badge::new("Warning").appearance(BadgeAppearance::Warning))
                    .child(Badge::new("12").size(BadgeSize::Large))
                    .child(Badge::dot().appearance(BadgeAppearance::Danger)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .w(px(320.))
                    .child(ProgressBar::new(25.))
                    .child(ProgressBar::new(60.))
                    .child(ProgressBar::new(100.).appearance(ProgressAppearance::Success))
                    .child(ProgressBar::new(40.).appearance(ProgressAppearance::Error))
                    .child(ProgressBar::new(0.).indeterminate()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(Avatar::new("Ada Lovelace").size(AvatarSize::Small))
                    .child(Avatar::new("Grace Hopper"))
                    .child(Avatar::new("Alan Turing").size(AvatarSize::Large))
                    .child(Avatar::new("Katherine Johnson").size(AvatarSize::ExtraLarge)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .child(Spinner::new().size(SpinnerSize::Tiny))
                    .child(Spinner::new().size(SpinnerSize::ExtraSmall))
                    .child(Spinner::new().size(SpinnerSize::Small))
                    .child(Spinner::new().size(SpinnerSize::Medium))
                    .child(
                        Spinner::new()
                            .size(SpinnerSize::Medium)
                            .label("Loading your files"),
                    ),
            );

        let dialog_section = {
            let weak = cx.entity().downgrade();
            let open_dialog = {
                let weak = weak.clone();
                move |open: bool, cx: &mut App| {
                    weak.update(cx, |this, cx| {
                        this.dialog_open = open;
                        cx.notify();
                    })
                    .ok();
                }
            };
            let open_alert = {
                let weak = weak.clone();
                move |open: bool, cx: &mut App| {
                    weak.update(cx, |this, cx| {
                        this.alert_open = open;
                        cx.notify();
                    })
                    .ok();
                }
            };
            let close_dialog = open_dialog.clone();
            let close_alert = open_alert.clone();
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .p(px(16.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            Button::new("gallery-dialog-open")
                                .appearance(ButtonAppearance::Primary)
                                .on_click(move |_, _, cx| open_dialog(true, cx))
                                .accessibility_label("Open dialog")
                                .child("Open dialog"),
                        )
                        .child(
                            Button::new("gallery-alert-open")
                                .appearance(ButtonAppearance::Secondary)
                                .on_click(move |_, _, cx| open_alert(true, cx))
                                .accessibility_label("Open alert dialog")
                                .child("Open alert dialog"),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(t.color.neutral_foreground_3)
                                .child(
                                    "Esc / scrim dismiss the first; the alert requires a choice",
                                ),
                        ),
                )
                .child(
                    Dialog::new("gallery-dialog")
                        .title("Edit details")
                        .open(self.dialog_open)
                        .on_open_change({
                            let close = close_dialog.clone();
                            move |open, _, _, cx| close(open, cx)
                        })
                        .child("Dialogs dim the page, take focus and return it on close.")
                        .footer_child(
                            Button::new("gallery-dialog-cancel")
                                // Fluent's dialog dismiss action is a bordered
                                // secondary button, not a borderless one.
                                .appearance(ButtonAppearance::Secondary)
                                .accessibility_label("Cancel")
                                .on_click({
                                    let close = close_dialog.clone();
                                    move |_, _, cx| close(false, cx)
                                })
                                .child("Cancel"),
                        )
                        .footer_child(
                            Button::new("gallery-dialog-save")
                                .appearance(ButtonAppearance::Primary)
                                .accessibility_label("Save")
                                .on_click({
                                    let close = close_dialog.clone();
                                    move |_, _, cx| close(false, cx)
                                })
                                .child("Save"),
                        ),
                )
                .child(
                    AlertDialog::new("gallery-alert")
                        .title("Discard draft?")
                        .open(self.alert_open)
                        .on_open_change({
                            let close = close_alert.clone();
                            move |open, _, _, cx| close(open, cx)
                        })
                        .child(
                            "An alert dialog has no Escape or scrim dismissal: choose an action.",
                        )
                        .footer_child(
                            Button::new("gallery-alert-keep")
                                .appearance(ButtonAppearance::Secondary)
                                .accessibility_label("Keep editing")
                                .on_click({
                                    let close = close_alert.clone();
                                    move |_, _, cx| close(false, cx)
                                })
                                .child("Keep editing"),
                        )
                        .footer_child(
                            Button::new("gallery-alert-discard")
                                .appearance(ButtonAppearance::Secondary)
                                .text_color(t.color.status_danger_foreground_1)
                                .accessibility_label("Discard")
                                .on_click({
                                    let close = close_alert.clone();
                                    move |_, _, cx| close(false, cx)
                                })
                                .child("Discard"),
                        ),
                )
        };

        let sheet_section = {
            let weak = cx.entity().downgrade();
            let open_sheet = {
                let weak = weak.clone();
                move |side: SheetSide, cx: &mut App| {
                    weak.update(cx, |this, cx| {
                        this.sheet_side = Some(side);
                        cx.notify();
                    })
                    .ok();
                }
            };
            let close_sheet = {
                let weak = weak.clone();
                move |cx: &mut App| {
                    weak.update(cx, |this, cx| {
                        this.sheet_side = None;
                        cx.notify();
                    })
                    .ok();
                }
            };
            let mut openers = div().flex().items_center().gap(px(12.));
            for (side, id, label) in [
                (SheetSide::Right, "sheet-open-right", "Open right"),
                (SheetSide::Left, "sheet-open-left", "Open left"),
                (SheetSide::Top, "sheet-open-top", "Open top"),
                (SheetSide::Bottom, "sheet-open-bottom", "Open bottom"),
            ] {
                let open_sheet = open_sheet.clone();
                openers = openers.child(
                    Button::new(id)
                        .appearance(ButtonAppearance::Secondary)
                        .on_click(move |_, _, cx| open_sheet(side, cx))
                        .accessibility_label(label)
                        .child(label),
                );
            }
            let close_from_scrim = close_sheet.clone();
            let close_from_button = close_sheet.clone();
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .p(px(16.))
                .child(openers.child(
                    div()
                        .text_size(px(12.))
                        .text_color(t.color.neutral_foreground_3)
                        .child("Sheets reuse the dialog scrim and focus trap; Esc or a scrim press closes."),
                ))
                .child(
                    Sheet::new("gallery-sheet")
                        .side(self.sheet_side.unwrap_or(SheetSide::Right))
                        .open(self.sheet_side.is_some())
                        .on_open_change(move |_, _, cx| close_from_scrim(cx))
                        .title("Filters")
                        .child(
                            "The drawer slides in from the selected edge, dims the page and traps Tab until it closes.",
                        )
                        .footer_child(
                            Button::new("gallery-sheet-cancel")
                                .appearance(ButtonAppearance::Secondary)
                                .on_click(move |_, _, cx| close_from_button(cx))
                                .accessibility_label("Cancel")
                                .child("Cancel"),
                        )
                        .footer_child(
                            Button::new("gallery-sheet-apply")
                                .appearance(ButtonAppearance::Primary)
                                .on_click({
                                    let close_sheet = close_sheet.clone();
                                    move |_, _, cx| close_sheet(cx)
                                })
                                .accessibility_label("Apply")
                                .child("Apply"),
                        ),
                )
        };

        // `FLUENT_GALLERY_SECTION` narrows the gallery to one section; useful for
        // screenshots and pixel baselines that would otherwise be below the fold.
        let want = |name: &str| match std::env::var("FLUENT_GALLERY_SECTION") {
            Ok(value) => value == name,
            Err(_) => true,
        };
        let mut sections: Vec<AnyElement> = Vec::new();
        if want("buttons") {
            sections.push(matrix.into_any_element());
        }
        if want("inputs") {
            sections.push(input_matrix.into_any_element());
        }
        if want("textareas") {
            sections.push(textarea_matrix.into_any_element());
        }
        if want("fields") {
            sections.push(field_section.into_any_element());
        }
        if want("selection") {
            sections.push(selection_section.into_any_element());
        }
        if want("switch") {
            sections.push(switch_section.into_any_element());
        }
        if want("misc") {
            sections.push(misc_section.into_any_element());
        }
        if want("feedback") {
            sections.push(feedback_section.into_any_element());
        }
        if want("tabs") {
            sections.push(tabs_section.into_any_element());
        }
        if want("dialog") {
            sections.push(dialog_section.into_any_element());
        }
        if want("sheet") {
            sections.push(sheet_section.into_any_element());
        }
        FluentRoot::new()
            .flex()
            .flex_col()
            // Placeholder artwork until the gallery has a real app logo; it also
            // exercises the leading slot that TitleBar reserves for one.
            .child(TitleBar::new("gallery-titlebar", "Fluent Gallery").icon(FluentIcon::Settings))
            .child(header)
            .child(
                div()
                    .id("matrix-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(sections)
                    .child(
                        div()
                            .p(px(16.))
                            .flex()
                            .gap(px(8.))
                            .child(swatch(t.color.brand_background))
                            .child(swatch(t.color.neutral_background_1_hover))
                            .child(swatch(t.color.neutral_stroke_accessible))
                            .child(swatch(t.color.stroke_focus_2)),
                    ),
            )
            .into_any_element()
    }
}

fn main() {
    if std::env::var_os("FLUENT_GALLERY_LIST_FIXTURES").is_some() {
        for fixture in fixtures::FIXTURES {
            println!("{}\t{}\t{}", fixture.name, fixture.width, fixture.height);
        }
        return;
    }
    fluent_ui::application().run(|cx: &mut App| {
        fluent_ui::init(cx);

        if let Some(value) = std::env::var_os("FLUENT_GALLERY_APPEARANCE") {
            let appearance = match value.to_string_lossy().to_ascii_lowercase().as_str() {
                "dark" => Appearance::Dark,
                "hc" | "high-contrast" => Appearance::HighContrast,
                _ => Appearance::Light,
            };
            FluentTheme::for_appearance(appearance).install(cx);
        }

        // A fixture pins the window to its canvas so the PNG is reproducible.
        let fixture_size = std::env::var("FLUENT_GALLERY_FIXTURE")
            .ok()
            .and_then(|name| fixtures::size(&name));
        let (width, height) = fixture_size.unwrap_or_else(|| {
            std::env::var("FLUENT_GALLERY_SIZE")
                .ok()
                .and_then(|value| {
                    let (w, h) = value.split_once('x')?;
                    Some((w.trim().parse::<f32>().ok()?, h.trim().parse::<f32>().ok()?))
                })
                .unwrap_or((900.0, 640.0))
        });
        let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..TitleBar::window_options("Fluent Gallery")
                },
                |window, cx| cx.new(|cx| Gallery::new(window, cx)),
            )
            .expect("failed to open window");

        #[cfg(feature = "screenshot")]
        if let Some(path) = std::env::var_os("FLUENT_GALLERY_SCREENSHOT") {
            let path = std::path::PathBuf::from(path);
            let executor = cx.background_executor().clone();
            cx.spawn(async move |cx| {
                // Give the first frames (layout, text shaping, SVG parse) time.
                executor.timer(Duration::from_millis(1200)).await;
                let captured =
                    cx.update(|cx| window.update(cx, |_, window, _| window.render_to_image()));
                match captured {
                    Ok(Ok(image)) => match image.save(&path) {
                        Ok(()) => eprintln!("screenshot written to {}", path.display()),
                        Err(error) => eprintln!("screenshot save failed: {error}"),
                    },
                    Ok(Err(error)) => eprintln!("render_to_image failed: {error:#}"),
                    Err(error) => eprintln!("window.update failed: {error:#}"),
                }
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
        #[cfg(not(feature = "screenshot"))]
        let _ = &window;

        if std::env::var_os("FLUENT_GALLERY_SMOKE").is_some() {
            let executor = cx.background_executor().clone();
            cx.spawn(async move |cx| {
                executor.timer(Duration::from_millis(1500)).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }

        cx.activate(true);
    });
}

#[cfg(test)]
mod tab_stop_tests {
    //! Tab-order regression for the two tab strips in the *default* gallery
    //! view (not a fixture). On the pre-fix code neither strip was wired to a
    //! focus handle, so the strip was skipped by Tab and its arrows only worked
    //! after programmatic focus.
    use super::*;
    use fluent_ui::gpui::{
        Entity, KeyDownEvent, Keystroke, PlatformInput, TestAppContext, WindowHandle,
    };

    type Handle = WindowHandle<Gallery>;

    fn settle(cx: &mut TestAppContext, window: Handle) {
        for _ in 0..2 {
            cx.update_window(window.into(), |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .unwrap();
            cx.run_until_parked();
        }
    }

    fn open(cx: &mut TestAppContext) -> (Handle, Entity<Gallery>) {
        cx.update(fluent_ui::init);
        let window = cx.open_window(size(px(1400.), px(900.)), |window, cx| {
            Gallery::new(window, cx)
        });
        cx.run_until_parked();
        let app = cx
            .update_window(window.into(), |root, _, _| {
                root.downcast::<Gallery>()
                    .expect("the gallery view is the root")
            })
            .unwrap();
        settle(cx, window);
        (window, app)
    }

    fn focus_strip(cx: &mut TestAppContext, window: Handle, focus: &FocusHandle) {
        cx.update_window(window.into(), |_, window, cx| focus.focus(window, cx))
            .unwrap();
        settle(cx, window);
    }

    fn focused(cx: &mut TestAppContext, window: Handle) -> Option<FocusHandle> {
        cx.update_window(window.into(), |_, window, cx| window.focused(cx))
            .unwrap()
    }

    fn press(cx: &mut TestAppContext, window: Handle, key: &'static str) {
        let keystroke = Keystroke::parse(key).unwrap();
        cx.update_window(window.into(), move |_, window, cx| {
            window.dispatch_event(
                PlatformInput::KeyDown(KeyDownEvent {
                    keystroke,
                    is_held: false,
                    prefer_character_input: false,
                }),
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
        settle(cx, window);
    }

    #[gpui::test]
    fn default_tab_strips_are_stops_and_own_arrow_and_home_end(cx: &mut TestAppContext) {
        let (window, app) = open(cx);
        let (horizontal, vertical) = cx.update(|cx| {
            let gallery = app.read(cx);
            (
                gallery.tab_focus.clone(),
                gallery.vertical_tab_focus.clone(),
            )
        });

        focus_strip(cx, window, &horizontal);
        assert_eq!(focused(cx, window), Some(horizontal.clone()));

        // Enter the strip from the real stop before it instead of relying only
        // on programmatic focus: Shift+Tab out, then Tab back in. This proves
        // the horizontal strip is registered in the page's tab order, not just
        // that a handle exists. On the pre-fix code the first Shift+Tab lands on
        // some other stop and the following Tab never reaches this strip.
        press(cx, window, "shift-tab");
        let before = focused(cx, window).expect("a real tab stop precedes the strips");
        assert!(
            before != horizontal && before != vertical,
            "Shift+Tab from the horizontal strip must land outside the tab strips"
        );
        press(cx, window, "tab");
        assert_eq!(
            focused(cx, window),
            Some(horizontal.clone()),
            "Tab from the preceding real stop must enter the horizontal strip"
        );

        // The consumer path: the strip's own handlers update the caller-owned
        // selection, and disabled tabs are skipped.
        press(cx, window, "right");
        assert_eq!(cx.update(|cx| app.read(cx).tab_index), 1);
        press(cx, window, "right");
        assert_eq!(cx.update(|cx| app.read(cx).tab_index), 3);
        press(cx, window, "home");
        assert_eq!(cx.update(|cx| app.read(cx).tab_index), 0);
        press(cx, window, "end");
        assert_eq!(cx.update(|cx| app.read(cx).tab_index), 3);

        // Both default strips are real Tab stops; the vertical one follows the
        // horizontal one in the page's paint order.
        press(cx, window, "tab");
        assert_eq!(
            focused(cx, window),
            Some(vertical.clone()),
            "Tab must leave the horizontal strip and stop on the vertical strip"
        );
        press(cx, window, "down");
        assert_eq!(cx.update(|cx| app.read(cx).vertical_tab_index), 1);

        press(cx, window, "tab");
        let after = focused(cx, window).expect("Tab leaves the strips for the next stop");
        assert!(
            after != horizontal && after != vertical,
            "Tab must continue past the strips instead of wrapping inside them"
        );
        press(cx, window, "shift-tab");
        assert_eq!(
            focused(cx, window),
            Some(vertical),
            "Shift+Tab returns to the vertical strip"
        );
        press(cx, window, "shift-tab");
        assert_eq!(
            focused(cx, window),
            Some(horizontal),
            "Shift+Tab from the vertical strip returns to the horizontal strip"
        );
    }
}
