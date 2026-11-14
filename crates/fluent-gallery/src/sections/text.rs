use super::*;

impl Gallery {
    pub(crate) fn input_matrix(
        &mut self,
        cx: &mut Context<Self>,
        theme: &FluentTheme,
    ) -> AnyElement {
        let t = theme.tokens();
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
        input_matrix.into_any_element()
    }

    pub(crate) fn textarea_matrix(
        &mut self,
        cx: &mut Context<Self>,
        theme: &FluentTheme,
    ) -> AnyElement {
        let t = theme.tokens();
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

        textarea_matrix.into_any_element()
    }

    pub(crate) fn field_section(&mut self) -> AnyElement {
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

        field_section.into_any_element()
    }
}
