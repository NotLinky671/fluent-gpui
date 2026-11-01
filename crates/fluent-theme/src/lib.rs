//! Fluent 2 theme engine.
//!
//! Owns the global [`FluentTheme`]: token set, appearance and brand ramp.
//! Projects a one-way mapping into `gpui_base::Theme` so base primitives pick
//! up Fluent colors; `gpui-base` is never the source of truth (ADR-0006).

use std::sync::Arc;

use fluent_tokens::{BrandVariants, Tokens};
use gpui::{App, Global};
use gpui_base::{Theme as BaseTheme, ThemeAppearance};

/// Which built-in palette the theme carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    Light,
    Dark,
    HighContrast,
}

/// The Fluent design system state for one application.
///
/// Cheap to clone: tokens are shared through an `Arc`.
#[derive(Clone, Debug)]
pub struct FluentTheme {
    tokens: Arc<Tokens>,
    appearance: Appearance,
    brand: BrandVariants,
}

impl FluentTheme {
    /// Fluent `webLightTheme`.
    pub fn web_light() -> Self {
        Self::new(Tokens::web_light(), Appearance::Light, BrandVariants::WEB)
    }

    /// Fluent `webDarkTheme`.
    pub fn web_dark() -> Self {
        Self::new(Tokens::web_dark(), Appearance::Dark, BrandVariants::WEB)
    }

    /// The built-in theme for an appearance, using the default web brand.
    pub fn for_appearance(appearance: Appearance) -> Self {
        match appearance {
            Appearance::Dark => Self::web_dark(),
            Appearance::Light | Appearance::HighContrast => Self::web_light(),
        }
    }

    /// A theme generated from a custom brand ramp, like `createLightTheme(brand)`.
    pub fn custom(brand: BrandVariants, appearance: Appearance) -> Self {
        let tokens = match appearance {
            Appearance::Light => Tokens::light(&brand),
            _ => Tokens::dark(&brand),
        };
        Self::new(tokens, appearance, brand)
    }

    fn new(tokens: Tokens, appearance: Appearance, brand: BrandVariants) -> Self {
        Self {
            tokens: Arc::new(tokens),
            appearance,
            brand,
        }
    }

    /// The token set this theme resolves against.
    pub fn tokens(&self) -> &Tokens {
        &self.tokens
    }

    /// Which palette is active.
    pub fn appearance(&self) -> Appearance {
        self.appearance
    }

    /// The brand ramp this theme was generated from.
    pub fn brand(&self) -> BrandVariants {
        self.brand
    }

    /// Returns the installed theme, or the default light theme when none is set.
    pub fn global(cx: &App) -> Self {
        cx.try_global::<Self>()
            .cloned()
            .unwrap_or_else(Self::web_light)
    }

    /// Mutable access to the installed theme, installing a default if needed.
    pub fn global_mut(cx: &mut App) -> &mut Self {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::web_light());
        }
        cx.global_mut::<Self>()
    }

    /// Installs this theme as the global one and projects it into `gpui-base`.
    pub fn install(&self, cx: &mut App) {
        cx.set_global(self.clone());

        let base = BaseTheme::global_mut(cx);
        base.appearance = match self.appearance {
            Appearance::Light => ThemeAppearance::Light,
            _ => ThemeAppearance::Dark,
        };

        // One-way projection: Fluent tokens are the source of truth.
        let c = &self.tokens.color;
        base.tokens.colors.background           = c.neutral_background_1;
        base.tokens.colors.foreground           = c.neutral_foreground_1;
        base.tokens.colors.surface              = c.neutral_background_1;
        base.tokens.colors.surface_foreground   = c.neutral_foreground_1;
        base.tokens.colors.primary              = c.brand_background;
        base.tokens.colors.primary_foreground   = c.neutral_foreground_on_brand;
        base.tokens.colors.secondary            = c.neutral_background_1_hover;
        base.tokens.colors.secondary_foreground = c.neutral_foreground_1;
        base.tokens.colors.muted                = c.neutral_background_1_hover;
        base.tokens.colors.muted_foreground     = c.neutral_foreground_3;
        base.tokens.colors.accent               = c.brand_background_2;
        base.tokens.colors.accent_foreground    = c.neutral_foreground_1;
        base.tokens.colors.border               = c.neutral_stroke_1;
        base.tokens.colors.input                = c.neutral_stroke_1;
        base.tokens.colors.ring                 = c.stroke_focus_2;
        base.tokens.colors.selection = c.brand_background.alpha(0.3);
        // `destructive` stays at the base default until status colors land in T-009.
    }
}

impl Global for FluentTheme {}

/// Installs the default theme. Called by `fluent_ui::init`.
pub fn init(cx: &mut App) {
    FluentTheme::web_light().install(cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_carry_the_right_appearance() {
        assert_eq!(FluentTheme::web_light().appearance(), Appearance::Light);
        assert_eq!(FluentTheme::web_dark().appearance(), Appearance::Dark);
    }

    #[test]
    fn for_appearance_selects_the_matching_palette() {
        assert_eq!(
            FluentTheme::for_appearance(Appearance::Dark).appearance(),
            Appearance::Dark
        );
        assert_eq!(
            FluentTheme::for_appearance(Appearance::Light).appearance(),
            Appearance::Light
        );
    }

    #[test]
    fn custom_brand_flows_into_tokens() {
        let theme = FluentTheme::custom(BrandVariants::TEAMS, Appearance::Light);
        assert_eq!(
            theme.tokens().color.brand_background,
            BrandVariants::TEAMS.stop(80)
        );
    }
}
