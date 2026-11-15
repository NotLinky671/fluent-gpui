use super::*;

impl Gallery {
    pub(crate) fn dialog_section(
        &mut self,
        cx: &mut Context<Self>,
        theme: &FluentTheme,
    ) -> AnyElement {
        let t = theme.tokens();
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

        dialog_section.into_any_element()
    }

    pub(crate) fn sheet_section(
        &mut self,
        cx: &mut Context<Self>,
        theme: &FluentTheme,
    ) -> AnyElement {
        let t = theme.tokens();
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

        sheet_section.into_any_element()
    }

    /// The right-click surface. `open` stays on the page so the demo exercises
    /// the controlled path, and the menu is the same `Menu` the other demos
    /// use: only the trigger and the placement are different.
    pub(crate) fn context_menu_section(
        &mut self,
        cx: &mut Context<Self>,
        theme: &FluentTheme,
    ) -> AnyElement {
        let t = theme.tokens();
        let weak = cx.entity().downgrade();
        let on_change = move |next: bool, _window: &mut Window, cx: &mut App| {
            weak.update(cx, |this, cx| {
                this.context_menu_open = next;
                cx.notify();
            })
            .ok();
        };

        let menu = Menu::new("gallery-context-menu")
            .item(MenuItem::new("gallery-context-cut", "Cut").secondary("Ctrl+X"))
            .item(MenuItem::new("gallery-context-copy", "Copy").secondary("Ctrl+C"))
            .item(MenuItem::new("gallery-context-paste", "Paste").secondary("Ctrl+V"))
            .separator()
            .header(MenuHeader::new("Arrange"))
            .item(MenuItem::new("gallery-context-front", "Bring to front"))
            .item(MenuItem::new("gallery-context-delete", "Delete").danger(true));

        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(16.))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(t.color.neutral_foreground_2)
                    .child("ContextMenu - right click inside the outline"),
            )
            .child(
                div()
                    .relative()
                    .w(px(280.))
                    .h(px(160.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(t.color.neutral_stroke_1)
                    .bg(t.color.neutral_background_2)
                    .child(
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.))
                            .text_color(t.color.neutral_foreground_2)
                            .child("Right click"),
                    )
                    .child(
                        ContextMenu::new("gallery-context", menu)
                            .region(div().size_full())
                            .open(self.context_menu_open)
                            .on_open_change(on_change),
                    ),
            )
            .into_any_element()
    }
}
