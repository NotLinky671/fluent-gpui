use super::*;

impl Gallery {
    pub(crate) fn selection_section(&mut self, cx: &mut Context<Self>) -> AnyElement {
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

        selection_section.into_any_element()
    }

    pub(crate) fn switch_section(&mut self, cx: &mut Context<Self>) -> AnyElement {
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

        switch_section.into_any_element()
    }

    pub(crate) fn misc_section(&mut self) -> AnyElement {
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

        misc_section.into_any_element()
    }

    pub(crate) fn tabs_section(&mut self, cx: &mut Context<Self>) -> AnyElement {
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

        tabs_section.into_any_element()
    }

    pub(crate) fn feedback_section(&mut self) -> AnyElement {
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

        feedback_section.into_any_element()
    }
}
