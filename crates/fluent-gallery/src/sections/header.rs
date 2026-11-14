use super::*;

impl Gallery {
    pub(crate) fn header(&mut self, cx: &mut Context<Self>, theme: &FluentTheme) -> AnyElement {
        let t = theme.tokens();
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

        header.into_any_element()
    }
}
