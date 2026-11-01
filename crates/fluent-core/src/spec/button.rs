//! Button spec.
//!
//! Ported from
//! `packages/react-components/react-button/library/src/components/Button/useButtonStyles.styles.ts`
//! (see `plan/specs/button.md` for the line-by-line source table).

use std::time::Duration;

use fluent_theme::FluentTheme;
use fluent_tokens::CubicBezier;
use gpui::{FontWeight, Hsla, Pixels, px};

use crate::spec::ControlSize;

/// Fluent `ButtonAppearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonAppearance {
    /// Filled neutral surface with a visible border. Fluent's default.
    #[default]
    Secondary,
    /// Brand-filled.
    Primary,
    /// Transparent surface, visible border.
    Outline,
    /// Transparent surface and border, secondary foreground.
    Subtle,
    /// Transparent surface and border, brand foreground on interaction.
    Transparent,
}

/// Fluent `ButtonShape`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonShape {
    #[default]
    Rounded,
    Circular,
    Square,
}

/// Where an icon sits relative to the label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconPosition {
    Before,
    After,
}

/// Everything the button needs to resolve its look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonProps {
    pub appearance: ButtonAppearance,
    pub size: ControlSize,
    pub shape: ButtonShape,
    pub disabled: bool,
    pub icon: Option<IconPosition>,
    /// Icon without a label: square button sized to the icon.
    pub icon_only: bool,
}

/// The three colours of one visual state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonColors {
    pub background: Hsla,
    pub foreground: Hsla,
    pub border: Hsla,
}

/// Focus indicator, expressed as the inset rings Fluent draws for buttons.
///
/// Fluent uses a custom focus indicator for buttons rather than the generic
/// outline: a 1px inset ring in `colorStrokeFocus2`, plus — for the primary
/// appearance — a 2px inset ring in `colorNeutralForegroundOnBrand` at the edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusStyle {
    pub radius: Pixels,
    pub border_color: Hsla,
    pub inner_ring_color: Hsla,
    pub inner_ring_width: Pixels,
    pub outer_ring_color: Option<Hsla>,
    pub outer_ring_width: Pixels,
}

/// Size-dependent geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonMetrics {
    pub min_width: Pixels,
    pub padding_x: Pixels,
    pub padding_y: Pixels,
    pub radius: Pixels,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub font_weight: FontWeight,
    pub icon_size: Pixels,
    pub icon_gap: Pixels,
    pub border_width: Pixels,
    pub transition_duration: Duration,
    pub transition_curve: CubicBezier,
}

/// Resolved button appearance for one set of props.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonSpec {
    pub rest: ButtonColors,
    pub hover: ButtonColors,
    pub pressed: ButtonColors,
    pub disabled: ButtonColors,
    pub focus: FocusStyle,
    pub metrics: ButtonMetrics,
}

fn colors(background: Hsla, foreground: Hsla, border: Hsla) -> ButtonColors {
    ButtonColors {
        background,
        foreground,
        border,
    }
}

/// Resolves the button's colours and geometry.
///
/// Reads only [`fluent_tokens::Tokens`] fields; no hard-coded colours.
pub fn resolve(theme: &FluentTheme, props: &ButtonProps) -> ButtonSpec {
    let t = theme.tokens();
    let c = &t.color;
    let transparent = c.transparent_stroke;

    let (rest, hover, pressed) = match props.appearance {
        // Secondary is Fluent's base style: filled neutral surface + border.
        ButtonAppearance::Secondary => (
            colors(c.neutral_background_1, c.neutral_foreground_1, c.neutral_stroke_1),
            colors(
                c.neutral_background_1_hover,
                c.neutral_foreground_1_hover,
                c.neutral_stroke_1_hover,
            ),
            colors(
                c.neutral_background_1_pressed,
                c.neutral_foreground_1_pressed,
                c.neutral_stroke_1_pressed,
            ),
        ),
        // Outline keeps the border but drops the fill.
        ButtonAppearance::Outline => (
            colors(c.transparent_background, c.neutral_foreground_1, c.neutral_stroke_1),
            colors(
                c.transparent_background,
                c.neutral_foreground_1_hover,
                c.neutral_stroke_1_hover,
            ),
            colors(
                c.transparent_background,
                c.neutral_foreground_1_pressed,
                c.neutral_stroke_1_pressed,
            ),
        ),
        ButtonAppearance::Primary => (
            colors(c.brand_background, c.neutral_foreground_on_brand, transparent),
            colors(
                c.brand_background_hover,
                c.neutral_foreground_on_brand,
                transparent,
            ),
            colors(
                c.brand_background_pressed,
                c.neutral_foreground_on_brand,
                transparent,
            ),
        ),
        ButtonAppearance::Subtle => (
            colors(c.subtle_background, c.neutral_foreground_2, transparent),
            colors(
                c.subtle_background_hover,
                c.neutral_foreground_2_hover,
                transparent,
            ),
            colors(
                c.subtle_background_pressed,
                c.neutral_foreground_2_pressed,
                transparent,
            ),
        ),
        ButtonAppearance::Transparent => (
            colors(c.transparent_background, c.neutral_foreground_2, transparent),
            colors(
                c.transparent_background,
                c.neutral_foreground_2_brand_hover,
                transparent,
            ),
            colors(
                c.transparent_background,
                c.neutral_foreground_2_brand_pressed,
                transparent,
            ),
        ),
    };

    // Disabled starts from the shared disabled palette; each appearance then
    // overrides the surface or border it owns.
    let disabled_base = colors(
        c.neutral_background_disabled,
        c.neutral_foreground_disabled,
        c.neutral_stroke_disabled,
    );
    let disabled = match props.appearance {
        ButtonAppearance::Secondary => disabled_base,
        ButtonAppearance::Outline => ButtonColors {
            background: c.transparent_background,
            ..disabled_base
        },
        ButtonAppearance::Primary => ButtonColors {
            border: transparent,
            ..disabled_base
        },
        ButtonAppearance::Subtle | ButtonAppearance::Transparent => ButtonColors {
            background: c.transparent_background,
            border: transparent,
            ..disabled_base
        },
    };

    ButtonSpec {
        rest,
        hover,
        pressed,
        disabled,
        focus: resolve_focus(theme, props),
        metrics: resolve_metrics(theme, props),
    }
}

fn resolve_focus(theme: &FluentTheme, props: &ButtonProps) -> FocusStyle {
    let t = theme.tokens();
    let c = &t.color;

    // The ring radius follows size, then shape — matching the order in which
    // `useRootFocusStyles` merges `rootFocusStyles[size]` and `[shape]`.
    let size_radius = match props.size {
        ControlSize::Small => t.radius.small,
        ControlSize::Medium => t.radius.medium,
        ControlSize::Large => t.radius.large,
    };
    let radius = match props.shape {
        ButtonShape::Rounded => size_radius,
        ButtonShape::Circular => t.radius.circular,
        ButtonShape::Square => t.radius.none,
    };

    FocusStyle {
        radius,
        border_color: c.stroke_focus_2,
        inner_ring_color: c.stroke_focus_2,
        inner_ring_width: t.stroke_width.thin,
        // Primary buttons add an on-brand ring so the indicator stays visible
        // against the brand fill.
        outer_ring_color: match props.appearance {
            ButtonAppearance::Primary => Some(c.neutral_foreground_on_brand),
            _ => None,
        },
        outer_ring_width: match props.appearance {
            ButtonAppearance::Primary => t.stroke_width.thick,
            _ => px(0.),
        },
    }
}

fn resolve_metrics(theme: &FluentTheme, props: &ButtonProps) -> ButtonMetrics {
    let t = theme.tokens();
    let s = &t.spacing.horizontal;

    let (min_width, padding_x, padding_y, font_size, line_height, weight, icon_size, icon_gap) =
        match props.size {
            ControlSize::Small => (
                px(64.),
                s.s,
                px(3.),
                t.font_size.base_200,
                t.line_height.base_200,
                t.font_weight.regular,
                px(20.),
                s.xs,
            ),
            ControlSize::Medium => (
                px(96.),
                s.m,
                px(5.),
                t.font_size.base_300,
                t.line_height.base_300,
                t.font_weight.semibold,
                px(20.),
                s.s_nudge,
            ),
            ControlSize::Large => (
                px(96.),
                s.l,
                px(8.),
                t.font_size.base_400,
                t.line_height.base_400,
                t.font_weight.semibold,
                px(24.),
                s.s_nudge,
            ),
        };

    // With an icon the block padding tightens to keep the control height.
    let padding_y = match (props.icon, props.size) {
        (Some(_), ControlSize::Small) => px(1.),
        (Some(_), ControlSize::Large) => px(7.),
        _ => padding_y,
    };

    // Icon-only buttons are square and sized to the icon box.
    let (min_width, padding_x, padding_y) = if props.icon_only {
        match props.size {
            ControlSize::Small => (px(24.), px(1.), px(1.)),
            ControlSize::Medium => (px(32.), px(5.), px(5.)),
            ControlSize::Large => (px(40.), px(7.), px(7.)),
        }
    } else {
        (min_width, padding_x, padding_y)
    };

    let radius = match props.shape {
        ButtonShape::Rounded => t.radius.medium,
        ButtonShape::Circular => t.radius.circular,
        ButtonShape::Square => t.radius.none,
    };

    ButtonMetrics {
        min_width,
        padding_x,
        padding_y,
        radius,
        font_size,
        line_height,
        font_weight: weight,
        icon_size,
        icon_gap,
        border_width: t.stroke_width.thin,
        transition_duration: t.duration.faster,
        transition_curve: t.curve.easy_ease,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_tokens::Tokens;
    use gpui::px;

    fn theme() -> FluentTheme {
        FluentTheme::web_light()
    }

    fn props(appearance: ButtonAppearance) -> ButtonProps {
        ButtonProps {
            appearance,
            ..Default::default()
        }
    }

    #[test]
    fn secondary_is_fluent_base_style() {
        let t = Tokens::web_light();
        let spec = resolve(&theme(), &props(ButtonAppearance::Secondary));
        assert_eq!(spec.rest.background, t.color.neutral_background_1);
        assert_eq!(spec.rest.foreground, t.color.neutral_foreground_1);
        assert_eq!(spec.rest.border, t.color.neutral_stroke_1);
        assert_eq!(spec.hover.background, t.color.neutral_background_1_hover);
        assert_eq!(spec.pressed.background, t.color.neutral_background_1_pressed);
        assert_eq!(spec.hover.border, t.color.neutral_stroke_1_hover);
    }

    #[test]
    fn primary_uses_brand_and_on_brand_foreground() {
        let t = Tokens::web_light();
        let spec = resolve(&theme(), &props(ButtonAppearance::Primary));
        assert_eq!(spec.rest.background, t.color.brand_background);
        assert_eq!(spec.rest.foreground, t.color.neutral_foreground_on_brand);
        assert_eq!(spec.rest.border, t.color.transparent_stroke);
        assert_eq!(spec.hover.background, t.color.brand_background_hover);
        assert_eq!(spec.pressed.background, t.color.brand_background_pressed);
    }

    #[test]
    fn outline_keeps_border_but_drops_fill() {
        let t = Tokens::web_light();
        let spec = resolve(&theme(), &props(ButtonAppearance::Outline));
        assert_eq!(spec.rest.background, t.color.transparent_background);
        assert_eq!(spec.rest.border, t.color.neutral_stroke_1);
        assert_eq!(spec.hover.background, t.color.transparent_background);
        assert_eq!(spec.hover.border, t.color.neutral_stroke_1_hover);
    }

    #[test]
    fn subtle_and_transparent_differ_only_in_hover_foreground() {
        let t = Tokens::web_light();
        let subtle = resolve(&theme(), &props(ButtonAppearance::Subtle));
        let transparent = resolve(&theme(), &props(ButtonAppearance::Transparent));

        assert_eq!(subtle.rest, transparent.rest);
        assert_eq!(subtle.hover.background, t.color.subtle_background_hover);
        assert_eq!(
            transparent.hover.background,
            t.color.transparent_background
        );
        assert_eq!(subtle.hover.foreground, t.color.neutral_foreground_2_hover);
        assert_eq!(
            transparent.hover.foreground,
            t.color.neutral_foreground_2_brand_hover
        );
    }

    #[test]
    fn disabled_follows_appearance_overrides() {
        let t = Tokens::web_light();
        let secondary = resolve(&theme(), &props(ButtonAppearance::Secondary)).disabled;
        assert_eq!(secondary.background, t.color.neutral_background_disabled);
        assert_eq!(secondary.border, t.color.neutral_stroke_disabled);

        let outline = resolve(&theme(), &props(ButtonAppearance::Outline)).disabled;
        assert_eq!(outline.background, t.color.transparent_background);
        assert_eq!(outline.border, t.color.neutral_stroke_disabled);

        let primary = resolve(&theme(), &props(ButtonAppearance::Primary)).disabled;
        assert_eq!(primary.background, t.color.neutral_background_disabled);
        assert_eq!(primary.border, t.color.transparent_stroke);

        let subtle = resolve(&theme(), &props(ButtonAppearance::Subtle)).disabled;
        assert_eq!(subtle.background, t.color.transparent_background);
        assert_eq!(subtle.border, t.color.transparent_stroke);

        for spec in [secondary, outline, primary, subtle] {
            assert_eq!(spec.foreground, t.color.neutral_foreground_disabled);
        }
    }

    #[test]
    fn size_metrics_match_fluent_table() {
        let t = Tokens::web_light();
        let small = resolve(
            &theme(),
            &ButtonProps {
                size: ControlSize::Small,
                ..Default::default()
            },
        )
        .metrics;
        assert_eq!(small.min_width, px(64.));
        assert_eq!(small.padding_x, px(8.));
        assert_eq!(small.padding_y, px(3.));
        assert_eq!(small.font_size, px(12.));
        assert_eq!(small.line_height, px(16.));
        assert_eq!(small.font_weight, t.font_weight.regular);
        assert_eq!(small.icon_gap, px(4.));

        let medium = resolve(&theme(), &ButtonProps::default()).metrics;
        assert_eq!(medium.min_width, px(96.));
        assert_eq!(medium.padding_x, px(12.));
        assert_eq!(medium.padding_y, px(5.));
        assert_eq!(medium.font_size, px(14.));
        assert_eq!(medium.font_weight, t.font_weight.semibold);

        let large = resolve(
            &theme(),
            &ButtonProps {
                size: ControlSize::Large,
                ..Default::default()
            },
        )
        .metrics;
        assert_eq!(large.padding_x, px(16.));
        assert_eq!(large.padding_y, px(8.));
        assert_eq!(large.font_size, px(16.));
        assert_eq!(large.line_height, px(22.));
        assert_eq!(large.icon_size, px(24.));
    }

    #[test]
    fn icon_tightens_block_padding() {
        let small = resolve(
            &theme(),
            &ButtonProps {
                size: ControlSize::Small,
                icon: Some(IconPosition::Before),
                ..Default::default()
            },
        )
        .metrics;
        assert_eq!(small.padding_y, px(1.));

        let large = resolve(
            &theme(),
            &ButtonProps {
                size: ControlSize::Large,
                icon: Some(IconPosition::After),
                ..Default::default()
            },
        )
        .metrics;
        assert_eq!(large.padding_y, px(7.));
    }

    #[test]
    fn icon_only_is_square() {
        for (size, expected) in [
            (ControlSize::Small, px(24.)),
            (ControlSize::Medium, px(32.)),
            (ControlSize::Large, px(40.)),
        ] {
            let m = resolve(
                &theme(),
                &ButtonProps {
                    size,
                    icon: Some(IconPosition::Before),
                    icon_only: true,
                    ..Default::default()
                },
            )
            .metrics;
            assert_eq!(m.min_width, expected, "icon-only width for {size:?}");
        }
    }

    #[test]
    fn shape_overrides_radius() {
        let rounded = resolve(&theme(), &ButtonProps::default()).metrics.radius;
        assert_eq!(rounded, px(4.));

        let square = resolve(
            &theme(),
            &ButtonProps {
                shape: ButtonShape::Square,
                ..Default::default()
            },
        )
        .metrics;
        assert_eq!(square.radius, px(0.));

        let circular = resolve(
            &theme(),
            &ButtonProps {
                shape: ButtonShape::Circular,
                ..Default::default()
            },
        )
        .metrics;
        assert_eq!(circular.radius, px(10000.));
    }

    #[test]
    fn focus_radius_follows_size_then_shape() {
        let small = resolve(
            &theme(),
            &ButtonProps {
                size: ControlSize::Small,
                ..Default::default()
            },
        )
        .focus;
        assert_eq!(small.radius, px(2.));

        let large = resolve(
            &theme(),
            &ButtonProps {
                size: ControlSize::Large,
                ..Default::default()
            },
        )
        .focus;
        assert_eq!(large.radius, px(6.));

        let circular = resolve(
            &theme(),
            &ButtonProps {
                shape: ButtonShape::Circular,
                ..Default::default()
            },
        )
        .focus;
        assert_eq!(circular.radius, px(10000.));
    }

    #[test]
    fn primary_focus_adds_on_brand_ring() {
        let t = Tokens::web_light();
        let primary = resolve(&theme(), &props(ButtonAppearance::Primary)).focus;
        assert_eq!(
            primary.outer_ring_color,
            Some(t.color.neutral_foreground_on_brand)
        );
        assert_eq!(primary.outer_ring_width, px(2.));
        assert_eq!(primary.inner_ring_width, px(1.));
        assert_eq!(primary.inner_ring_color, t.color.stroke_focus_2);

        let secondary = resolve(&theme(), &props(ButtonAppearance::Secondary)).focus;
        assert_eq!(secondary.outer_ring_color, None);
        assert_eq!(secondary.outer_ring_width, px(0.));
    }

    #[test]
    fn dark_theme_resolves_dark_tokens() {
        let t = Tokens::web_dark();
        let spec = resolve(&FluentTheme::web_dark(), &props(ButtonAppearance::Secondary));
        assert_eq!(spec.rest.background, t.color.neutral_background_1);
        assert_eq!(spec.rest.background, gpui::Hsla::from(gpui::rgba(0x292929ff)));
        assert_eq!(spec.focus.inner_ring_color, t.color.stroke_focus_2);
    }

    #[test]
    fn transition_uses_faster_and_easy_ease() {
        let t = Tokens::web_light();
        let m = resolve(&theme(), &ButtonProps::default()).metrics;
        assert_eq!(m.transition_duration, Duration::from_millis(100));
        assert_eq!(m.transition_curve, t.curve.easy_ease);
    }
}