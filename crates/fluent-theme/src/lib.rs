//! Fluent 2 theme engine.
//!
//! Owns the global [`FluentTheme`]: token set, appearance, brand ramp and font
//! mode. Projects a one-way mapping into `gpui_base::Theme` so base primitives
//! pick up Fluent colors; `gpui-base` is never the source of truth (ADR-0006).

use std::sync::Arc;

use fluent_tokens::{BrandVariants, Tokens};
use gpui::{App, Global, SharedString, Subscription, Window, WindowAppearance};
use gpui_base::{Theme as BaseTheme, ThemeAppearance};

/// GPUI's name for "whatever the platform considers its UI font".
const SYSTEM_FONT: &str = ".SystemUIFont";

/// Which built-in palette the theme carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    Light,
    Dark,
    HighContrast,
}

/// Where the UI font comes from.
///
/// Fluent's own guidance is to use native system fonts, which is what [`Self::System`]
/// does. [`Self::SegoeCompat`] trades that for a consistent Segoe-like face on
/// platforms that do not ship Segoe UI (ADR-0004).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FontMode {
    /// Native UI font for the platform.
    #[default]
    System,
    /// Segoe UI where available, otherwise Selawik (metric-compatible, OFL).
    SegoeCompat,
    /// Caller-provided candidates; the first installed one wins.
    Custom(Vec<SharedString>),
}

/// The Fluent design system state for one application.
///
/// Cheap to clone: tokens are shared through an `Arc`.
#[derive(Clone, Debug)]
pub struct FluentTheme {
    tokens: Arc<Tokens>,
    appearance: Appearance,
    brand: BrandVariants,
    font_mode: FontMode,
    font_family: SharedString,
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

    /// The built-in high-contrast theme (`teamsHighContrastTheme`).
    pub fn high_contrast() -> Self {
        Self::new(
            Tokens::high_contrast(),
            Appearance::HighContrast,
            BrandVariants::WEB,
        )
    }

    /// The built-in theme for an appearance, using the default web brand.
    pub fn for_appearance(appearance: Appearance) -> Self {
        match appearance {
            Appearance::Light => Self::web_light(),
            Appearance::Dark => Self::web_dark(),
            Appearance::HighContrast => Self::high_contrast(),
        }
    }

    /// A theme generated from a custom brand ramp, like `createLightTheme(brand)`.
    /// High contrast uses the fixed accessible palette; the brand is retained
    /// for later switches back to light or dark.
    pub fn custom(brand: BrandVariants, appearance: Appearance) -> Self {
        let tokens = match appearance {
            Appearance::Light => Tokens::web_light_with(&brand),
            Appearance::Dark => Tokens::web_dark_with(&brand),
            Appearance::HighContrast => Tokens::high_contrast(),
        };
        Self::new(tokens, appearance, brand)
    }

    fn new(tokens: Tokens, appearance: Appearance, brand: BrandVariants) -> Self {
        Self {
            tokens: Arc::new(tokens),
            appearance,
            brand,
            font_mode: FontMode::System,
            font_family: SharedString::from(SYSTEM_FONT),
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

    /// The resolved UI font family.
    pub fn font_family(&self) -> &SharedString {
        &self.font_family
    }

    /// Which font mode this theme was configured with.
    pub fn font_mode(&self) -> &FontMode {
        &self.font_mode
    }

    /// Rebuilds the theme for another appearance, keeping brand and font mode.
    pub fn with_appearance(&self, appearance: Appearance) -> Self {
        if appearance == self.appearance {
            return self.clone();
        }
        let tokens = match appearance {
            Appearance::Light => Tokens::web_light_with(&self.brand),
            Appearance::Dark => Tokens::web_dark_with(&self.brand),
            Appearance::HighContrast => Tokens::high_contrast(),
        };
        Self {
            tokens: Arc::new(tokens),
            appearance,
            brand: self.brand,
            font_mode: self.font_mode.clone(),
            font_family: self.font_family.clone(),
        }
    }

    /// Sets the font mode; resolved on the next [`Self::install`].
    pub fn with_font_mode(mut self, font_mode: FontMode) -> Self {
        self.font_mode = font_mode;
        self
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
    ///
    /// Resolves the font family against the platform's installed fonts first,
    /// so [`Self::font_family`] is always a family the text system knows.
    pub fn install(&self, cx: &mut App) {
        let mut theme = self.clone();
        theme.font_family = theme.resolve_font(cx);
        cx.set_global(theme.clone());

        let base = BaseTheme::global_mut(cx);
        base.appearance = match theme.appearance {
            Appearance::Light => ThemeAppearance::Light,
            _ => ThemeAppearance::Dark,
        };

        // One-way projection: Fluent tokens are the source of truth.
        let c = &theme.tokens.color;
        base.tokens.colors.background = c.neutral_background_1;
        base.tokens.colors.foreground = c.neutral_foreground_1;
        base.tokens.colors.surface = c.neutral_background_1;
        base.tokens.colors.surface_foreground = c.neutral_foreground_1;
        base.tokens.colors.primary = c.brand_background;
        base.tokens.colors.primary_foreground = c.neutral_foreground_on_brand;
        base.tokens.colors.secondary = c.neutral_background_1_hover;
        base.tokens.colors.secondary_foreground = c.neutral_foreground_1;
        base.tokens.colors.muted = c.neutral_background_1_hover;
        base.tokens.colors.muted_foreground = c.neutral_foreground_3;
        base.tokens.colors.accent = c.brand_background_2;
        base.tokens.colors.accent_foreground = c.neutral_foreground_1;
        base.tokens.colors.border = c.neutral_stroke_1;
        base.tokens.colors.input = c.neutral_stroke_1;
        base.tokens.colors.ring = c.stroke_focus_2;
        base.tokens.colors.selection = c.brand_background.alpha(0.3);
        // `destructive` stays at the base default until status colors are wired.
    }

    /// Switches the installed theme whenever the OS appearance changes.
    ///
    /// The callback fires once immediately, so the window starts on the system
    /// appearance. Keep the returned [`Subscription`] alive for as long as the
    /// window should track the system. A manually selected high-contrast palette
    /// is preserved; select light/dark explicitly to resume following the OS.
    pub fn follow_system(window: &mut Window) -> Subscription {
        window.observe_window_appearance(|window, cx| {
            let current = Self::global(cx);
            let appearance = system_appearance(current.appearance(), window.appearance());
            if current.appearance() != appearance {
                current.with_appearance(appearance).install(cx);
            }
        })
    }

    fn resolve_font(&self, cx: &App) -> SharedString {
        let available = cx.text_system().all_font_names();
        let candidates: Vec<SharedString> = match &self.font_mode {
            FontMode::System => system_candidates(),
            FontMode::SegoeCompat => vec![
                SharedString::from("Segoe UI"),
                SharedString::from("Selawik"),
            ],
            FontMode::Custom(list) => list.clone(),
        };

        candidates
            .into_iter()
            .find(|candidate| {
                let name = candidate.as_ref();
                name == SYSTEM_FONT || available.iter().any(|a| a.eq_ignore_ascii_case(name))
            })
            .unwrap_or_else(|| SharedString::from(SYSTEM_FONT))
    }
}

fn system_appearance(current: Appearance, system: WindowAppearance) -> Appearance {
    if current == Appearance::HighContrast {
        return current;
    }
    match system {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => Appearance::Light,
    }
}

fn system_candidates() -> Vec<SharedString> {
    if cfg!(target_os = "windows") {
        vec![
            SharedString::from("Segoe UI Variable Text"),
            SharedString::from("Segoe UI"),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            SharedString::from(SYSTEM_FONT),
            SharedString::from("SF Pro Text"),
            SharedString::from("Helvetica Neue"),
        ]
    } else {
        vec![
            SharedString::from("Inter"),
            SharedString::from("Noto Sans"),
            SharedString::from("DejaVu Sans"),
        ]
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
    fn custom_high_contrast_uses_the_fixed_palette_and_retains_brand() {
        let custom = FluentTheme::custom(BrandVariants::TEAMS, Appearance::HighContrast);
        let fixed = FluentTheme::high_contrast();
        assert_eq!(custom.tokens().color, fixed.tokens().color);
        assert_eq!(custom.brand(), BrandVariants::TEAMS);
        assert_eq!(
            custom.with_appearance(Appearance::Dark).tokens().color,
            FluentTheme::custom(BrandVariants::TEAMS, Appearance::Dark)
                .tokens()
                .color
        );
    }

    #[test]
    fn system_tracking_preserves_high_contrast_and_maps_both_normal_palettes() {
        for system in [
            WindowAppearance::Light,
            WindowAppearance::VibrantLight,
            WindowAppearance::Dark,
            WindowAppearance::VibrantDark,
        ] {
            assert_eq!(
                system_appearance(Appearance::HighContrast, system),
                Appearance::HighContrast
            );
        }
        assert_eq!(
            system_appearance(Appearance::Dark, WindowAppearance::Light),
            Appearance::Light
        );
        assert_eq!(
            system_appearance(Appearance::Light, WindowAppearance::Dark),
            Appearance::Dark
        );
    }

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
    fn high_contrast_is_not_the_light_theme() {
        let hc = FluentTheme::for_appearance(Appearance::HighContrast);
        assert_eq!(hc.appearance(), Appearance::HighContrast);
        assert_ne!(
            hc.tokens().color.neutral_background_1,
            FluentTheme::web_light().tokens().color.neutral_background_1
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

    #[test]
    fn with_appearance_keeps_brand_and_font_mode() {
        let theme = FluentTheme::custom(BrandVariants::TEAMS, Appearance::Light)
            .with_font_mode(FontMode::SegoeCompat);
        let dark = theme.with_appearance(Appearance::Dark);

        assert_eq!(dark.appearance(), Appearance::Dark);
        assert_eq!(dark.brand(), BrandVariants::TEAMS);
        assert_eq!(dark.font_mode(), &FontMode::SegoeCompat);
        assert_eq!(
            dark.tokens().color.brand_background,
            BrandVariants::TEAMS.stop(70)
        );
    }

    #[test]
    fn switching_to_the_same_appearance_is_a_no_op() {
        let theme = FluentTheme::web_light();
        let same = theme.with_appearance(Appearance::Light);
        assert_eq!(same.appearance(), theme.appearance());
    }
}
