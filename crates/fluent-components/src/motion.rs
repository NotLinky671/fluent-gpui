//! Bridges the pure motion spec to GPUI's transition machinery.
//!
//! [`fluent_core::spec::motion`] owns the durations, curves and the colour
//! arithmetic; this module adapts a colour to `gpui_base`'s `Interpolate` trait
//! so [`gpui_base::motion::transition`] can drive it frame by frame (and honour
//! the system reduce-motion setting for us).

use fluent_core::gpui_compat::{App, Hsla, Pixels, Window};
use fluent_core::spec::motion as spec;
use gpui_base::motion::{Interpolate, Transition, TransitionId, transition};

/// A colour that can be animated by GPUI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimColor(pub Hsla);

impl Interpolate for AnimColor {
    fn interpolate(&self, target: &Self, progress: f32) -> Self {
        Self(spec::mix(self.0, target.0, progress))
    }
}

/// Animates toward `target`, returning the colour to paint this frame.
///
/// `id` must identify a single channel: two calls with the same id and different
/// targets will fight over the same keyed state.
pub fn color(
    id: impl Into<TransitionId>,
    target: Hsla,
    policy: Transition,
    window: &mut Window,
    cx: &mut App,
) -> Hsla {
    transition(id, AnimColor(target), policy, window, cx).0
}

/// Animates a control state colour with Fluent's fast, easy-ease policy.
pub fn state_color(
    id: impl Into<TransitionId>,
    target: Hsla,
    window: &mut Window,
    cx: &mut App,
) -> Hsla {
    color(id, target, spec::state_policy(), window, cx)
}
/// A length that can be animated by GPUI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimPixels(pub Pixels);

impl Interpolate for AnimPixels {
    fn interpolate(&self, target: &Self, progress: f32) -> Self {
        Self(self.0 + (target.0 - self.0) * progress)
    }
}

/// Animates a length toward `target`, returning the value to lay out this frame.
pub fn length(
    id: impl Into<TransitionId>,
    target: Pixels,
    policy: Transition,
    window: &mut Window,
    cx: &mut App,
) -> Pixels {
    transition(id, AnimPixels(target), policy, window, cx).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_core::gpui_compat::px;

    #[test]
    fn lengths_interpolate_linearly() {
        let from = AnimPixels(px(4.));
        let to = AnimPixels(px(24.));
        assert_eq!(from.interpolate(&to, 0.0), from);
        assert_eq!(from.interpolate(&to, 1.0), to);
        assert_eq!(from.interpolate(&to, 0.5), AnimPixels(px(14.)));
    }
}
/// A fraction of the parent's size, animatable by GPUI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimFraction(pub f32);

impl Interpolate for AnimFraction {
    fn interpolate(&self, target: &Self, progress: f32) -> Self {
        Self(self.0 + (target.0 - self.0) * progress)
    }
}

/// Animates a parent-relative fraction.
///
/// Used for geometry that has to scale about its own centre without knowing the
/// pixel size in advance: the input's focus underline insets from 0.5 to 0, which
/// is a horizontal collapse from the middle, exactly like Fluent's `scaleX`.
pub fn fraction(
    id: impl Into<TransitionId>,
    target: f32,
    policy: Transition,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    transition(id, AnimFraction(target), policy, window, cx).0
}
