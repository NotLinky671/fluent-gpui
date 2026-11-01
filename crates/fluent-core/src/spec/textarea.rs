//! Textarea spec.
//!
//! Ported from
//! `packages/react-components/react-textarea/library/src/components/Textarea/useTextareaStyles.styles.ts`.

use fluent_theme::FluentTheme;
use gpui::{Hsla, Pixels, px};

use crate::spec::ControlSize;

/// Fluent `TextareaAppearance` (the deprecated `-shadow` variants are omitted).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextareaAppearance {
    /// Filled surface with a visible border. Fluent's default.
    #[default]
    Outline,
    /// Transparent surface and border.
    Filled,
    /// `colorNeutralBackground3`, no border.
    FilledDarker,
    /// `colorNeutralBackground1`, no border.
    FilledLighter,
}

/// Everything the textarea chrome needs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextareaProps {
    pub appearance: TextareaAppearance,
    pub size: ControlSize,
    pub disabled: bool,
    pub invalid: bool,
}

/// The colours of one visual state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextareaColors {
    pub background: Hsla,
    pub border: Hsla,
    pub border_bottom: Hsla,
}

/// Size-dependent geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextareaMetrics {
    pub min_height: Pixels,
    pub padding_x: Pixels,
    pub padding_y: Pixels,
    pub radius: Pixels,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub border_width: Pixels,
    pub underline_height: Pixels,
}

/// Resolved textarea appearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextareaSpec {
    pub rest: TextareaColors,
    pub hover: TextareaColors,
    pub focus: TextareaColors,
    pub disabled: TextareaColors,
    pub invalid_border: Hsla,
    pub focus_underline: Hsla,
    pub focus_underline_pressed: Hsla,
    pub text: Hsla,
    pub placeholder: Hsla,
    pub disabled_text: Hsla,
    pub metrics: TextareaMetrics,
}

fn colors(background: Hsla, border: Hsla, border_bottom: Hsla) -> TextareaColors {
    TextareaColors {
        background,
        border,
        border_bottom,
    }
}

/// Resolves the textarea's colours and geometry. Reads only token fields.
pub fn resolve(theme: &FluentTheme, props: &TextareaProps) -> TextareaSpec {
    let t = theme.tokens();
    let c = &t.color;
    let transparent = c.transparent_stroke;

    let (rest, hover, focus) = match props.appearance {
        TextareaAppearance::Outline => (
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
            // Outline textareas turn the bottom edge into the brand focus line.
            colors(
                c.neutral_background_1,
                c.neutral_stroke_1_pressed,
                c.compound_brand_stroke,
            ),
        ),
        TextareaAppearance::Filled => (
            colors(c.transparent_background, transparent, transparent),
            colors(c.transparent_background, c.transparent_stroke, transparent),
            colors(c.transparent_background, c.transparent_stroke, transparent),
        ),
        TextareaAppearance::FilledDarker => (
            colors(c.neutral_background_3, transparent, transparent),
            colors(c.neutral_background_3, transparent, transparent),
            colors(c.neutral_background_3, transparent, transparent),
        ),
        TextareaAppearance::FilledLighter => (
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

    TextareaSpec {
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

fn resolve_metrics(theme: &FluentTheme, props: &TextareaProps) -> TextareaMetrics {
    let t = theme.tokens();
    let h = &t.spacing.horizontal;
    let v = &t.spacing.vertical;

    let (min_height, padding_x, padding_y, font_size, line_height) = match props.size {
        ControlSize::Small => (
            px(40.),
            h.s_nudge + h.xxs,
            v.xs,
            t.font_size.base_200,
            t.line_height.base_200,
        ),
        ControlSize::Medium => (
            px(52.),
            h.m_nudge + h.xxs,
            v.s_nudge,
            t.font_size.base_300,
            t.line_height.base_300,
        ),
        ControlSize::Large => (
            px(64.),
            h.m + h.xxs,
            v.s,
            t.font_size.base_400,
            t.line_height.base_400,
        ),
    };

    TextareaMetrics {
        min_height,
        padding_x,
        padding_y,
        radius: t.radius.medium,
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

    #[test]
    fn min_heights_match_fluent_textareas() {
        for (size, expected) in [
            (ControlSize::Small, px(40.)),
            (ControlSize::Medium, px(52.)),
            (ControlSize::Large, px(64.)),
        ] {
            let m = resolve(
                &theme(),
                &TextareaProps {
                    size,
                    ..Default::default()
                },
            )
            .metrics;
            assert_eq!(m.min_height, expected, "min height for {size:?}");
        }
    }

    #[test]
    fn padding_matches_the_content_plus_regular_union() {
        let small = resolve(
            &theme(),
            &TextareaProps {
                size: ControlSize::Small,
                ..Default::default()
            },
        )
        .metrics;
        assert_eq!(small.padding_x, px(8.));
        assert_eq!(small.padding_y, px(4.));

        let large = resolve(
            &theme(),
            &TextareaProps {
                size: ControlSize::Large,
                ..Default::default()
            },
        )
        .metrics;
        assert_eq!(large.padding_x, px(14.));
        assert_eq!(large.padding_y, px(8.));
    }

    #[test]
    fn outline_focus_turns_the_bottom_edge_brand() {
        let t = Tokens::web_light();
        let spec = resolve(&theme(), &TextareaProps::default());
        assert_eq!(spec.focus.border_bottom, t.color.compound_brand_stroke);
        assert_eq!(spec.rest.border, t.color.neutral_stroke_1);
        assert_eq!(spec.rest.border_bottom, t.color.neutral_stroke_accessible);
    }

    #[test]
    fn filled_appearances_have_no_border() {
        let t = Tokens::web_light();
        for appearance in [
            TextareaAppearance::Filled,
            TextareaAppearance::FilledDarker,
            TextareaAppearance::FilledLighter,
        ] {
            let spec = resolve(
                &theme(),
                &TextareaProps {
                    appearance,
                    ..Default::default()
                },
            );
            assert_eq!(
                spec.rest.border, t.color.transparent_stroke,
                "{appearance:?}"
            );
        }
        assert_eq!(
            resolve(
                &theme(),
                &TextareaProps {
                    appearance: TextareaAppearance::FilledDarker,
                    ..Default::default()
                }
            )
            .rest
            .background,
            t.color.neutral_background_3
        );
    }

    #[test]
    fn disabled_and_invalid_use_the_shared_tokens() {
        let t = Tokens::web_light();
        let disabled = resolve(
            &theme(),
            &TextareaProps {
                disabled: true,
                ..Default::default()
            },
        );
        assert_eq!(disabled.disabled.border, t.color.neutral_stroke_disabled);
        assert_eq!(disabled.disabled_text, t.color.neutral_foreground_disabled);

        let invalid = resolve(&theme(), &TextareaProps::default());
        assert_eq!(invalid.invalid_border, t.color.palette_red_border_2);
    }
}
