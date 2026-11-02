//! Bridges the pure motion spec to GPUI's transition machinery.
//!
//! [`fluent_core::spec::motion`] owns the durations, curves and the colour
//! arithmetic; this module adapts a colour to `gpui_base`'s `Interpolate` trait
//! so [`gpui_base::motion::transition`] can drive it frame by frame (and honour
//! the system reduce-motion setting for us).

use fluent_core::gpui_compat::{App, Hsla, Window};
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
