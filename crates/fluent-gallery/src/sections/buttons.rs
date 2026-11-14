use super::*;

impl Gallery {
    pub(crate) fn matrix(&mut self, cx: &mut Context<Self>, theme: &FluentTheme) -> AnyElement {
        let t = theme.tokens();
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

        matrix.into_any_element()
    }
}
