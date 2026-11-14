//! Assert the colour actually submitted for the SVG icon.
use super::*;
use crate::FluentRoot;
use gpui::{Context, Render, TestAppContext, size};

struct View {
    disabled: bool,
    inherited: bool,
}
impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let child = if self.inherited {
            div()
                .text_color(gpui::rgb(0x12ab34))
                .child(Icon::new(FluentIcon::Add))
                .into_any_element()
        } else {
            Button::new("icon-probe")
                .appearance(ButtonAppearance::Primary)
                .disabled(self.disabled)
                .icon(FluentIcon::Add)
                .icon_only()
                .into_any_element()
        };
        FluentRoot::new().child(child)
    }
}

#[gpui::test]
fn a_disabled_primary_icon_uses_the_disabled_foreground(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    let h = cx.open_window(size(px(240.), px(160.)), |_, _| View {
        disabled: true,
        inherited: false,
    });
    cx.run_until_parked();
    let colors = cx
        .update_window(h.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            window
                .painted_monochrome_sprites()
                .into_iter()
                .map(|s| s.color)
                .collect::<Vec<_>>()
        })
        .unwrap();
    let expected = button::resolve(
        &FluentTheme::web_light(),
        &ButtonProps {
            disabled: true,
            appearance: ButtonAppearance::Primary,
            ..Default::default()
        },
    )
    .disabled
    .foreground;
    assert_eq!(
        colors,
        vec![expected],
        "the rendered SVG must use the same disabled foreground as the text"
    );
}

#[gpui::test]
fn an_icon_without_an_override_inherits_the_surrounding_colour(cx: &mut TestAppContext) {
    let h = cx.open_window(size(px(240.), px(160.)), |_, _| View {
        disabled: false,
        inherited: true,
    });
    cx.run_until_parked();
    let colors = cx
        .update_window(h.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            window
                .painted_monochrome_sprites()
                .into_iter()
                .map(|s| s.color)
                .collect::<Vec<_>>()
        })
        .unwrap();
    assert_eq!(colors, vec![gpui::Hsla::from(gpui::rgb(0x12ab34))]);
}
