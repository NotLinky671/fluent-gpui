//! Input spec.
//!
//! Ported from
//! `packages/react-components/react-input/library/src/components/Input/useInputStyles.styles.ts`
//! (see `plan/specs/input.md`).

use fluent_theme::FluentTheme;
use gpui::{Hsla, Pixels, px};

use crate::spec::ControlSize;

/// Fluent `InputAppearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputAppearance {
    /// Filled surface with a visible border. Fluent's default.
    #[default]
    Outline,
    /// Transparent surface, only the bottom edge is drawn.
    Underline,
    /// Filled with `colorNeutralBackground3`, no visible border.
    FilledDarker,
    /// Filled with `colorNeutralBackground1`, no visible border.
    FilledLighter,
}

/// Everything the input chrome needs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputProps {
    pub appearance: InputAppearance,
    pub size: ControlSize,
    pub disabled: bool,
    /// Shows the red error border while the field is not focused.
    pub invalid: bool,
}

/// The colours of one visual state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputColors {
    pub background: Hsla,
    /// All four edges.
    pub border: Hsla,
    /// The heavier bottom edge Fluent draws on outline inputs.
    pub border_bottom: Hsla,
}

/// Size-dependent geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputMetrics {
    pub height: Pixels,
    /// Horizontal padding when the field has no content slots.
    pub padding_x: Pixels,
    pub gap: Pixels,
    pub radius: Pixels,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub border_width: Pixels,
    /// Height of the focus underline overlay.
    pub underline_height: Pixels,
}

/// Resolved input appearance for one set of props.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputSpec {
    pub rest: InputColors,
    pub hover: InputColors,
    pub focus: InputColors,
    pub disabled: InputColors,
    pub invalid_border: Hsla,
    pub focus_underline: Hsla,
    pub focus_underline_pressed: Hsla,
    pub text: Hsla,
    pub placeholder: Hsla,
    /// Foreground for the disabled state.
    pub disabled_text: Hsla,
    pub metrics: InputMetrics,
}

fn colors(background: Hsla, border: Hsla, border_bottom: Hsla) -> InputColors {
    InputColors {
        background,
        border,
        border_bottom,
    }
}

/// Resolves the input's colours and geometry. Reads only token fields.
pub fn resolve(theme: &FluentTheme, props: &InputProps) -> InputSpec {
    let t = theme.tokens();
    let c = &t.color;
    let transparent = c.transparent_stroke;

    let (rest, hover, focus) = match props.appearance {
        // Outline is the base style: filled surface, 1px border, heavier bottom.
        InputAppearance::Outline => (
            colors(
                c.neutral_background_1,
                c.neutral_stroke_1,
                c.neutral_stroke_accessible,
            ),
            colors(
                c.neutral_background_1,
                c.neutral_stroke_1_hover,
                c.neutral_stroke_accessible_hover,
            ),
            colors(
                c.neutral_background_1,
                c.neutral_stroke_1_pressed,
                c.neutral_stroke_accessible_pressed,
            ),
        ),
        // Underline drops the fill and every edge except the bottom one.
        InputAppearance::Underline => (
            colors(
                c.transparent_background,
                transparent,
                c.neutral_stroke_accessible,
            ),
            colors(
                c.transparent_background,
                transparent,
                c.neutral_stroke_accessible_hover,
            ),
            colors(
                c.transparent_background,
                transparent,
                c.neutral_stroke_accessible_pressed,
            ),
        ),
        // Filled appearances have no visible border at all.
        InputAppearance::FilledDarker => (
            colors(c.neutral_background_3, transparent, transparent),
            colors(c.neutral_background_3, transparent, transparent),
            colors(c.neutral_background_3, transparent, transparent),
        ),
        InputAppearance::FilledLighter => (
            colors(c.neutral_background_1, transparent, transparent),
            colors(c.neutral_background_1, transparent, transparent),
            colors(c.neutral_background_1, transparent, transparent),
        ),
    };

    let disabled = colors(
        c.transparent_background,
        c.neutral_stroke_disabled,
        c.neutral_stroke_disabled,
    );

    InputSpec {
        rest,
        hover,
        focus,
        disabled,
        invalid_border: c.palette_red_border_2,
        focus_underline: c.compound_brand_stroke,
        focus_underline_pressed: c.compound_brand_stroke_pressed,
        text: c.neutral_foreground_1,
        placeholder: c.neutral_foreground_4,
        disabled_text: c.neutral_foreground_disabled,
        metrics: resolve_metrics(theme, props),
    }
}

fn resolve_metrics(theme: &FluentTheme, props: &InputProps) -> InputMetrics {
    let t = theme.tokens();

    let (height, padding_x, gap, font_size, line_height) = match props.size {
        ControlSize::Small => (
            px(24.),
            px(8.),
            px(2.),
            t.font_size.base_200,
            t.line_height.base_200,
        ),
        ControlSize::Medium => (
            px(32.),
            px(12.),
            px(2.),
            t.font_size.base_300,
            t.line_height.base_300,
        ),
        ControlSize::Large => (
            px(40.),
            px(18.),
            px(6.),
            t.font_size.base_400,
            t.line_height.base_400,
        ),
    };

    InputMetrics {
        height,
        padding_x,
        gap,
        // Underline inputs are square-cornered in Fluent.
        radius: match props.appearance {
            InputAppearance::Underline => t.radius.none,
            _ => t.radius.medium,
        },
        font_size,
        line_height,
        border_width: t.stroke_width.thin,
        underline_height: px(2.),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_tokens::Tokens;

    fn theme() -> FluentTheme {
        FluentTheme::web_light()
    }

    fn props(appearance: InputAppearance) -> InputProps {
        InputProps {
            appearance,
            ..Default::default()
        }
    }

    #[test]
    fn heights_match_fluent_field_sizes() {
        for (size, expected) in [
            (ControlSize::Small, px(24.)),
            (ControlSize::Medium, px(32.)),
            (ControlSize::Large, px(40.)),
        ] {
            let m = resolve(
                &theme(),
                &InputProps {
                    size,
                    ..Default::default()
                },
            )
            .metrics;
            assert_eq!(m.height, expected, "height for {size:?}");
        }
    }

    #[test]
    fn horizontal_padding_follows_size() {
        assert_eq!(
            resolve(&theme(), &InputProps::default()).metrics.padding_x,
            px(12.)
        );
        assert_eq!(
            resolve(
                &theme(),
                &InputProps {
                    size: ControlSize::Small,
                    ..Default::default()
                }
            )
            .metrics
            .padding_x,
            px(8.)
        );
        assert_eq!(
            resolve(
                &theme(),
                &InputProps {
                    size: ControlSize::Large,
                    ..Default::default()
                }
            )
            .metrics
            .padding_x,
            px(18.)
        );
    }

    #[test]
    fn outline_is_the_base_style() {
        let t = Tokens::web_light();
        let spec = resolve(&theme(), &props(InputAppearance::Outline));
        assert_eq!(spec.rest.background, t.color.neutral_background_1);
        assert_eq!(spec.rest.border, t.color.neutral_stroke_1);
        assert_eq!(spec.rest.border_bottom, t.color.neutral_stroke_accessible);
        assert_eq!(
            spec.hover.border_bottom,
            t.color.neutral_stroke_accessible_hover
        );
    }

    #[test]
    fn underline_has_no_fill_or_side_borders() {
        let t = Tokens::web_light();
        let spec = resolve(&theme(), &props(InputAppearance::Underline));
        assert_eq!(spec.rest.background, t.color.transparent_background);
        assert_eq!(spec.rest.border, t.color.transparent_stroke);
        assert_eq!(spec.rest.border_bottom, t.color.neutral_stroke_accessible);
        assert_eq!(spec.metrics.radius, px(0.));
    }

    #[test]
    fn filled_appearances_have_no_border() {
        let t = Tokens::web_light();
        let darker = resolve(&theme(), &props(InputAppearance::FilledDarker));
        assert_eq!(darker.rest.background, t.color.neutral_background_3);
        assert_eq!(darker.rest.border, t.color.transparent_stroke);

        let lighter = resolve(&theme(), &props(InputAppearance::FilledLighter));
        assert_eq!(lighter.rest.background, t.color.neutral_background_1);
        assert_eq!(lighter.rest.border, t.color.transparent_stroke);
    }

    #[test]
    fn focus_underline_uses_the_compound_brand_stroke() {
        let t = Tokens::web_light();
        let spec = resolve(&theme(), &InputProps::default());
        assert_eq!(spec.focus_underline, t.color.compound_brand_stroke);
        assert_eq!(
            spec.focus_underline_pressed,
            t.color.compound_brand_stroke_pressed
        );
        assert_eq!(spec.metrics.underline_height, px(2.));
    }

    #[test]
    fn disabled_drops_the_fill_and_uses_the_disabled_stroke() {
        let t = Tokens::web_light();
        let spec = resolve(
            &theme(),
            &InputProps {
                disabled: true,
                ..Default::default()
            },
        );
        assert_eq!(spec.disabled.background, t.color.transparent_background);
        assert_eq!(spec.disabled.border, t.color.neutral_stroke_disabled);
    }

    #[test]
    fn disabled_text_uses_the_disabled_foreground() {
        let t = Tokens::web_light();
        let spec = resolve(&theme(), &InputProps::default());
        assert_eq!(spec.disabled_text, t.color.neutral_foreground_disabled);
    }

    #[test]
    fn invalid_border_is_the_red_palette_stroke() {
        let t = Tokens::web_light();
        let spec = resolve(&theme(), &InputProps::default());
        assert_eq!(spec.invalid_border, t.color.palette_red_border_2);
    }

    #[test]
    fn large_text_uses_body2_metrics() {
        let m = resolve(
            &theme(),
            &InputProps {
                size: ControlSize::Large,
                ..Default::default()
            },
        )
        .metrics;
        assert_eq!(m.font_size, px(16.));
        assert_eq!(m.line_height, px(22.));
        assert_eq!(m.gap, px(6.));
    }
}
