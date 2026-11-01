//! Fluent 2 token values.
//!
//! Hand-written subset for M0 shape validation. T-009 replaces this module with
//! generated code covering all 467 tokens; the field naming here is the shape
//! the generator must produce.

use std::time::Duration;

use gpui::{
    BoxShadow, FontWeight, Hsla, Pixels, SharedString, hsla, point, px, rgba, transparent_black,
};

use crate::BrandVariants;

/// Opaque `0xRRGGBB` to [`Hsla`].
fn hex(value: u32) -> Hsla {
    Hsla::from(rgba((value << 8) | 0xff))
}

/// Complete token set for one appearance.
#[derive(Clone, Debug, PartialEq)]
pub struct Tokens {
    pub color: ColorTokens,
    pub font_size: FontSizeTokens,
    pub line_height: LineHeightTokens,
    pub font_weight: FontWeightTokens,
    pub font_family: FontFamilyTokens,
    pub spacing: SpacingTokens,
    pub radius: RadiusTokens,
    pub stroke_width: StrokeWidthTokens,
    pub duration: DurationTokens,
    pub curve: CurveTokens,
    pub shadow: ShadowTokens,
}

impl Tokens {
    /// Fluent `webLightTheme`: light palette on `brandWeb`.
    pub fn web_light() -> Self {
        Self::light(&BrandVariants::WEB)
    }

    /// Fluent `webDarkTheme`: dark palette on `brandWeb`.
    pub fn web_dark() -> Self {
        Self::dark(&BrandVariants::WEB)
    }

    /// `createLightTheme(brand)`.
    pub fn light(brand: &BrandVariants) -> Self {
        Self {
            color: ColorTokens::light(brand),
            font_size: FontSizeTokens::default(),
            line_height: LineHeightTokens::default(),
            font_weight: FontWeightTokens::default(),
            font_family: FontFamilyTokens::default(),
            spacing: SpacingTokens::default(),
            radius: RadiusTokens::default(),
            stroke_width: StrokeWidthTokens::default(),
            duration: DurationTokens::default(),
            curve: CurveTokens::default(),
            shadow: ShadowTokens::default(),
        }
    }

    /// `createDarkTheme(brand)`.
    pub fn dark(brand: &BrandVariants) -> Self {
        Self {
            color: ColorTokens::dark(brand),
            font_size: FontSizeTokens::default(),
            line_height: LineHeightTokens::default(),
            font_weight: FontWeightTokens::default(),
            font_family: FontFamilyTokens::default(),
            spacing: SpacingTokens::default(),
            radius: RadiusTokens::default(),
            stroke_width: StrokeWidthTokens::default(),
            duration: DurationTokens::default(),
            curve: CurveTokens::default(),
            shadow: ShadowTokens::default(),
        }
    }
}

/// Color tokens (subset used by M1 components).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorTokens {
    pub neutral_foreground_1: Hsla,
    pub neutral_foreground_1_hover: Hsla,
    pub neutral_foreground_1_pressed: Hsla,
    pub neutral_foreground_2: Hsla,
    pub neutral_foreground_3: Hsla,
    pub neutral_foreground_disabled: Hsla,
    pub neutral_foreground_on_brand: Hsla,

    pub neutral_background_1: Hsla,
    pub neutral_background_1_hover: Hsla,
    pub neutral_background_1_pressed: Hsla,
    pub neutral_background_1_selected: Hsla,
    pub neutral_background_disabled: Hsla,

    pub neutral_stroke_1: Hsla,
    pub neutral_stroke_1_hover: Hsla,
    pub neutral_stroke_1_pressed: Hsla,
    pub neutral_stroke_accessible: Hsla,
    pub neutral_stroke_accessible_hover: Hsla,
    pub neutral_stroke_accessible_pressed: Hsla,
    pub neutral_stroke_disabled: Hsla,

    pub stroke_focus_1: Hsla,
    pub stroke_focus_2: Hsla,

    pub brand_foreground_1: Hsla,
    pub brand_background: Hsla,
    pub brand_background_hover: Hsla,
    pub brand_background_pressed: Hsla,
    pub brand_background_selected: Hsla,
    pub brand_background_2: Hsla,
    pub compound_brand_background: Hsla,
    pub compound_brand_stroke: Hsla,
    pub compound_brand_stroke_hover: Hsla,
    pub compound_brand_stroke_pressed: Hsla,

    pub transparent_background: Hsla,
    pub transparent_stroke: Hsla,
    pub subtle_background: Hsla,
    pub subtle_background_hover: Hsla,
    pub subtle_background_pressed: Hsla,
}

impl ColorTokens {
    /// Values from `alias/lightColor.ts` with `brandWeb` substituted.
    pub fn light(brand: &BrandVariants) -> Self {
        Self {
            neutral_foreground_1: hex(0x242424),
            neutral_foreground_1_hover: hex(0x242424),
            neutral_foreground_1_pressed: hex(0x242424),
            neutral_foreground_2: hex(0x424242),
            neutral_foreground_3: hex(0x616161),
            neutral_foreground_disabled: hex(0xbdbdbd),
            neutral_foreground_on_brand: hex(0xffffff),

            neutral_background_1: hex(0xffffff),
            neutral_background_1_hover: hex(0xf5f5f5),
            neutral_background_1_pressed: hex(0xe0e0e0),
            neutral_background_1_selected: hex(0xebebeb),
            neutral_background_disabled: hex(0xf0f0f0),

            neutral_stroke_1: hex(0xd1d1d1),
            neutral_stroke_1_hover: hex(0xc7c7c7),
            neutral_stroke_1_pressed: hex(0xb3b3b3),
            neutral_stroke_accessible: hex(0x616161),
            neutral_stroke_accessible_hover: hex(0x575757),
            neutral_stroke_accessible_pressed: hex(0x4d4d4d),
            neutral_stroke_disabled: hex(0xe0e0e0),

            stroke_focus_1: hex(0xffffff),
            stroke_focus_2: hex(0x000000),

            brand_foreground_1: brand.stop(80),
            brand_background: brand.stop(80),
            brand_background_hover: brand.stop(70),
            brand_background_pressed: brand.stop(40),
            brand_background_selected: brand.stop(60),
            brand_background_2: brand.stop(160),
            compound_brand_background: brand.stop(80),
            compound_brand_stroke: brand.stop(80),
            compound_brand_stroke_hover: brand.stop(70),
            compound_brand_stroke_pressed: brand.stop(60),

            transparent_background: transparent_black(),
            transparent_stroke: transparent_black(),
            subtle_background: transparent_black(),
            subtle_background_hover: hex(0xf5f5f5),
            subtle_background_pressed: hex(0xe0e0e0),
        }
    }

    /// Values from `alias/darkColor.ts` with `brandWeb` substituted.
    pub fn dark(brand: &BrandVariants) -> Self {
        Self {
            neutral_foreground_1: hex(0xffffff),
            neutral_foreground_1_hover: hex(0xffffff),
            neutral_foreground_1_pressed: hex(0xffffff),
            neutral_foreground_2: hex(0xd6d6d6),
            neutral_foreground_3: hex(0xadadad),
            neutral_foreground_disabled: hex(0x5c5c5c),
            neutral_foreground_on_brand: hex(0xffffff),

            neutral_background_1: hex(0x292929),
            neutral_background_1_hover: hex(0x3d3d3d),
            neutral_background_1_pressed: hex(0x1f1f1f),
            neutral_background_1_selected: hex(0x383838),
            neutral_background_disabled: hex(0x141414),

            neutral_stroke_1: hex(0x666666),
            neutral_stroke_1_hover: hex(0x757575),
            neutral_stroke_1_pressed: hex(0x6b6b6b),
            neutral_stroke_accessible: hex(0xadadad),
            neutral_stroke_accessible_hover: hex(0xbdbdbd),
            neutral_stroke_accessible_pressed: hex(0xb3b3b3),
            neutral_stroke_disabled: hex(0x424242),

            stroke_focus_1: hex(0x000000),
            stroke_focus_2: hex(0xffffff),

            brand_foreground_1: brand.stop(100),
            brand_background: brand.stop(70),
            brand_background_hover: brand.stop(80),
            brand_background_pressed: brand.stop(40),
            brand_background_selected: brand.stop(60),
            brand_background_2: brand.stop(20),
            compound_brand_background: brand.stop(100),
            compound_brand_stroke: brand.stop(100),
            compound_brand_stroke_hover: brand.stop(110),
            compound_brand_stroke_pressed: brand.stop(90),

            transparent_background: transparent_black(),
            transparent_stroke: transparent_black(),
            subtle_background: transparent_black(),
            subtle_background_hover: hex(0x383838),
            subtle_background_pressed: hex(0x2e2e2e),
        }
    }
}

/// `fontSizeBase100…600`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontSizeTokens {
    pub base_100: Pixels,
    pub base_200: Pixels,
    pub base_300: Pixels,
    pub base_400: Pixels,
    pub base_500: Pixels,
    pub base_600: Pixels,
}

impl Default for FontSizeTokens {
    fn default() -> Self {
        Self {
            base_100: px(10.),
            base_200: px(12.),
            base_300: px(14.),
            base_400: px(16.),
            base_500: px(20.),
            base_600: px(24.),
        }
    }
}

/// `lineHeightBase100…600`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHeightTokens {
    pub base_100: Pixels,
    pub base_200: Pixels,
    pub base_300: Pixels,
    pub base_400: Pixels,
    pub base_500: Pixels,
    pub base_600: Pixels,
}

impl Default for LineHeightTokens {
    fn default() -> Self {
        Self {
            base_100: px(14.),
            base_200: px(16.),
            base_300: px(20.),
            base_400: px(22.),
            base_500: px(28.),
            base_600: px(32.),
        }
    }
}

/// `fontWeight*`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontWeightTokens {
    pub regular: FontWeight,
    pub medium: FontWeight,
    pub semibold: FontWeight,
    pub bold: FontWeight,
}

impl Default for FontWeightTokens {
    fn default() -> Self {
        Self {
            regular: FontWeight::NORMAL,
            medium: FontWeight::MEDIUM,
            semibold: FontWeight::SEMIBOLD,
            bold: FontWeight::BOLD,
        }
    }
}

/// `fontFamily*`. Resolved per platform by the theme engine (T-011).
#[derive(Clone, Debug, PartialEq)]
pub struct FontFamilyTokens {
    pub base: SharedString,
    pub monospace: SharedString,
    pub numeric: SharedString,
}

impl Default for FontFamilyTokens {
    fn default() -> Self {
        Self {
            base: "Segoe UI".into(),
            monospace: "Consolas".into(),
            numeric: "Bahnschrift".into(),
        }
    }
}

/// One spacing axis (`spacingHorizontal*` / `spacingVertical*`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpacingScale {
    pub none: Pixels,
    pub xxs: Pixels,
    pub xs: Pixels,
    pub s_nudge: Pixels,
    pub s: Pixels,
    pub m_nudge: Pixels,
    pub m: Pixels,
    pub l: Pixels,
    pub xl: Pixels,
    pub xxl: Pixels,
    pub xxxl: Pixels,
}

impl Default for SpacingScale {
    fn default() -> Self {
        Self {
            none: px(0.),
            xxs: px(2.),
            xs: px(4.),
            s_nudge: px(6.),
            s: px(8.),
            m_nudge: px(10.),
            m: px(12.),
            l: px(16.),
            xl: px(20.),
            xxl: px(24.),
            xxxl: px(32.),
        }
    }
}

/// `spacingHorizontal*` and `spacingVertical*`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SpacingTokens {
    pub horizontal: SpacingScale,
    pub vertical: SpacingScale,
}

/// `borderRadius*`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadiusTokens {
    pub none: Pixels,
    pub small: Pixels,
    pub medium: Pixels,
    pub large: Pixels,
    pub x_large: Pixels,
    pub circular: Pixels,
}

impl Default for RadiusTokens {
    fn default() -> Self {
        Self {
            none: px(0.),
            small: px(2.),
            medium: px(4.),
            large: px(6.),
            x_large: px(8.),
            circular: px(10000.),
        }
    }
}

/// `strokeWidth*`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeWidthTokens {
    pub thin: Pixels,
    pub thick: Pixels,
    pub thicker: Pixels,
    pub thickest: Pixels,
}

impl Default for StrokeWidthTokens {
    fn default() -> Self {
        Self {
            thin: px(1.),
            thick: px(2.),
            thicker: px(3.),
            thickest: px(4.),
        }
    }
}

/// `duration*`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DurationTokens {
    pub ultra_fast: Duration,
    pub faster: Duration,
    pub fast: Duration,
    pub normal: Duration,
    pub gentle: Duration,
    pub slow: Duration,
    pub slower: Duration,
    pub ultra_slow: Duration,
}

impl Default for DurationTokens {
    fn default() -> Self {
        Self {
            ultra_fast: Duration::from_millis(50),
            faster: Duration::from_millis(100),
            fast: Duration::from_millis(150),
            normal: Duration::from_millis(200),
            gentle: Duration::from_millis(250),
            slow: Duration::from_millis(300),
            slower: Duration::from_millis(400),
            ultra_slow: Duration::from_millis(500),
        }
    }
}

/// A CSS `cubic-bezier(x1, y1, x2, y2)` easing curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl CubicBezier {
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Evaluates the curve at progress `t` (0..=1) using bisection on x.
    ///
    /// Matches CSS semantics: the control points define a parametric curve and
    /// the result is y for the given x.
    pub fn sample(&self, t: f32) -> f32 {
        let t = t.clamp(0., 1.);
        let bezier = |a: f32, b: f32, u: f32| {
            let v = 1. - u;
            3. * v * v * u * a + 3. * v * u * u * b + u * u * u
        };
        let (mut lo, mut hi) = (0., 1.);
        for _ in 0..24 {
            let mid = (lo + hi) / 2.;
            if bezier(self.x1, self.x2, mid) < t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let u = (lo + hi) / 2.;
        bezier(self.y1, self.y2, u)
    }
}

/// `curve*`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveTokens {
    pub accelerate_max: CubicBezier,
    pub accelerate_mid: CubicBezier,
    pub accelerate_min: CubicBezier,
    pub decelerate_max: CubicBezier,
    pub decelerate_mid: CubicBezier,
    pub decelerate_min: CubicBezier,
    pub easy_ease_max: CubicBezier,
    pub easy_ease: CubicBezier,
    pub linear: CubicBezier,
}

impl Default for CurveTokens {
    fn default() -> Self {
        Self {
            accelerate_max: CubicBezier::new(0.9, 0.1, 1.0, 0.2),
            accelerate_mid: CubicBezier::new(1.0, 0.0, 1.0, 1.0),
            accelerate_min: CubicBezier::new(0.8, 0.0, 0.78, 1.0),
            decelerate_max: CubicBezier::new(0.1, 0.9, 0.2, 1.0),
            decelerate_mid: CubicBezier::new(0.0, 0.0, 0.0, 1.0),
            decelerate_min: CubicBezier::new(0.33, 0.0, 0.1, 1.0),
            easy_ease_max: CubicBezier::new(0.8, 0.0, 0.2, 1.0),
            easy_ease: CubicBezier::new(0.33, 0.0, 0.67, 1.0),
            linear: CubicBezier::new(0.0, 0.0, 1.0, 1.0),
        }
    }
}

/// `shadow2…64`. Each elevation is two layers: ambient (soft) + key (directional).
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowTokens {
    pub s2: [BoxShadow; 2],
    pub s4: [BoxShadow; 2],
    pub s8: [BoxShadow; 2],
    pub s16: [BoxShadow; 2],
    pub s28: [BoxShadow; 2],
    pub s64: [BoxShadow; 2],
}

impl Default for ShadowTokens {
    fn default() -> Self {
        Self {
            s2: elevation(2., 1., 0.12, 0.14),
            s4: elevation(2., 2., 0.12, 0.14),
            s8: elevation(2., 4., 0.12, 0.14),
            s16: elevation(2., 8., 0.12, 0.14),
            s28: elevation(8., 14., 0.12, 0.14),
            s64: elevation(8., 32., 0.12, 0.14),
        }
    }
}

/// Builds the ambient + key pair for one elevation step.
fn elevation(ambient_blur: f32, key_blur: f32, ambient_alpha: f32, key_alpha: f32) -> [BoxShadow; 2] {
    [
        BoxShadow {
            color: hsla(0., 0., 0., ambient_alpha),
            offset: point(px(0.), px(0.)),
            blur_radius: px(ambient_blur),
            spread_radius: px(0.),
            inset: false,
        },
        BoxShadow {
            color: hsla(0., 0., 0., key_alpha),
            offset: point(px(0.), px(key_blur / 2.)),
            blur_radius: px(key_blur),
            spread_radius: px(0.),
            inset: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_match_fluent_values() {
        let light = Tokens::web_light();
        assert_eq!(light.color.neutral_background_1, hex(0xffffff));
        assert_eq!(light.color.neutral_foreground_1, hex(0x242424));
        assert_eq!(light.color.neutral_stroke_1, hex(0xd1d1d1));
        assert_eq!(light.color.brand_background, hex(0x0f6cbd));

        let dark = Tokens::web_dark();
        assert_eq!(dark.color.neutral_background_1, hex(0x292929));
        assert_eq!(dark.color.neutral_foreground_1, hex(0xffffff));
        assert_eq!(dark.color.neutral_stroke_1, hex(0x666666));
        assert_eq!(dark.color.brand_background, hex(0x115ea3));
    }

    #[test]
    fn focus_stroke_inverts_between_appearances() {
        let light = Tokens::web_light();
        let dark = Tokens::web_dark();
        assert_eq!(light.color.stroke_focus_2, hex(0x000000));
        assert_eq!(dark.color.stroke_focus_2, hex(0xffffff));
        assert_eq!(light.color.stroke_focus_1, hex(0xffffff));
        assert_eq!(dark.color.stroke_focus_1, hex(0x000000));
    }

    #[test]
    fn scales_match_fluent_ramps() {
        let t = Tokens::web_light();
        assert_eq!(t.spacing.horizontal.m, px(12.));
        assert_eq!(t.spacing.vertical.s_nudge, px(6.));
        assert_eq!(t.radius.medium, px(4.));
        assert_eq!(t.stroke_width.thick, px(2.));
        assert_eq!(t.font_size.base_300, px(14.));
        assert_eq!(t.line_height.base_300, px(20.));
        assert_eq!(t.duration.faster, Duration::from_millis(100));
    }

    #[test]
    fn curves_are_monotonic_and_anchored() {
        let c = Tokens::web_light().curve.easy_ease;
        assert!((c.sample(0.) - 0.).abs() < 1e-3);
        assert!((c.sample(1.) - 1.).abs() < 1e-3);
        let mut prev = 0.;
        for step in 0..=20 {
            let y = c.sample(step as f32 / 20.);
            assert!(y >= prev - 1e-3, "curve must not regress: {y} < {prev}");
            prev = y;
        }
    }

    #[test]
    fn brand_ramps_are_ordered_dark_to_light() {
        let ramp = BrandVariants::WEB;
        assert_eq!(ramp.stop(80), hex(0x0f6cbd));
        assert_eq!(ramp.stop(160), hex(0xebf3fc));
        let mut prev = ramp.stop(10).l;
        for stop in (20..=160).step_by(10) {
            let l = ramp.stop(stop).l;
            assert!(l > prev, "ramp must get lighter: {stop}");
            prev = l;
        }
    }
}