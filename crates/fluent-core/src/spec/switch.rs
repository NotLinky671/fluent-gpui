//! Switch spec.
//!
//! Ported from
//! `react-switch/.../useSwitchStyles.styles.ts`.

use fluent_theme::FluentTheme;
use gpui::{Hsla, Pixels, px};

use crate::spec::ControlSize;

/// Switch state that affects visuals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwitchProps {
    pub size: ControlSize,
    pub checked: bool,
    pub disabled: bool,
}

/// Resolved switch colours and geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwitchSpec {
    pub track_width: Pixels,
    pub track_height: Pixels,
    pub thumb_size: Pixels,
    /// Left offset of the thumb when checked.
    pub thumb_offset: Pixels,
    /// Left padding when unchecked (inside the track border).
    pub thumb_rest_offset: Pixels,
    pub radius: Pixels,
    pub label_gap: Pixels,
    pub border_width: Pixels,
    pub track_bg: Hsla,
    pub track_border: Hsla,
    pub thumb: Hsla,
    pub hover_track_bg: Hsla,
    pub hover_track_border: Hsla,
    pub hover_thumb: Hsla,
    pub pressed_track_bg: Hsla,
    pub pressed_track_border: Hsla,
    pub pressed_thumb: Hsla,
}

/// Resolves a switch.
pub fn resolve(theme: &FluentTheme, props: &SwitchProps) -> SwitchSpec {
    let t = theme.tokens();
    let c = &t.color;
    // Measured against Fluent's rendered switch: the thumb is ~70% of the
    // track height, with a visible gap on every side.
    let (track_width, track_height) = match props.size {
        ControlSize::Small => (px(32.), px(16.)),
        _ => (px(40.), px(20.)),
    };
    // The origin component centres the thumb with flex inside the 1px track
    // border, so the leftover height above and below must be a whole number of
    // pixels: an 11px thumb in the small 16px track leaves 1.5px, which the
    // renderer snaps and paints one pixel low. A 6px total inset keeps 2px of
    // air inside the border and divides evenly for both sizes.
    let thumb_size = track_height - px(6.);
    let thumb_inset = (track_height - thumb_size) / 2.0;
    let thumb_offset = track_width - thumb_size - thumb_inset;

    if props.disabled {
        return SwitchSpec {
            track_width,
            track_height,
            thumb_size,
            thumb_offset,
            thumb_rest_offset: thumb_inset,
            radius: t.radius.circular,
            label_gap: t.spacing.horizontal.s,
            border_width: t.stroke_width.thin,
            track_bg: c.neutral_background_disabled,
            track_border: c.transparent_stroke_disabled,
            thumb: c.neutral_foreground_disabled,
            hover_track_bg: c.neutral_background_disabled,
            hover_track_border: c.transparent_stroke_disabled,
            hover_thumb: c.neutral_foreground_disabled,
            pressed_track_bg: c.neutral_background_disabled,
            pressed_track_border: c.transparent_stroke_disabled,
            pressed_thumb: c.neutral_foreground_disabled,
        };
    }

    if props.checked {
        SwitchSpec {
            track_width,
            track_height,
            thumb_size,
            thumb_offset,
            thumb_rest_offset: thumb_inset,
            radius: t.radius.circular,
            label_gap: t.spacing.horizontal.s,
            border_width: t.stroke_width.thin,
            track_bg: c.compound_brand_background,
            track_border: c.transparent_stroke,
            thumb: c.neutral_foreground_inverted,
            hover_track_bg: c.compound_brand_background_hover,
            hover_track_border: c.transparent_stroke_interactive,
            hover_thumb: c.neutral_foreground_inverted,
            pressed_track_bg: c.compound_brand_background_pressed,
            pressed_track_border: c.transparent_stroke_interactive,
            pressed_thumb: c.neutral_foreground_inverted,
        }
    } else {
        SwitchSpec {
            track_width,
            track_height,
            thumb_size,
            thumb_offset,
            thumb_rest_offset: thumb_inset,
            radius: t.radius.circular,
            label_gap: t.spacing.horizontal.s,
            border_width: t.stroke_width.thin,
            track_bg: c.transparent_background,
            track_border: c.neutral_stroke_accessible,
            thumb: c.neutral_stroke_accessible,
            hover_track_bg: c.transparent_background,
            hover_track_border: c.neutral_stroke_accessible_hover,
            hover_thumb: c.neutral_stroke_accessible_hover,
            pressed_track_bg: c.transparent_background,
            pressed_track_border: c.neutral_stroke_accessible_pressed,
            pressed_thumb: c.neutral_stroke_accessible_pressed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_tokens::Tokens;

    #[test]
    fn sizes_match_fluent_switches() {
        let theme = FluentTheme::web_light();
        let medium = resolve(&theme, &SwitchProps::default());
        assert_eq!(medium.track_width, px(40.));
        assert_eq!(medium.track_height, px(20.));
        assert_eq!(medium.thumb_size, px(14.));
        assert_eq!(medium.thumb_offset, px(23.));

        let small = resolve(
            &theme,
            &SwitchProps {
                size: ControlSize::Small,
                ..Default::default()
            },
        );
        assert_eq!(small.track_width, px(32.));
        assert_eq!(small.track_height, px(16.));
        assert_eq!(small.thumb_size, px(10.));
        assert_eq!(small.thumb_offset, px(19.));

        // The thumb sits inside the 1px border, so its leftover height must be
        // even; otherwise the flex centring lands on a half pixel and the
        // painted thumb is a pixel low (the small switch used to do this).
        for spec in [medium, small] {
            let inset = (spec.track_height - px(2.) - spec.thumb_size) / 2.0;
            assert_eq!(inset, inset.round(), "{spec:?}");
        }
    }

    #[test]
    fn checked_switch_uses_compound_brand_background() {
        let t = Tokens::web_light();
        let spec = resolve(
            &FluentTheme::web_light(),
            &SwitchProps {
                checked: true,
                ..Default::default()
            },
        );
        assert_eq!(spec.track_bg, t.color.compound_brand_background);
        assert_eq!(spec.thumb, t.color.neutral_foreground_inverted);
        assert_eq!(spec.hover_track_bg, t.color.compound_brand_background_hover);
    }

    #[test]
    fn unchecked_switch_uses_the_accessible_stroke_for_track_and_thumb() {
        let t = Tokens::web_light();
        let spec = resolve(&FluentTheme::web_light(), &SwitchProps::default());
        assert_eq!(spec.track_border, t.color.neutral_stroke_accessible);
        assert_eq!(spec.thumb, t.color.neutral_stroke_accessible);
        assert_eq!(spec.track_bg, t.color.transparent_background);
    }

    #[test]
    fn disabled_switch_never_uses_brand_colors() {
        let t = Tokens::web_light();
        let spec = resolve(
            &FluentTheme::web_light(),
            &SwitchProps {
                checked: true,
                disabled: true,
                ..Default::default()
            },
        );
        assert_eq!(spec.track_bg, t.color.neutral_background_disabled);
        assert_eq!(spec.thumb, t.color.neutral_foreground_disabled);
    }
}
