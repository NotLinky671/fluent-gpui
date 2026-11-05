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
/// How long the pointer must rest on a submenu row before it opens.
///
/// Fluent opens submenus on a short hover delay so that sweeping the pointer
/// across a menu does not flash every branch on the way. The value is a
/// documented default rather than a measured one: no official capture of the
/// delay was available.
pub const DURATION_MENU_HOVER_OPEN: std::time::Duration = std::time::Duration::from_millis(300);
pub fn hide_policy() -> Transition {
    Transition::new(DURATION_ULTRA_FAST).easing(curve_accelerate())
}

/// A sheet (overlay drawer) sliding in or out.
///
/// Fluent names `durationSlow` for large surfaces that enter by moving. The
/// curve is `curveEasyEase` rather than a separate enter/exit pair because
/// `gpui-base`'s `Presence` drives both directions with one policy; a symmetric
/// curve keeps the exit from decelerating into a stop. Neither value has been
/// measured against a rendered Fluent drawer, so treat it as the documented
/// default rather than a captured timing.
pub fn sheet_policy() -> Transition {
    Transition::new(DURATION_SLOW).easing(curve_easy_ease())
}

/// Mixes two colours, taking the short way around the hue circle.
///
/// Channels are mixed **premultiplied by alpha**, so a fully transparent
/// endpoint contributes no colour at all. Mixing raw channels instead makes a
/// transparent-black rest state (Fluent's `subtle` and `transparent` button
/// appearances) walk through dark translucent greys on the way to its hover
/// fill: the surface flashes darker than either endpoint before settling.
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

    let alpha = from.a + (to.a - from.a) * progress;
    // Both endpoints invisible: nothing is painted, so the colour is arbitrary.
    if alpha <= f32::EPSILON {
        return Hsla { a: 0.0, ..to };
    }

    // Premultiplied weights. For two opaque colours these are `1 - progress`
    // and `progress`, so opaque mixing is unchanged.
    let weight_to = to.a * progress / alpha;
    let weight_from = from.a * (1.0 - progress) / alpha;

    let mut delta = (to.h - from.h).rem_euclid(1.0);
    if delta > 0.5 {
        delta -= 1.0;
    }
    Hsla {
        h: (from.h + delta * weight_to).rem_euclid(1.0),
        s: from.s * weight_from + to.s * weight_to,
        l: from.l * weight_from + to.l * weight_to,
        a: alpha,
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
        // Premultiplied: a fully transparent endpoint contributes no lightness, so
        // fading transparent black into white stays white while alpha rises. Mixing
        // raw channels here would give l = 0.5 — a half-transparent grey, which is
        // the dark flash this guards against.
        let a = color(0.0, 1.0, 0.0, 0.0);
        let b = color(0.0, 1.0, 1.0, 1.0);
        let middle = mix(a, b, 0.5);
        assert!(
            (middle.l - 1.0).abs() < 1e-6,
            "transparent black must not drag lightness down, got {}",
            middle.l
        );
        assert!((middle.a - 0.5).abs() < 1e-6);
    }

    #[test]
    fn fading_out_of_transparency_never_darkens_the_surface() {
        // Fluent's subtle and transparent button appearances rest on
        // `transparent_black()`. Their hover and pressed fades must only ever add
        // the target colour, never pass through a darker one.
        let rest = color(0.0, 0.0, 0.0, 0.0);
        let hover = color(0.0, 0.0, 0.96, 1.0);
        for step in 0..=20 {
            let progress = step as f32 / 20.0;
            let mixed = mix(rest, hover, progress);
            if progress > 0.0 {
                assert!(
                    (mixed.l - hover.l).abs() < 1e-5,
                    "at progress {progress} lightness dropped to {}",
                    mixed.l
                );
            }
        }
    }

    #[test]
    fn opaque_colours_still_mix_linearly() {
        let black = color(0.0, 0.0, 0.0, 1.0);
        let white = color(0.0, 0.0, 1.0, 1.0);
        let middle = mix(black, white, 0.5);
        assert!((middle.l - 0.5).abs() < 1e-6);
        assert!((middle.a - 1.0).abs() < 1e-6);
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

/// Opacity for a menu surface appearing.
///
/// Measured from a recording of the Fluent docs menu: the panel appears at its
/// final size and position and only its opacity changes, reaching ~0.76 of the
/// way in the first 100 ms of a roughly 350 ms ramp and then trailing off. A
/// quartic ease-out reproduces those samples within a few percent.
pub fn menu_reveal_opacity(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    1.0 - (1.0 - progress).powi(4)
}

/// How long the menu surface takes to fade in.
pub const DURATION_MENU_REVEAL: Duration = Duration::from_millis(350);

#[test]
fn the_menu_reveal_matches_the_recorded_fade() {
    // Samples taken from a recording of the Fluent docs menu: opacity against
    // elapsed time over a 350 ms ramp.
    let recorded = [
        (0.000_f32, 0.000_f32),
        (0.094, 0.338),
        (0.286, 0.758),
        (0.571, 0.928),
        (1.000, 1.000),
    ];
    for (progress, expected) in recorded {
        let actual = menu_reveal_opacity(progress);
        assert!(
            (actual - expected).abs() < 0.05,
            "at progress {progress} expected about {expected}, got {actual}"
        );
    }
    assert_eq!(menu_reveal_opacity(0.0), 0.0);
    assert_eq!(menu_reveal_opacity(1.0), 1.0);
}

#[test]
fn the_menu_reveal_only_rises() {
    let mut previous = menu_reveal_opacity(0.0);
    for step in 1..=20 {
        let value = menu_reveal_opacity(step as f32 / 20.0);
        assert!(value >= previous, "opacity dipped at step {step}");
        previous = value;
    }
}

/// Vertical offset of a menu surface while it fades in.
///
/// Frame differencing of the Fluent docs recording puts the menu about 4 px
/// higher early in the fade and back at its resting place by the end, so the
/// surface settles downward rather than appearing in place. The recording runs at
/// 30 fps, so the magnitude is only known to a couple of pixels.
///
/// Runs on the same curve as [`menu_reveal_opacity`] so the surface reaches its
/// resting place and full opacity at the same moment. Driving the offset linearly
/// over the same duration read as "fade in first, then slide", because the quartic
/// opacity is nearly finished while a linear offset is still halfway.
pub fn menu_reveal_offset(progress: f32) -> f32 {
    -4.0 * (1.0 - menu_reveal_opacity(progress))
}
