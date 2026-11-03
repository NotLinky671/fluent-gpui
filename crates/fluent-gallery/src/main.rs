//! Component gallery: the manual review surface for `fluent-ui`.
//!
//! - `FLUENT_GALLERY_SMOKE=1` opens the window and quits, for CI boot checks.
//! - `FLUENT_GALLERY_SCREENSHOT=<path>` (with `--features screenshot`) writes a
//!   PNG of the first frame.

mod fixtures;

use std::time::Duration;

use fluent_ui::gpui::{
    AnyElement, App, Bounds, ClickEvent, Context, Hsla, Render, StatefulInteractiveElement,
    Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions, div, px, size,
};
use fluent_ui::gpui_base::input::{InputState, TextareaState};
use fluent_ui::prelude::*;
use fluent_ui::{
    Appearance, Avatar, AvatarSize, Badge, BadgeAppearance, BadgeSize, Button, ButtonAppearance,
    ButtonShape, Checkbox, ControlSize, Divider, Field, FluentIcon, FluentRoot, FluentTheme,
    FontMode, InputAppearance, Link, Menu, MenuDropdown, MenuHeader, MenuItem, Popover,
    ProgressAppearance, ProgressBar, Radio, Spinner, SpinnerSize, Switch, TextInput, Textarea,
    TextareaAppearance, TooltipSurface, ValidationMessage,
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
            shapes = shapes.child(
                Button::new(("shape", index))
                    .shape(shape)
                    .on_click(handler())
                    .child(match shape {
                        ButtonShape::Rounded => "rounded",
                        ButtonShape::Circular => "circular",
                        ButtonShape::Square => "square",
                    }),
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
                    .child(label),
            );
            index += 1;
        }
        icon_row = icon_row.child(
            Button::new(("icon", index))
                .appearance(ButtonAppearance::Primary)
                .icon_after(FluentIcon::ChevronDown)
                .on_click(handler())
                .child("Options"),
        );
        index += 1;
        for icon in [
            FluentIcon::Add,
            FluentIcon::MoreHorizontal,
            FluentIcon::Dismiss,
        ] {
            icon_row = icon_row.child(
                Button::new(("icon-only", index))
                    .appearance(ButtonAppearance::Subtle)
                    .icon(icon)
                    .icon_only()
                    .on_click(handler()),
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
            let mut row = div().flex().items_center().gap(px(12.)).child(
                div()
                    .w(px(96.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(match appearance {
                        InputAppearance::Outline => "outline",
                        InputAppearance::Underline => "underline",
                        InputAppearance::FilledDarker => "filled-darker",
                        InputAppearance::FilledLighter => "filled-lighter",
                    }),
            );
            if matches!(appearance, InputAppearance::FilledLighter) {
                row = row.bg(FluentTheme::global(cx).tokens().color.neutral_background_2);
            }
            for size in SIZES {
                row = row.child(
                    div().w(px(220.)).child(
                        TextInput::new(&self.inputs[input_index])
                            .appearance(appearance)
                            .size(size),
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
                        .size(ControlSize::Medium),
                ),
            )
            .child(
                div().w(px(220.)).child(
                    TextInput::new(&self.inputs[13])
                        .invalid(true)
                        .size(ControlSize::Medium),
                ),
            )
            .child(
                div().w(px(220.)).child(
                    TextInput::new(&self.inputs[14])
                        .content_before(fluent_ui::Icon::new(FluentIcon::Search).size(px(16.)))
                        .size(ControlSize::Medium),
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
            let mut row = div().flex().items_start().gap(px(12.)).child(
                div()
                    .w(px(96.))
                    .pt(px(8.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(match appearance {
                        TextareaAppearance::Outline => "outline",
                        TextareaAppearance::Filled => "filled",
                        TextareaAppearance::FilledDarker => "filled-darker",
                        TextareaAppearance::FilledLighter => "filled-lighter",
                    }),
            );
            if matches!(appearance, TextareaAppearance::FilledLighter) {
                row = row.bg(FluentTheme::global(cx).tokens().color.neutral_background_2);
            }
            for size in SIZES {
                row = row.child(
                    div().w(px(220.)).child(
                        Textarea::new(&self.textareas[textarea_index])
                            .appearance(appearance)
                            .size(size),
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
                            .size(ControlSize::Medium),
                    ),
                )
                .child(
                    div().w(px(220.)).child(
                        Textarea::new(&self.textareas[10])
                            .invalid(true)
                            .size(ControlSize::Medium),
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
                        .child(TextInput::new(&self.inputs[15])),
                ),
            )
            .child(
                div().w(px(300.)).child(
                    Field::new("Email")
                        .required(true)
                        .validation_message(ValidationMessage::Error(
                            "Enter a valid email address".into(),
                        ))
                        .child(TextInput::new(&self.inputs[16]).invalid(true)),
                ),
            )
            .child(
                div().w(px(460.)).child(
                    Field::new("Storage")
                        .horizontal()
                        .hint("Used across all workspaces.")
                        .child(TextInput::new(&self.inputs[17])),
                ),
            )
            .child(
                div().w(px(300.)).child(
                    Field::new("Disabled field")
                        .disabled(true)
                        .child(TextInput::new(&self.inputs[2]).disabled(true)),
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
                            .trigger(Button::new("popover-trigger").child("Open popover"))
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
                        Button::new("menu-dropdown-trigger").child("Open menu"),
                        Menu::new("menu-dropdown-panel")
                            .item(MenuItem::new("dd-rename", "Rename").icon(FluentIcon::Settings))
                            .item(MenuItem::new("dd-copy", "Copy"))
                            .separator()
                            .item(MenuItem::new("dd-delete", "Delete").danger(true)),
                    )
                    .default_open(true),
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
                    .child(Spinner::new().size(SpinnerSize::Small))
                    .child(Spinner::new().size(SpinnerSize::Medium))
                    .child(Spinner::new().size(SpinnerSize::Large))
                    .child(
                        Spinner::new()
                            .size(SpinnerSize::Medium)
                            .label("Loading your files"),
                    ),
            );

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
        FluentRoot::new()
            .flex()
            .flex_col()
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
                    titlebar: Some(TitlebarOptions {
                        title: Some("Fluent Gallery".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
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
