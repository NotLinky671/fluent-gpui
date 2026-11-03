//! Fluent 2 `Dialog` / `AlertDialog` surface metrics and colours.
//!
//! Upstream (`useDialogStyles` / `useDialogSurfaceStyles` in
//! `@fluentui/react-dialog`) gives the modal a full-viewport scrim at
//! `colorBackgroundOverlay`, a centred surface on `colorNeutralBackground1`
//! with `borderRadiusXLarge` and `shadow64`, `spacingXXL` padding and a
//! `spacingS`-gapped footer. Title is `fontSizeBase500`/`semibold`,
//! body `fontSizeBase300`; both at their matching line-height tokens.
//!
//! Values Fluent does not publish as a token are marked `Approximation`
//! with the reason, not presented as official figures.

use crate::gpui_compat::{Hsla, Pixels, px};
use fluent_theme::FluentTheme;

/// Resolved dialog surface.
#[derive(Clone, Debug, PartialEq)]
pub struct DialogSpec {
    /// Full-viewport dimming layer behind the card.
    ///
    /// `colorBackgroundOverlay` — black at 40% on `web_light` /
    /// `teamsHighContrast`, 50% on `web_dark`.
    pub scrim: Hsla,
    /// Card background: `colorNeutralBackground1`.
    pub background: Hsla,
    /// Card foreground: `colorNeutralForeground1`.
    pub foreground: Hsla,
    /// Card corner radius: `borderRadiusXLarge` (8px).
    pub radius: Pixels,
    /// Card padding: `spacingXXL` (24px) on every edge.
    pub padding: Pixels,
    /// Gap between the title, the body and the footer.
    ///
    /// Fluent lays the surface out as a column with `spacingL` (16px)
    /// between the title and body rows; the footer sits at the card edge.
    pub content_gap: Pixels,
    /// Gap between footer buttons: `spacingS` (8px).
    pub footer_gap: Pixels,
    /// Title size: `fontSizeBase500` (20px).
    pub title_size: Pixels,
    /// Title line height: `lineHeightBase500` (28px).
    pub title_line_height: Pixels,
    /// Body size: `fontSizeBase300` (14px).
    pub body_size: Pixels,
    /// Body line height: `lineHeightBase300` (20px).
    pub body_line_height: Pixels,
    /// Smallest width the card may shrink to.
    ///
    /// Approximation: upstream publishes no `minWidth` token for the dialog
    /// surface; the smallest Fluent dialog example still reads correctly at
    /// 280px, so the card keeps that floor instead of collapsing under a
    /// narrow viewport.
    pub min_width: Pixels,
    /// Largest width the card may grow to.
    ///
    /// Approximation: upstream publishes no `maxWidth` token; the Fluent
    /// dialog caps its surface at 600px in the default examples, so the card
    /// keeps that ceiling. Callers can still shrink it via `max_w` styling.
    pub max_width: Pixels,
    /// Elevation: `shadow64` (ambient 0 0 8 + key 0 32 64).
    pub shadow: [gpui::BoxShadow; 2],
}

/// Resolves the Fluent dialog surface; `AlertDialog` shares every value.
pub fn dialog(theme: &FluentTheme) -> DialogSpec {
    let t = theme.tokens();
    let c = &t.color;
    DialogSpec {
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
        min_width: px(280.),
        max_width: px(600.),
        shadow: t.shadow.s64.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrim_uses_the_overlay_token() {
        let theme = FluentTheme::web_light();
        let spec = dialog(&theme);
        assert_eq!(spec.scrim, theme.tokens().color.background_overlay);
        // web_light's overlay is black at 40%; assert the resolved colour so a
        // token snapshot change cannot silently weaken the scrim.
        let rgba = spec.scrim.to_rgb();
        assert!((rgba.a - 0.4).abs() < 0.01);
    }

    #[test]
    fn surface_uses_the_published_tokens() {
        let theme = FluentTheme::web_light();
        let t = theme.tokens();
        let spec = dialog(&theme);
        assert_eq!(spec.background, t.color.neutral_background_1);
        assert_eq!(spec.foreground, t.color.neutral_foreground_1);
        assert_eq!(spec.radius, t.radius.x_large);
        assert_eq!(spec.padding, px(24.));
        assert_eq!(spec.footer_gap, px(8.));
        assert_eq!(spec.title_size, t.font_size.base_500);
        assert_eq!(spec.title_line_height, t.line_height.base_500);
        assert_eq!(spec.body_size, t.font_size.base_300);
        assert_eq!(spec.body_line_height, t.line_height.base_300);
        assert_eq!(spec.shadow, t.shadow.s64);
    }

    #[test]
    fn dark_theme_scrim_is_stronger() {
        let spec = dialog(&FluentTheme::web_dark());
        let rgba = spec.scrim.to_rgb();
        assert!((rgba.a - 0.5).abs() < 0.01);
    }
}
