//! Window material policy: what a window asks the compositor for, and the layer
//! the root paints so that material can show through.
//!
//! The request and the result are different things. A window can ask for Mica,
//! the compositor can refuse (an old build, a policy that disables transparency,
//! a remote session), and the application still has to look right. [`resolve`]
//! turns the request plus what the platform reported into the two values the
//! renderer needs: the appearance to hand the window, and the colour to paint
//! behind the content. Anything short of a confirmed material yields an opaque
//! layer, so a refused request degrades to the solid window this library
//! shipped before rather than to an unreadable translucent one.

use fluent_theme::FluentTheme;
use gpui::{Hsla, WindowBackgroundAppearance};

/// The material a window asks the compositor for.
///
/// Only Windows 11 offers Mica; elsewhere a request resolves to the solid
/// fallback. The variants stay platform neutral so that callers and tests do not
/// have to branch on the target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowBackdrop {
    /// No material. The root paints the theme background opaque.
    #[default]
    Solid,
    /// Windows 11 Mica: the desktop wallpaper, blurred and tinted by the theme.
    Mica,
    /// Windows 11 Mica Alt: the same material, tuned for tabbed surfaces.
    MicaAlt,
}

/// What the platform reported for a material request.
///
/// This is deliberately not a `bool`: a refusal has to say whether the platform
/// has no such material or the call failed, because the two mean different
/// things in a report and only one of them is worth retrying.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackdropOutcome {
    /// The compositor accepted the request and reports the material back.
    Applied,
    /// The platform has no such material, or the build is too old for it.
    Unsupported(&'static str),
    /// The request reached the compositor and it returned this `HRESULT`.
    Failed(i32),
    /// The compositor answered with a different material than the one asked for.
    /// Windows accepts the call on some builds and keeps its own backdrop, so
    /// "the call returned zero" is not the same as "the material is showing".
    Mismatch { asked: i32, reported: i32 },
}

/// Opacity of the layer Fluent paints over a material.
///
/// `LayerFillColorDefault` is `#80FFFFFF` in the light theme — white at fifty
/// percent — so the same alpha keeps the dark theme's layer dark while still
/// letting the material's tone through. Measured on Windows 11 25H2 over the
/// default blue wallpaper: the Mica surface is `(239, 244, 249)`, and this layer
/// turns it into `(247, 249, 252)` — a visible tint that keeps the theme's text
/// contrast, unlike a fully transparent root, which would put body text on
/// whatever wallpaper the user chose.
pub const MATERIAL_LAYER_ALPHA: f32 = 0.5;

/// The colour and alpha the root paints behind its children.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropPolicy {
    /// Appearance to hand the window before the next frame.
    pub window: WindowBackgroundAppearance,
    /// Background the root paints behind its children.
    pub layer: Hsla,
    /// Whether the compositor material is expected to be visible. `false` means
    /// the layer is opaque and the window looks exactly like a solid one.
    pub material: bool,
}

/// The appearance to request from the window for `backdrop`.
pub fn window_appearance(backdrop: WindowBackdrop) -> WindowBackgroundAppearance {
    match backdrop {
        WindowBackdrop::Solid => WindowBackgroundAppearance::Opaque,
        WindowBackdrop::Mica => WindowBackgroundAppearance::MicaBackdrop,
        WindowBackdrop::MicaAlt => WindowBackgroundAppearance::MicaAltBackdrop,
    }
}

/// Resolves a material request and the platform's answer into what to draw.
///
/// `outcome` is ignored for [`WindowBackdrop::Solid`]: nothing was requested, so
/// there is nothing to honour. High contrast always wins — Windows asks
/// applications to drop materials when the user picks a contrast theme, and a
/// translucent layer over an unknown desktop is exactly what that setting exists
/// to prevent.
pub fn resolve(
    request: WindowBackdrop,
    outcome: BackdropOutcome,
    high_contrast: bool,
    theme: &FluentTheme,
) -> BackdropPolicy {
    let base = theme.tokens().color.neutral_background_1;
    let show_material =
        request != WindowBackdrop::Solid && outcome == BackdropOutcome::Applied && !high_contrast;

    if show_material {
        BackdropPolicy {
            window: window_appearance(request),
            layer: Hsla {
                a: MATERIAL_LAYER_ALPHA,
                ..base
            },
            material: true,
        }
    } else {
        BackdropPolicy {
            window: WindowBackgroundAppearance::Opaque,
            layer: base,
            material: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_theme::FluentTheme;
    use gpui::WindowBackgroundAppearance;

    fn theme() -> FluentTheme {
        FluentTheme::web_light()
    }

    #[test]
    fn a_solid_request_paints_the_theme_background_opaque() {
        let policy = resolve(
            WindowBackdrop::Solid,
            BackdropOutcome::Applied,
            false,
            &theme(),
        );

        assert_eq!(policy.window, WindowBackgroundAppearance::Opaque);
        assert_eq!(policy.layer.a, 1.0);
        assert_eq!(
            policy.layer,
            theme().tokens().color.neutral_background_1,
            "a solid window must keep painting the theme background"
        );
        assert!(!policy.material);
    }

    #[test]
    fn a_confirmed_material_leaves_a_translucent_layer() {
        let policy = resolve(
            WindowBackdrop::Mica,
            BackdropOutcome::Applied,
            false,
            &theme(),
        );

        assert_eq!(policy.window, WindowBackgroundAppearance::MicaBackdrop);
        assert!(policy.material);
        assert!(
            (policy.layer.a - MATERIAL_LAYER_ALPHA).abs() < 1e-6,
            "the layer must let the material through, got alpha {}",
            policy.layer.a
        );
    }

    #[test]
    fn mica_alt_asks_the_window_for_the_tabbed_material() {
        let policy = resolve(
            WindowBackdrop::MicaAlt,
            BackdropOutcome::Applied,
            false,
            &theme(),
        );

        assert_eq!(policy.window, WindowBackgroundAppearance::MicaAltBackdrop);
        assert!(policy.material);
    }

    #[test]
    fn an_unsupported_platform_falls_back_to_the_solid_window() {
        let policy = resolve(
            WindowBackdrop::Mica,
            BackdropOutcome::Unsupported("windows before build 22621"),
            false,
            &theme(),
        );

        assert_eq!(policy.window, WindowBackgroundAppearance::Opaque);
        assert_eq!(policy.layer.a, 1.0);
        assert!(!policy.material);
    }

    #[test]
    fn a_failed_request_falls_back_to_the_solid_window() {
        let policy = resolve(
            WindowBackdrop::Mica,
            BackdropOutcome::Failed(-2147024809),
            false,
            &theme(),
        );

        assert_eq!(policy.window, WindowBackgroundAppearance::Opaque);
        assert_eq!(policy.layer.a, 1.0);
        assert!(!policy.material);
    }

    #[test]
    fn a_compositor_that_answers_with_another_material_falls_back() {
        let policy = resolve(
            WindowBackdrop::Mica,
            BackdropOutcome::Mismatch {
                asked: 2,
                reported: 1,
            },
            false,
            &theme(),
        );

        assert_eq!(policy.window, WindowBackgroundAppearance::Opaque);
        assert_eq!(policy.layer.a, 1.0);
        assert!(!policy.material);
    }

    #[test]
    fn high_contrast_refuses_the_material_even_when_it_was_accepted() {
        let policy = resolve(
            WindowBackdrop::Mica,
            BackdropOutcome::Applied,
            true,
            &theme(),
        );

        assert_eq!(policy.window, WindowBackgroundAppearance::Opaque);
        assert_eq!(policy.layer.a, 1.0);
        assert!(!policy.material);
    }

    #[test]
    fn the_layer_keeps_the_theme_hue_and_takes_only_opacity_away() {
        let solid = resolve(
            WindowBackdrop::Solid,
            BackdropOutcome::Applied,
            false,
            &theme(),
        );
        let mica = resolve(
            WindowBackdrop::Mica,
            BackdropOutcome::Applied,
            false,
            &theme(),
        );

        assert_eq!(
            (mica.layer.h, mica.layer.s, mica.layer.l),
            (solid.layer.h, solid.layer.s, solid.layer.l)
        );
    }
}
