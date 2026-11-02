//! Fluent 2 `Menu` metrics and colours.

use crate::gpui_compat::{Hsla, Pixels, px};
use fluent_theme::FluentTheme;

/// What the caller can vary on a menu item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuItemProps {
    /// Destructive action: red label, red hover.
    pub danger: bool,
    pub disabled: bool,
}

/// Resolved appearance of one menu item.
#[derive(Clone, Copy, Debug)]
pub struct MenuItemSpec {
    pub height: Pixels,
    pub padding_x: Pixels,
    pub radius: Pixels,
    pub gap: Pixels,
    pub icon_size: Pixels,
    /// Leading slot reserved for an icon or a check mark.
    ///
    /// Fluent keeps this slot in the layout even when a row has neither, so
    /// labels line up down the menu instead of shifting row by row.
    pub indicator_slot: Pixels,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub background: Hsla,
    pub foreground: Hsla,
    pub hover_background: Hsla,
    pub hover_foreground: Hsla,
    pub disabled_foreground: Hsla,
}

/// The floating panel a menu lives in.
#[derive(Clone, Copy, Debug)]
pub struct MenuSurfaceSpec {
    pub radius: Pixels,
    pub padding: Pixels,
    pub min_width: Pixels,
    pub background: Hsla,
    pub foreground: Hsla,
}

/// Resolves one menu item.
pub fn item(theme: &FluentTheme, props: &MenuItemProps) -> MenuItemSpec {
    let t = theme.tokens();
    let c = &t.color;

    let (foreground, hover_background, hover_foreground) = if props.danger {
        (
            c.palette_red_foreground_1,
            c.neutral_background_1_hover,
            c.palette_red_foreground_1,
        )
    } else {
        (
            c.neutral_foreground_1,
            c.neutral_background_1_hover,
            c.neutral_foreground_1,
        )
    };

    MenuItemSpec {
        height: px(32.),
        padding_x: px(12.),
        radius: px(4.),
        gap: px(8.),
        icon_size: px(20.),
        indicator_slot: px(20.),
        font_size: px(14.),
        line_height: px(20.),
        background: c.transparent_background,
        foreground,
        hover_background,
        hover_foreground,
        disabled_foreground: c.neutral_foreground_disabled,
    }
}

/// Resolves the panel.
pub fn surface(theme: &FluentTheme) -> MenuSurfaceSpec {
    let t = theme.tokens();
    MenuSurfaceSpec {
        radius: px(4.),
        padding: px(4.),
        min_width: px(160.),
        background: t.color.neutral_background_1,
        foreground: t.color.neutral_foreground_1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> FluentTheme {
        FluentTheme::web_light()
    }

    #[test]
    fn items_are_32px_with_12px_inset() {
        let spec = item(&theme(), &MenuItemProps::default());
        assert_eq!(spec.height, px(32.));
        assert_eq!(spec.padding_x, px(12.));
        assert_eq!(spec.radius, px(4.));
    }

    #[test]
    fn danger_items_use_the_red_ramp() {
        let t = theme();
        let plain = item(&t, &MenuItemProps::default());
        let danger = item(
            &t,
            &MenuItemProps {
                danger: true,
                disabled: false,
            },
        );
        assert_ne!(plain.foreground, danger.foreground);
        assert_eq!(danger.foreground, t.tokens().color.palette_red_foreground_1);
    }

    #[test]
    fn the_panel_uses_the_neutral_surface() {
        let t = theme();
        let spec = surface(&t);
        assert_eq!(spec.background, t.tokens().color.neutral_background_1);
        assert_eq!(spec.radius, px(4.));
    }
}
