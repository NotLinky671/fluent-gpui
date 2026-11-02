//! Fluent motion: durations, easing curves and the transition policy shared by
//! every interactive component.
//!
//! Fluent 2 describes motion as a set of named curves and durations rather than
//! ad-hoc timings. Only the subset the components actually use is defined here;
//! add a name when a component needs it, not before.
//!
//! The values below follow Fluent's motion guidance (duration ladder 50/100/150/
//! 200/300/400/500 ms, `curveEasyEase` = cubic-bezier(0.33, 0, 0.67, 1)). They
//! have not been cross-checked against a rendered Fluent surface the way the
//! visual specs were, so treat them as the documented defaults.

use std::time::Duration;

use crate::gpui_compat::Hsla;
use gpui_base::motion::{Easing, Transition};

/// The focus underline retracting, per Fluent's input styles.
pub const DURATION_ULTRA_FAST: Duration = Duration::from_millis(50);

/// State-change colours: hover, pressed and selected.
pub const DURATION_FASTER: Duration = Duration::from_millis(100);

/// Appearance changes that move something: a thumb, an indicator bar.
pub const DURATION_NORMAL: Duration = Duration::from_millis(200);

/// Longer transitions: sheets and dialogs sliding into place.
pub const DURATION_SLOW: Duration = Duration::from_millis(300);

/// Fluent's `curveEasyEase`.
pub fn curve_easy_ease() -> Easing {
    Easing::cubic_bezier(0.33, 0.0, 0.67, 1.0).expect("curveEasyEase is a valid bezier")
}

/// Fluent's `curveDecelerateMid`: fast start, soft landing.
pub fn curve_decelerate() -> Easing {
    Easing::cubic_bezier(0.0, 0.0, 0.0, 1.0).expect("curveDecelerateMid is a valid bezier")
}

/// The policy for control state colours (hover, pressed, selected).
pub fn state_policy() -> Transition {
    Transition::new(DURATION_FASTER).easing(curve_easy_ease())
}

/// The policy for a moving part (switch thumb, tab indicator).
pub fn motion_policy() -> Transition {
    Transition::new(DURATION_NORMAL).easing(curve_easy_ease())
}

/// Fluent's `curveAccelerateMid`: slow start, quick exit.
pub fn curve_accelerate() -> Easing {
    Easing::cubic_bezier(1.0, 0.0, 1.0, 1.0).expect("curveAccelerateMid is a valid bezier")
}

/// A focus indicator growing in: 200 ms, decelerating.
pub fn reveal_policy() -> Transition {
    Transition::new(DURATION_NORMAL).easing(curve_decelerate())
}

/// A focus indicator retracting: Fluent uses the faster duration here.
pub fn hide_policy() -> Transition {
    Transition::new(DURATION_ULTRA_FAST).easing(curve_accelerate())
}

/// Mixes two colours, taking the short way around the hue circle.
///
/// Components normally animate through [`state_policy`] and GPUI's transition
/// machinery; this function is the pure kernel behind it, kept separate so the
/// arithmetic is testable without a window.
pub fn mix(from: Hsla, to: Hsla, progress: f32) -> Hsla {
    let progress = progress.clamp(0.0, 1.0);
    // Short-circuit the endpoints: the hue wrap below would otherwise land on
    // 0.6999999 instead of 0.7 through floating point error.
    if progress == 0.0 {
        return from;
    }
    if progress == 1.0 {
        return to;
    }
    let mut delta = (to.h - from.h).rem_euclid(1.0);
    if delta > 0.5 {
        delta -= 1.0;
    }
    Hsla {
        h: (from.h + delta * progress).rem_euclid(1.0),
        s: from.s + (to.s - from.s) * progress,
        l: from.l + (to.l - from.l) * progress,
        a: from.a + (to.a - from.a) * progress,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(h: f32, s: f32, l: f32, a: f32) -> Hsla {
        Hsla { h, s, l, a }
    }

    #[test]
    fn mix_returns_the_endpoints() {
        let a = color(0.1, 0.5, 0.4, 1.0);
        let b = color(0.7, 0.9, 0.8, 0.5);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
    }

    #[test]
    fn mix_clamps_out_of_range_progress() {
        let a = color(0.2, 0.5, 0.4, 1.0);
        let b = color(0.4, 0.5, 0.4, 1.0);
        assert_eq!(mix(a, b, -1.0), a);
        assert_eq!(mix(a, b, 2.0), b);
    }

    #[test]
    fn mix_takes_the_short_way_around_the_hue_circle() {
        // 350 deg -> 10 deg must travel +20 deg, not -340 deg.
        let a = color(350.0 / 360.0, 0.5, 0.5, 1.0);
        let b = color(10.0 / 360.0, 0.5, 0.5, 1.0);
        let middle = mix(a, b, 0.5);
        let degrees = middle.h * 360.0;
        assert!(!(5.0..=355.0).contains(&degrees), "wrapped to {degrees}");
    }

    #[test]
    fn mix_interpolates_lightness_and_alpha() {
        let a = color(0.0, 1.0, 0.0, 0.0);
        let b = color(0.0, 1.0, 1.0, 1.0);
        let middle = mix(a, b, 0.5);
        assert!((middle.l - 0.5).abs() < 1e-6);
        assert!((middle.a - 0.5).abs() < 1e-6);
    }

    #[test]
    fn curves_are_anchored_at_both_ends() {
        for easing in [curve_easy_ease(), curve_decelerate(), curve_accelerate()] {
            assert!(easing.sample(0.0).abs() < 1e-3);
            assert!((easing.sample(1.0) - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn the_duration_ladder_matches_fluent() {
        assert_eq!(DURATION_FASTER, Duration::from_millis(100));
        assert_eq!(DURATION_NORMAL, Duration::from_millis(200));
        assert_eq!(DURATION_SLOW, Duration::from_millis(300));
        assert_eq!(DURATION_ULTRA_FAST, Duration::from_millis(50));
        assert!(DURATION_ULTRA_FAST < DURATION_FASTER);
        assert!(DURATION_FASTER < DURATION_NORMAL && DURATION_NORMAL < DURATION_SLOW);
    }
}
