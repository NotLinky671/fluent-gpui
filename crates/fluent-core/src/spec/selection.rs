//! Checkbox and Radio spec.
//!
//! Ported from
//! `react-checkbox/.../useCheckboxStyles.styles.ts` and
//! `react-radio/.../useRadioStyles.styles.ts`.

use fluent_theme::FluentTheme;
use gpui::{Hsla, Pixels, px};

use crate::spec::ControlSize;

/// Checkbox state that affects visuals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheckboxProps {
    pub size: ControlSize,
    pub checked: bool,
    pub disabled: bool,
}

/// Resolved checkbox colours and geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckboxSpec {
    pub box_size: Pixels,
    pub radius: Pixels,
    pub border_width: Pixels,
    pub border: Hsla,
    pub hover_border: Hsla,
    pub pressed_border: Hsla,
    pub background: Hsla,
    pub hover_background: Hsla,
    pub pressed_background: Hsla,
    pub mark: Hsla,
    pub label: Hsla,
}

/// Radio state that affects visuals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RadioProps {
    pub checked: bool,
    pub disabled: bool,
}

/// Resolved radio colours and geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadioSpec {
    pub circle: Pixels,
    pub dot: Pixels,
    pub border_width: Pixels,
    pub border: Hsla,
    pub hover_border: Hsla,
    pub pressed_border: Hsla,
    pub dot_color: Hsla,
    pub hover_dot: Hsla,
    pub pressed_dot: Hsla,
    pub label: Hsla,
}

/// Resolves a checkbox.
pub fn checkbox(theme: &FluentTheme, props: &CheckboxProps) -> CheckboxSpec {
    let t = theme.tokens();
    let c = &t.color;

    let box_size = match props.size {
        ControlSize::Large => px(20.),
        _ => px(16.),
    };

    if props.disabled {
        return CheckboxSpec {
            box_size,
            radius: t.radius.small,
            border_width: t.stroke_width.thin,
            border: c.neutral_stroke_disabled,
            hover_border: c.neutral_stroke_disabled,
            pressed_border: c.neutral_stroke_disabled,
            background: if props.checked {
                c.neutral_background_disabled
            } else {
                c.transparent_background
            },
            hover_background: c.neutral_background_disabled,
            pressed_background: c.neutral_background_disabled,
            mark: c.neutral_foreground_disabled,
            label: c.neutral_foreground_disabled,
        };
    }

    if props.checked {
        CheckboxSpec {
            box_size,
            radius: t.radius.small,
            border_width: t.stroke_width.thin,
            border: c.compound_brand_background,
            hover_border: c.compound_brand_background_hover,
            pressed_border: c.compound_brand_background_pressed,
            background: c.compound_brand_background,
            hover_background: c.compound_brand_background_hover,
            pressed_background: c.compound_brand_background_pressed,
            mark: c.neutral_foreground_inverted,
            label: c.neutral_foreground_1,
        }
    } else {
        CheckboxSpec {
            box_size,
            radius: t.radius.small,
            border_width: t.stroke_width.thin,
            border: c.neutral_stroke_accessible,
            hover_border: c.neutral_stroke_accessible_hover,
            pressed_border: c.neutral_stroke_accessible_pressed,
            background: c.transparent_background,
            hover_background: c.transparent_background,
            pressed_background: c.transparent_background,
            mark: c.transparent_background,
            label: c.neutral_foreground_1,
        }
    }
}

/// Resolves a radio button.
pub fn radio(theme: &FluentTheme, props: &RadioProps) -> RadioSpec {
    let t = theme.tokens();
    let c = &t.color;
    let circle = px(16.);
    // Fluent paints the dot as a 16px box scaled to 0.625, i.e. 10px. Keep the
    // gap between the two diameters an even number of pixels: a fractional dot
    // (9.5) leaves a 0.5px inset that the renderer snaps to a whole pixel, so
    // the dot lands visibly off-centre on both 1x and 2x displays.
    let dot = px(10.);

    if props.disabled {
        return RadioSpec {
            circle,
            dot,
            border_width: t.stroke_width.thin,
            border: c.neutral_stroke_disabled,
            hover_border: c.neutral_stroke_disabled,
            pressed_border: c.neutral_stroke_disabled,
            dot_color: c.neutral_foreground_disabled,
            hover_dot: c.neutral_foreground_disabled,
            pressed_dot: c.neutral_foreground_disabled,
            label: c.neutral_foreground_disabled,
        };
    }

    if props.checked {
        RadioSpec {
            circle,
            dot,
            border_width: t.stroke_width.thin,
            border: c.compound_brand_stroke,
            hover_border: c.compound_brand_stroke_hover,
            pressed_border: c.compound_brand_stroke_pressed,
            dot_color: c.compound_brand_foreground_1,
            hover_dot: c.compound_brand_foreground_1_hover,
            pressed_dot: c.compound_brand_foreground_1_pressed,
            label: c.neutral_foreground_1,
        }
    } else {
        RadioSpec {
            circle,
            dot,
            border_width: t.stroke_width.thin,
            border: c.neutral_stroke_accessible,
            hover_border: c.neutral_stroke_accessible_hover,
            pressed_border: c.neutral_stroke_accessible_pressed,
            dot_color: c.transparent_background,
            hover_dot: c.transparent_background,
            pressed_dot: c.transparent_background,
            label: c.neutral_foreground_1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_tokens::Tokens;

    #[test]
    fn checkbox_sizes_match_fluent() {
        let theme = FluentTheme::web_light();
        assert_eq!(
            checkbox(&theme, &CheckboxProps::default()).box_size,
            px(16.)
        );
        assert_eq!(
            checkbox(
                &theme,
                &CheckboxProps {
                    size: ControlSize::Large,
                    ..Default::default()
                }
            )
            .box_size,
            px(20.)
        );
    }

    #[test]
    fn checked_checkbox_uses_the_compound_brand_fill() {
        let t = Tokens::web_light();
        let spec = checkbox(
            &FluentTheme::web_light(),
            &CheckboxProps {
                checked: true,
                ..Default::default()
            },
        );
        assert_eq!(spec.background, t.color.compound_brand_background);
        assert_eq!(spec.mark, t.color.neutral_foreground_inverted);
        assert_eq!(
            spec.hover_background,
            t.color.compound_brand_background_hover
        );
    }

    #[test]
    fn unchecked_checkbox_uses_the_accessible_stroke() {
        let t = Tokens::web_light();
        let spec = checkbox(&FluentTheme::web_light(), &CheckboxProps::default());
        assert_eq!(spec.border, t.color.neutral_stroke_accessible);
        assert_eq!(spec.background, t.color.transparent_background);
    }

    #[test]
    fn disabled_checkbox_never_uses_brand_colors() {
        let t = Tokens::web_light();
        let spec = checkbox(
            &FluentTheme::web_light(),
            &CheckboxProps {
                checked: true,
                disabled: true,
                ..Default::default()
            },
        );
        assert_eq!(spec.border, t.color.neutral_stroke_disabled);
        assert_eq!(spec.background, t.color.neutral_background_disabled);
        assert_eq!(spec.mark, t.color.neutral_foreground_disabled);
    }

    #[test]
    fn radio_dot_is_five_eighths_of_the_circle() {
        let spec = radio(&FluentTheme::web_light(), &RadioProps::default());
        assert_eq!(spec.circle, px(16.));
        // 16px × 0.625. Both diameters are even, so the 3px inset the flex
        // centring produces lands on a whole pixel; a 9.5px dot would be
        // snapped and sit half a pixel off-centre.
        assert_eq!(spec.dot, px(10.));
    }

    #[test]
    fn checked_radio_uses_compound_brand_stroke_and_dot() {
        let t = Tokens::web_light();
        let spec = radio(
            &FluentTheme::web_light(),
            &RadioProps {
                checked: true,
                ..Default::default()
            },
        );
        assert_eq!(spec.border, t.color.compound_brand_stroke);
        assert_eq!(spec.dot_color, t.color.compound_brand_foreground_1);
    }

    #[test]
    fn label_colors_follow_disabled_state() {
        let t = Tokens::web_light();
        assert_eq!(
            checkbox(&FluentTheme::web_light(), &CheckboxProps::default()).label,
            t.color.neutral_foreground_1
        );
        assert_eq!(
            checkbox(
                &FluentTheme::web_light(),
                &CheckboxProps {
                    disabled: true,
                    ..Default::default()
                }
            )
            .label,
            t.color.neutral_foreground_disabled
        );
    }
}
