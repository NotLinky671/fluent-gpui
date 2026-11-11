//! Fluent 2 title bar geometry and colours.
//!
//! The bar replaces the OS caption when a window asks for
//! `TitlebarOptions::appears_transparent`. Windows draws its own caption at 32px
//! with 46px-wide buttons, so those numbers are fixed here rather than derived
//! from the control ramp.

use fluent_theme::FluentTheme;
use gpui::{Hsla, Pixels, px};

/// Geometry of the custom title bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TitleBarMetrics {
    pub height: Pixels,
    pub padding_x: Pixels,
    /// Leading room for the native macOS traffic lights.
    pub macos_padding_x: Pixels,
    pub gap: Pixels,
    pub icon_size: Pixels,
    pub title_font_size: Pixels,
    pub caption_width: Pixels,
    pub caption_height: Pixels,
    /// Glyph size inside a caption button.
    pub glyph_size: Pixels,
    /// Subtle outer radius of maximize/restore window outlines.
    pub glyph_corner_radius: Pixels,
    /// Maximize/restore outline thickness; preserve fractional device pixels.
    pub glyph_stroke_width: Pixels,
}

/// Colours of the bar and its caption buttons.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TitleBarColors {
    pub title: Hsla,
    pub glyph: Hsla,
    pub button_hover: Hsla,
    pub button_pressed: Hsla,
    /// Windows paints the close button red on hover, in every theme.
    pub close_hover: Hsla,
    pub close_pressed: Hsla,
    pub close_glyph_hover: Hsla,
}

/// Everything the title bar paints with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TitleBarSpec {
    pub metrics: TitleBarMetrics,
    pub colors: TitleBarColors,
}

/// Resolves the title bar spec.
pub fn bar(theme: &FluentTheme) -> TitleBarSpec {
    let c = &theme.tokens().color;
    TitleBarSpec {
        metrics: TitleBarMetrics {
            height: px(32.),
            padding_x: px(12.),
            macos_padding_x: px(80.),
            gap: px(8.),
            icon_size: px(16.),
            title_font_size: px(12.),
            caption_width: px(46.),
            caption_height: px(32.),
            glyph_size: px(10.),
            glyph_corner_radius: px(1.75),
            glyph_stroke_width: px(1.),
        },
        colors: TitleBarColors {
            title: c.neutral_foreground_1,
            glyph: c.neutral_foreground_2,
            button_hover: c.subtle_background_hover,
            button_pressed: c.subtle_background_pressed,
            // Windows caption red is a platform colour, independent of the app
            // palette (including high contrast). Keep white ink legible on it.
            close_hover: gpui::rgb(0xc42b1c).into(),
            close_pressed: gpui::Hsla::from(gpui::rgb(0xc42b1c)).opacity(0.9),
            // Fixed white: the caption close button keeps a white glyph on its red
            // hover in both themes, which no background token expresses.
            close_glyph_hover: gpui::hsla(0.0, 0.0, 1.0, 1.0),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_theme::{Appearance, FluentTheme};

    fn light() -> FluentTheme {
        FluentTheme::for_appearance(Appearance::Light)
    }

    #[test]
    fn the_bar_matches_the_windows_caption() {
        let spec = bar(&light());
        assert_eq!(spec.metrics.height, px(32.));
        assert_eq!(spec.metrics.caption_width, px(46.));
        assert_eq!(spec.metrics.caption_height, px(32.));
    }

    #[test]
    fn the_close_button_keeps_a_white_glyph() {
        let spec = bar(&light());
        assert_eq!(spec.colors.close_glyph_hover.a, 1.0);
        assert_eq!(spec.colors.close_glyph_hover.l, 1.0);
    }

    #[test]
    fn close_states_stay_legible_and_distinct_across_themes() {
        for theme in [
            FluentTheme::web_light(),
            FluentTheme::web_dark(),
            FluentTheme::high_contrast(),
        ] {
            let colors = bar(&theme).colors;
            assert_eq!(colors.close_hover, Hsla::from(gpui::rgb(0xc42b1c)));
            assert_eq!(colors.close_glyph_hover, gpui::hsla(0., 0., 1., 1.));
            assert_ne!(colors.close_hover, colors.close_pressed);
            assert_eq!(colors.close_pressed.a, 0.9);
        }
    }

    #[test]
    fn the_hover_fills_differ_from_the_rest() {
        let spec = bar(&light());
        assert_ne!(spec.colors.button_hover, spec.colors.button_pressed);
        assert_ne!(spec.colors.close_hover, spec.colors.button_hover);
    }
}
