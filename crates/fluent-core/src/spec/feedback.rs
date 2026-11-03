//! Badge, ProgressBar and Avatar spec.

use fluent_theme::FluentTheme;
use gpui::{Hsla, Pixels, px};

/// Fluent `BadgeAppearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeAppearance {
    #[default]
    Brand,
    Danger,
    Success,
    Warning,
    Important,
    Informative,
    Subtle,
}

/// Badge sizes (Fluent's tiny/extra-small are omitted).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// Resolved badge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BadgeSpec {
    pub height: Pixels,
    pub min_width: Pixels,
    pub padding_x: Pixels,
    pub font_size: Pixels,
    pub background: Hsla,
    pub foreground: Hsla,
}

/// Resolves a badge.
///
/// Foreground selection is contrast-driven, not a literal copy of upstream's
/// mapping. Fluent's CSS pairs `paletteYellowBackground3` with
/// `neutralForeground1`, but that breaks under the bundled token sets: in
/// `web_dark` the pair is yellow text-adjacent white-on-yellow (ratio ~1.3) and
/// in `teamsHighContrastTheme` both tokens resolve to `#ffffff`, making the
/// label invisible. So each badge resolves its foreground as whichever of the
/// theme's two static label tokens — `neutralForeground1Static` (dark ink) or
/// `neutralForegroundStaticInverted` (light ink) — contrasts more with the
/// chosen background. On `web_light` and `web_dark` this reproduces upstream's
/// choices (dark ink on yellow, white ink elsewhere); on the high-contrast set,
/// where every badge background is `#ffffff`, it lands on
/// `neutralForeground1Static` (`#000000`). Per ADR-0002, rendered legibility
/// outranks the CSS constant.
pub fn badge(theme: &FluentTheme, appearance: BadgeAppearance, size: BadgeSize) -> BadgeSpec {
    let t = theme.tokens();
    let c = &t.color;
    let background = match appearance {
        BadgeAppearance::Brand => c.brand_background,
        BadgeAppearance::Danger => c.palette_red_background_3,
        BadgeAppearance::Success => c.palette_green_background_3,
        BadgeAppearance::Warning => c.palette_yellow_background_3,
        BadgeAppearance::Important => c.neutral_foreground_1,
        BadgeAppearance::Informative => c.neutral_background_5,
        BadgeAppearance::Subtle => c.neutral_background_1,
    };
    let foreground = [
        c.neutral_foreground_1_static,
        c.neutral_foreground_static_inverted,
    ]
    .into_iter()
    .max_by(|a, b| {
        contrast_ratio(*a, background)
            .partial_cmp(&contrast_ratio(*b, background))
            .unwrap()
    })
    .unwrap();
    let (height, min_width, padding_x, font_size) = match size {
        BadgeSize::Small => (px(16.), px(16.), px(4.), t.font_size.base_200),
        BadgeSize::Medium => (px(20.), px(20.), px(6.), t.font_size.base_200),
        BadgeSize::Large => (px(24.), px(24.), px(8.), t.font_size.base_300),
    };
    BadgeSpec {
        height,
        min_width,
        padding_x,
        font_size,
        background,
        foreground,
    }
}

/// Fluent `ProgressBar` appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProgressAppearance {
    #[default]
    Default,
    Success,
    Warning,
    Error,
}

/// Resolved progress bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressSpec {
    pub track: Hsla,
    pub bar: Hsla,
    pub height: Pixels,
    pub radius: Pixels,
}

/// Resolves a progress bar.
pub fn progress(theme: &FluentTheme, appearance: ProgressAppearance) -> ProgressSpec {
    let t = theme.tokens();
    let c = &t.color;
    let bar = match appearance {
        ProgressAppearance::Default => c.compound_brand_background,
        ProgressAppearance::Success => c.palette_green_background_3,
        ProgressAppearance::Warning => c.palette_dark_orange_background_3,
        ProgressAppearance::Error => c.palette_red_background_3,
    };
    ProgressSpec {
        track: c.neutral_background_6,
        bar,
        height: px(4.),
        radius: t.radius.medium,
    }
}

/// Avatar sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvatarSize {
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
}

/// Resolved avatar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvatarSpec {
    pub size: Pixels,
    pub font_size: Pixels,
    pub background: Hsla,
    pub foreground: Hsla,
}

/// Resolves an avatar.
pub fn avatar(theme: &FluentTheme, size: AvatarSize) -> AvatarSpec {
    let t = theme.tokens();
    let c = &t.color;
    let (side, font) = match size {
        AvatarSize::Small => (px(24.), px(10.)),
        AvatarSize::Medium => (px(32.), px(12.)),
        AvatarSize::Large => (px(48.), px(16.)),
        AvatarSize::ExtraLarge => (px(64.), px(20.)),
    };
    AvatarSpec {
        size: side,
        font_size: font,
        background: c.brand_background_static,
        foreground: c.neutral_foreground_on_brand,
    }
}

/// Fluent spinner sizes.
///
/// Upstream ships eight steps, `extra-tiny` 16 through `huge` 44
/// (`useSpinnerStyles.styles.ts` in `@fluentui/react-spinner`). We take the
/// first four steps — upstream `extra-tiny` 16, `tiny` 20, `extra-small` 24
/// and `small` 28 — under simplified variant names, skipping the larger
/// display sizes meant for full-page loading states that this library does
/// not target yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpinnerSize {
    /// 16px — upstream `extra-tiny`.
    Tiny,
    /// 20px — upstream `tiny`.
    ExtraSmall,
    /// 24px — upstream `extra-small`.
    Small,
    /// 28px — upstream `small`.
    #[default]
    Medium,
}

/// Resolved spinner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinnerSpec {
    /// The full-ring track colour (`colorNeutralStrokeAlpha2`).
    pub track: Hsla,
    /// The sweeping arc colour (`colorBrandStroke2Contrast`).
    pub indicator: Hsla,
    /// Outer diameter.
    pub diameter: Pixels,
    /// Ring stroke width (`strokeWidthThick` for all four of our sizes per
    /// `useSpinnerStyles.styles.ts`; `strokeWidthThicker` only starts at the
    /// 32px upstream `medium`, which we do not implement).
    pub stroke_width: Pixels,
}

/// Resolves a spinner.
pub fn spinner(theme: &FluentTheme, size: SpinnerSize) -> SpinnerSpec {
    let t = theme.tokens();
    let c = &t.color;
    let (diameter, stroke_width) = match size {
        SpinnerSize::Tiny => (px(16.), t.stroke_width.thick),
        SpinnerSize::ExtraSmall => (px(20.), t.stroke_width.thick),
        SpinnerSize::Small => (px(24.), t.stroke_width.thick),
        SpinnerSize::Medium => (px(28.), t.stroke_width.thick),
    };
    SpinnerSpec {
        track: c.neutral_stroke_alpha_2,
        indicator: c.brand_stroke_2_contrast,
        diameter,
        stroke_width,
    }
}

/// Arc end points of the spinner indicator on the unit circle.
///
/// `delta` is the animation phase in 0..1 (one full turn per period). The arc
/// starts at `delta * 360°` and sweeps `SPINNER_ARC_DEGREES`, matching Fluent's
/// indicator that covers about three quarters of the track. Angles are
/// degrees clockwise from 12 o'clock; the returned points are on the unit
/// circle centred on the origin (y down).
pub fn spinner_arc_endpoints(delta: f32) -> [(f32, f32); 2] {
    let start = (delta.rem_euclid(1.0)) * 360.0;
    let end = start + SPINNER_ARC_DEGREES;
    [circle_point(start), circle_point(end)]
}

/// Degrees of track the indicator arc covers.
///
/// Approximation: Fluent's spinner arc spans about three quarters of the ring;
/// the exact upstream figure is not published as a token. Verify against
/// official screenshots.
pub const SPINNER_ARC_DEGREES: f32 = 270.0;

/// Point on a unit circle at `degrees` clockwise from 12 o'clock (y down).
fn circle_point(degrees: f32) -> (f32, f32) {
    let rad = degrees.to_radians();
    (rad.sin(), -rad.cos())
}

/// How far the indeterminate `ProgressBar` segment has travelled at `delta`.
///
/// `delta` is the animation phase in 0..1. The segment is `SEGMENT_WIDTH`
/// (0.4) of the track wide and its start slides linearly from `-0.4` (fully
/// off the left edge) to `1.0` (fully off the right edge). The returned
/// `(left, right)` pair is clamped into the track so the rounded ends are
/// never squared off by a clip.
pub fn indeterminate_segment_insets(delta: f32) -> (f32, f32) {
    let travel = -SEGMENT_WIDTH + delta * (1.0 + SEGMENT_WIDTH);
    (travel.max(0.0), (travel + SEGMENT_WIDTH).min(1.0))
}

/// Fraction of the track the indeterminate segment occupies.
const SEGMENT_WIDTH: f32 = 0.4;

/// Whether the indeterminate `ProgressBar` may animate.
///
/// Under reduced motion the bar renders a static segment parked at the
/// reduced-motion inset instead of sweeping.
pub fn indeterminate_animation_allowed(reduce_motion: bool) -> bool {
    !reduce_motion
}

/// Fixed `(left, right)` insets the indeterminate segment parks at under
/// reduced motion: centred on the track.
pub fn indeterminate_segment_reduced_insets() -> (f32, f32) {
    let start = (1.0 - SEGMENT_WIDTH) / 2.0;
    (start, start + SEGMENT_WIDTH)
}

/// Whether the `Spinner` may spin.
///
/// Under reduced motion the same ring + arc is drawn at phase 0 with no
/// repeating animation.
pub fn spinner_animation_allowed(reduce_motion: bool) -> bool {
    !reduce_motion
}

/// Resolved tooltip surface.
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipSpec {
    pub background: Hsla,
    pub foreground: Hsla,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub padding_x: Pixels,
    pub padding_top: Pixels,
    pub padding_bottom: Pixels,
    pub radius: Pixels,
    pub max_width: Pixels,
    pub shadow: [gpui::BoxShadow; 2],
}

/// Resolves the Fluent tooltip surface (dark neutral, static colors).
pub fn tooltip(theme: &FluentTheme) -> TooltipSpec {
    let t = theme.tokens();
    let c = &t.color;
    TooltipSpec {
        background: c.neutral_background_1,
        foreground: c.neutral_foreground_1,
        font_size: t.font_size.base_200,
        line_height: t.line_height.base_200,
        padding_x: px(11.),
        padding_top: px(4.),
        padding_bottom: px(6.),
        radius: t.radius.medium,
        max_width: px(240.),
        shadow: t.shadow.s4.clone(),
    }
}

/// WCAG relative luminance of an `Hsla` color.
fn relative_luminance(color: Hsla) -> f32 {
    let c = color.to_rgb();
    fn channel(v: f32) -> f32 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
}

/// WCAG contrast ratio between two colors (1..=21).
fn contrast_ratio(a: Hsla, b: Hsla) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = (la.max(lb), la.min(lb));
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_tokens::Tokens;

    #[test]
    fn badge_foreground_contrasts_with_background_on_every_theme() {
        let appearances = [
            BadgeAppearance::Brand,
            BadgeAppearance::Danger,
            BadgeAppearance::Success,
            BadgeAppearance::Warning,
            BadgeAppearance::Important,
            BadgeAppearance::Informative,
            BadgeAppearance::Subtle,
        ];
        let themes = [
            ("web_light", FluentTheme::web_light()),
            ("web_dark", FluentTheme::web_dark()),
            ("high_contrast", FluentTheme::high_contrast()),
        ];
        let mut failures = Vec::new();
        for (theme_name, theme) in &themes {
            for appearance in appearances {
                let spec = badge(theme, appearance, BadgeSize::Medium);
                let ratio = contrast_ratio(spec.foreground, spec.background);
                if ratio < 4.5 {
                    failures.push(format!(
                        "{appearance:?} on {theme_name}: contrast {ratio:.2} < 4.5"
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn badge_sizes_match_fluent() {
        let theme = FluentTheme::web_light();
        assert_eq!(
            badge(&theme, BadgeAppearance::Brand, BadgeSize::Small).height,
            px(16.)
        );
        assert_eq!(
            badge(&theme, BadgeAppearance::Brand, BadgeSize::Medium).height,
            px(20.)
        );
        assert_eq!(
            badge(&theme, BadgeAppearance::Brand, BadgeSize::Large).height,
            px(24.)
        );
    }

    #[test]
    fn badge_appearances_use_the_status_palette() {
        let t = Tokens::web_light();
        let theme = FluentTheme::web_light();
        assert_eq!(
            badge(&theme, BadgeAppearance::Danger, BadgeSize::Medium).background,
            t.color.palette_red_background_3
        );
        assert_eq!(
            badge(&theme, BadgeAppearance::Success, BadgeSize::Medium).background,
            t.color.palette_green_background_3
        );
        assert_eq!(
            badge(&theme, BadgeAppearance::Brand, BadgeSize::Medium).background,
            t.color.brand_background
        );
    }

    #[test]
    fn progress_bar_colors_follow_appearance() {
        let t = Tokens::web_light();
        let theme = FluentTheme::web_light();
        assert_eq!(
            progress(&theme, ProgressAppearance::Default).track,
            t.color.neutral_background_6
        );
        assert_eq!(
            progress(&theme, ProgressAppearance::Default).bar,
            t.color.compound_brand_background
        );
        assert_eq!(
            progress(&theme, ProgressAppearance::Error).bar,
            t.color.palette_red_background_3
        );
        assert_eq!(progress(&theme, ProgressAppearance::Default).height, px(4.));
    }

    #[test]
    fn spinner_sizes_are_distinct() {
        let theme = FluentTheme::web_light();
        let diameters: Vec<_> = [
            SpinnerSize::Tiny,
            SpinnerSize::ExtraSmall,
            SpinnerSize::Small,
            SpinnerSize::Medium,
        ]
        .iter()
        .map(|s| spinner(&theme, *s).diameter)
        .collect();
        for pair in diameters.windows(2) {
            assert!(pair[1] > pair[0], "sizes must strictly grow: {diameters:?}");
        }
    }

    #[test]
    fn spinner_sizes_match_the_upstream_ramp() {
        // extra-tiny 16 / tiny 20 / extra-small 24 / small 28, all
        // strokeWidthThick (2px), per useSpinnerStyles.styles.ts.
        let theme = FluentTheme::web_light();
        let t = Tokens::web_light();
        let cases = [
            (SpinnerSize::Tiny, px(16.)),
            (SpinnerSize::ExtraSmall, px(20.)),
            (SpinnerSize::Small, px(24.)),
            (SpinnerSize::Medium, px(28.)),
        ];
        for (size, diameter) in cases {
            let spec = spinner(&theme, size);
            assert_eq!(spec.diameter, diameter);
            assert_eq!(spec.stroke_width, t.stroke_width.thick);
        }
    }

    #[test]
    fn spinner_colours_come_from_tokens() {
        let t = Tokens::web_light();
        let theme = FluentTheme::web_light();
        let spec = spinner(&theme, SpinnerSize::Medium);
        assert_eq!(spec.track, t.color.neutral_stroke_alpha_2);
        assert_eq!(spec.indicator, t.color.brand_stroke_2_contrast);
        assert!(spec.stroke_width > px(0.));
    }

    #[test]
    fn spinner_arc_covers_about_three_quarters() {
        let [start, end] = spinner_arc_endpoints(0.0);
        // Phase 0 starts at 12 o'clock.
        assert!((start.0 - 0.0).abs() < 1e-6 && (start.1 + 1.0).abs() < 1e-6);
        // The arc end stays on the unit circle.
        let len = (end.0 * end.0 + end.1 * end.1).sqrt();
        assert!((len - 1.0).abs() < 1e-5);
    }

    #[test]
    fn indeterminate_segment_stays_inside_the_track() {
        for step in 0..=200 {
            let delta = step as f32 / 200.0;
            let (left, right) = indeterminate_segment_insets(delta);
            assert!((0.0..=1.0).contains(&left), "left out of bounds at {delta}");
            assert!(
                right >= left && right <= 1.0,
                "right out of bounds at {delta}"
            );
            // Edges clamp while the segment slides in or out; once fully on
            // track the width is exactly SEGMENT_WIDTH.
            let width = right - left;
            assert!(
                width <= SEGMENT_WIDTH + 1e-6,
                "width {width} exceeds the segment at {delta}"
            );
            if (0.4 / 1.4..=1.0 / 1.4).contains(&delta) {
                assert!(
                    (width - SEGMENT_WIDTH).abs() < 1e-6,
                    "width {width} drifted mid-travel at {delta}"
                );
            }
        }
    }

    #[test]
    fn reduced_motion_parks_a_centred_static_segment() {
        let (left, right) = indeterminate_segment_reduced_insets();
        assert!((left - 0.3).abs() < 1e-6 && (right - 0.7).abs() < 1e-6);
        assert!(!indeterminate_animation_allowed(true));
        assert!(indeterminate_animation_allowed(false));
        assert!(!spinner_animation_allowed(true));
        assert!(spinner_animation_allowed(false));
    }

    #[test]
    fn avatar_sizes_scale() {
        let theme = FluentTheme::web_light();
        assert_eq!(avatar(&theme, AvatarSize::Small).size, px(24.));
        assert_eq!(avatar(&theme, AvatarSize::Medium).size, px(32.));
        assert_eq!(avatar(&theme, AvatarSize::ExtraLarge).size, px(64.));
    }
}
