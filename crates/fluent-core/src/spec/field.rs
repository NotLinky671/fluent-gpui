//! Field and Label spec.
//!
//! Ported from
//! `react-field/.../useFieldStyles.styles.ts` and
//! `react-label/.../useLabelStyles.styles.ts`.

use fluent_theme::FluentTheme;
use gpui::{Hsla, Pixels, px};

use crate::spec::ControlSize;

/// Where the label sits relative to the control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldOrientation {
    /// Label above the control.
    #[default]
    Vertical,
    /// Label in a 33% column to the left.
    Horizontal,
}

/// Resolved colours and spacing for one field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldSpec {
    pub label_color: Hsla,
    pub label_disabled_color: Hsla,
    pub required_color: Hsla,
    pub hint_color: Hsla,
    pub error_color: Hsla,
    pub warning_color: Hsla,
    pub success_color: Hsla,
    pub label_font_size: Pixels,
    pub label_line_height: Pixels,
    pub label_gap: Pixels,
    pub hint_gap: Pixels,
    pub horizontal_label_fraction: f32,
    pub horizontal_gap: Pixels,
    pub message_gap: Pixels,
}

/// Resolves the field chrome for a size.
pub fn resolve(theme: &FluentTheme, size: ControlSize) -> FieldSpec {
    let t = theme.tokens();
    let c = &t.color;

    let (font_size, line_height, label_gap) = match size {
        // `useFieldStyles`: small label uses XXS padding/margin.
        ControlSize::Small => (t.font_size.base_200, t.line_height.base_200, px(2.)),
        // Medium uses XS margin below the label.
        ControlSize::Medium => (t.font_size.base_300, t.line_height.base_300, px(4.)),
        // Large uses SNudge.
        ControlSize::Large => (t.font_size.base_400, t.line_height.base_400, px(6.)),
    };

    FieldSpec {
        label_color: c.neutral_foreground_1,
        label_disabled_color: c.neutral_foreground_disabled,
        required_color: c.palette_red_foreground_3,
        hint_color: c.neutral_foreground_3,
        error_color: c.palette_red_foreground_1,
        warning_color: c.palette_dark_orange_foreground_1,
        success_color: c.palette_green_foreground_1,
        label_font_size: font_size,
        label_line_height: line_height,
        label_gap,
        hint_gap: px(2.),
        horizontal_label_fraction: 0.33,
        horizontal_gap: px(12.),
        message_gap: px(2.),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_tokens::Tokens;

    #[test]
    fn label_typography_follows_size() {
        let theme = FluentTheme::web_light();
        for (size, font, line) in [
            (ControlSize::Small, px(12.), px(16.)),
            (ControlSize::Medium, px(14.), px(20.)),
            (ControlSize::Large, px(16.), px(22.)),
        ] {
            let spec = resolve(&theme, size);
            assert_eq!(spec.label_font_size, font, "font for {size:?}");
            assert_eq!(spec.label_line_height, line, "line height for {size:?}");
        }
    }

    #[test]
    fn required_and_message_colours_come_from_the_status_palette() {
        let t = Tokens::web_light();
        let spec = resolve(&FluentTheme::web_light(), ControlSize::Medium);
        assert_eq!(spec.required_color, t.color.palette_red_foreground_3);
        assert_eq!(spec.error_color, t.color.palette_red_foreground_1);
        assert_eq!(spec.warning_color, t.color.palette_dark_orange_foreground_1);
        assert_eq!(spec.success_color, t.color.palette_green_foreground_1);
    }

    #[test]
    fn horizontal_layout_uses_a_third_for_the_label() {
        let spec = resolve(&FluentTheme::web_light(), ControlSize::Medium);
        assert!((spec.horizontal_label_fraction - 0.33).abs() < f32::EPSILON);
        assert_eq!(spec.horizontal_gap, px(12.));
    }
}
