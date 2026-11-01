//! Fluent 2 design tokens.
//!
//! Values come from a pinned `@fluentui/tokens` snapshot: `vendor/tokens.json`
//! is produced by `scripts/snapshot-tokens.mjs`, and `generated.rs` is produced
//! from it by `cargo xtask gen-tokens`. Nothing here is hand-maintained except
//! [`CubicBezier`]'s sampling and the tests.
//!
//! This crate owns values only. It performs no theming and no rendering.

mod brand;
mod generated;

pub use brand::BrandVariants;
pub use generated::*;

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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui::{Hsla, px, rgba};

    use super::*;

    fn hex(value: u32) -> Hsla {
        Hsla::from(rgba((value << 8) | 0xff))
    }

    #[test]
    fn snapshot_size_is_what_the_docs_claim() {
        assert_eq!(TOKEN_COUNT, 459);
    }

    #[test]
    fn light_and_dark_core_values_match_fluent() {
        let light = Tokens::web_light();
        assert_eq!(light.color.neutral_background_1, hex(0xffffff));
        assert_eq!(light.color.neutral_foreground_1, hex(0x242424));
        assert_eq!(light.color.brand_background, hex(0x0f6cbd));
        assert_eq!(light.color.stroke_focus_2, hex(0x000000));

        let dark = Tokens::web_dark();
        assert_eq!(dark.color.neutral_background_1, hex(0x292929));
        assert_eq!(dark.color.neutral_foreground_1, hex(0xffffff));
        assert_eq!(dark.color.brand_background, hex(0x115ea3));
        assert_eq!(dark.color.stroke_focus_2, hex(0xffffff));
    }

    #[test]
    fn teams_theme_uses_the_teams_brand() {
        let teams = Tokens::teams_light();
        assert_eq!(teams.color.brand_background, hex(0x5b5fc7));
        assert_ne!(
            teams.color.brand_background,
            Tokens::web_light().color.brand_background
        );
    }

    #[test]
    fn high_contrast_theme_is_present_and_distinct() {
        let hc = Tokens::high_contrast();
        assert_ne!(
            hc.color.neutral_background_1,
            Tokens::web_light().color.neutral_background_1
        );
    }

    #[test]
    fn custom_brand_flows_into_brand_tokens() {
        let custom = Tokens::web_light_with(&BrandVariants::TEAMS);
        assert_eq!(custom.color.brand_background, hex(0x5b5fc7));
        // Non-brand tokens stay on the web light palette.
        assert_eq!(custom.color.neutral_background_1, hex(0xffffff));
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
    fn shadows_are_two_layers() {
        let s = Tokens::web_light().shadow.s4;
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].blur_radius, px(2.));
        assert_eq!(s[1].blur_radius, px(4.));
        assert!(!s[0].inset && !s[1].inset);
    }

    #[test]
    fn brand_ramps_are_ordered_dark_to_light() {
        let ramp = BrandVariants::WEB;
        assert_eq!(ramp.stop(80), hex(0x0f6cbd));
        let mut prev = ramp.stop(10).l;
        for stop in (20..=160).step_by(10) {
            let l = ramp.stop(stop).l;
            assert!(l > prev, "ramp must get lighter: {stop}");
            prev = l;
        }
    }
}