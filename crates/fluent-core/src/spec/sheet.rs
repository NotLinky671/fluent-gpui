//! Fluent 2 `Sheet` (overlay drawer) geometry and colours.
//!
//! The drawer reuses the modal surface from [`super::dialog`]: the same
//! full-viewport scrim (`colorBackgroundOverlay`), the same `neutralBackground1`
//! panel with `neutralForeground1` text and `shadow64`, and the same 24px pad.
//! The differences are the anchored edge, a fixed extent along that edge and
//! corner rounding only on the two corners that face the page.
//!
//! Upstream `@fluentui/react-drawer` keeps its drawer sizes as component
//! constants rather than design tokens. The default here is Fluent's `small`
//! drawer (320px) and is marked as an approximation, not a token lookup.

use crate::gpui_compat::{Hsla, Pixels, px};
use fluent_theme::FluentTheme;

/// The edge a sheet slides in from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SheetSide {
    /// The default: a panel on the right edge.
    #[default]
    Right,
    /// A panel on the left edge.
    Left,
    /// A panel across the top edge.
    Top,
    /// A panel across the bottom edge.
    Bottom,
}

impl SheetSide {
    /// Whether the sheet occupies a vertical edge.
    ///
    /// A left/right sheet spans the full height and [`SheetSpec::size`] is its
    /// width; a top/bottom sheet spans the full width and `size` is its height.
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

/// Resolved sheet surface.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetSpec {
    /// Full-viewport dimming layer, shared with [`super::dialog`].
    pub scrim: Hsla,
    /// Panel background: `colorNeutralBackground1`.
    pub background: Hsla,
    /// Panel foreground: `colorNeutralForeground1`.
    pub foreground: Hsla,
    /// Inner corner radius: `borderRadiusXLarge` (8px).
    ///
    /// Only the corners facing the page are rounded; the corners on the window
    /// edge stay square so the surface does not show a notch there.
    pub radius: Pixels,
    /// Panel padding on every edge: `spacingXXL` (24px).
    pub padding: Pixels,
    /// Gap between the title, the body and the footer.
    pub content_gap: Pixels,
    /// Gap between footer actions: `spacingS` (8px).
    pub footer_gap: Pixels,
    /// Title size: `fontSizeBase500` (20px).
    pub title_size: Pixels,
    /// Title line height: `lineHeightBase500` (28px).
    pub title_line_height: Pixels,
    /// Body size: `fontSizeBase300` (14px).
    pub body_size: Pixels,
    /// Body line height: `lineHeightBase300` (20px).
    pub body_line_height: Pixels,
    /// Width (left/right) or height (top/bottom) of the panel.
    ///
    /// Approximation: Fluent's drawer sizes are component constants, not
    /// theme tokens. 320px is its `small` drawer; callers can override it with
    /// `Sheet::size`.
    pub size: Pixels,
    /// Elevation: `shadow64`, matching the dialog surface.
    pub shadow: [gpui::BoxShadow; 2],
}

/// Resolves the Fluent sheet surface.
pub fn sheet(theme: &FluentTheme) -> SheetSpec {
    let t = theme.tokens();
    let c = &t.color;
    SheetSpec {
        scrim: c.background_overlay,
        background: c.neutral_background_1,
        foreground: c.neutral_foreground_1,
        radius: t.radius.x_large,
        padding: t.spacing.horizontal.xxl,
        content_gap: t.spacing.horizontal.l,
        footer_gap: t.spacing.horizontal.s,
        title_size: t.font_size.base_500,
        title_line_height: t.line_height.base_500,
        body_size: t.font_size.base_300,
        body_line_height: t.line_height.base_300,
        size: px(320.),
        shadow: t.shadow.s64.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_theme::Appearance;

    fn theme(appearance: Appearance) -> FluentTheme {
        FluentTheme::for_appearance(appearance)
    }

    #[test]
    fn the_default_side_is_the_right_edge() {
        assert_eq!(SheetSide::default(), SheetSide::Right);
        assert!(SheetSide::Left.is_horizontal());
        assert!(SheetSide::Right.is_horizontal());
        assert!(!SheetSide::Top.is_horizontal());
        assert!(!SheetSide::Bottom.is_horizontal());
    }

    #[test]
    fn the_scrim_matches_the_dialog_modal_layer() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let theme = theme(appearance);
            let sheet = sheet(&theme);
            assert_eq!(sheet.scrim, super::super::dialog::dialog(&theme).scrim);
            assert_eq!(sheet.scrim, theme.tokens().color.background_overlay);
        }
    }

    #[test]
    fn the_scrim_strength_follows_the_appearance() {
        let light = sheet(&theme(Appearance::Light)).scrim.to_rgb();
        let dark = sheet(&theme(Appearance::Dark)).scrim.to_rgb();
        assert!((light.a - 0.4).abs() < 0.01);
        assert!((dark.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn the_surface_uses_the_published_tokens() {
        let theme = theme(Appearance::Light);
        let t = theme.tokens();
        let spec = sheet(&theme);
        assert_eq!(spec.background, t.color.neutral_background_1);
        assert_eq!(spec.foreground, t.color.neutral_foreground_1);
        assert_eq!(spec.radius, t.radius.x_large);
        assert_eq!(spec.padding, px(24.));
        assert_eq!(spec.content_gap, px(16.));
        assert_eq!(spec.footer_gap, px(8.));
        assert_eq!(spec.title_size, t.font_size.base_500);
        assert_eq!(spec.title_line_height, t.line_height.base_500);
        assert_eq!(spec.body_size, t.font_size.base_300);
        assert_eq!(spec.body_line_height, t.line_height.base_300);
        assert_eq!(spec.shadow, t.shadow.s64);
    }

    #[test]
    fn the_default_extent_is_the_small_drawer() {
        assert_eq!(sheet(&theme(Appearance::Light)).size, px(320.));
    }
}
