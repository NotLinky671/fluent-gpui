//! Badge, ProgressBar and Avatar spec.

use fluent_theme::FluentTheme;
use gpui::{Hsla, Pixels, px};

/// Fluent `BadgeAppearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeAppearance {
    #[default]
    Brand,
    Danger,
    Success,
    Warning,
    Important,
    Informative,
    Subtle,
}

/// Badge sizes (Fluent's tiny/extra-small are omitted).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// Resolved badge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BadgeSpec {
    pub height: Pixels,
    pub min_width: Pixels,
    pub padding_x: Pixels,
    pub font_size: Pixels,
    pub background: Hsla,
    pub foreground: Hsla,
}

/// Resolves a badge.
pub fn badge(theme: &FluentTheme, appearance: BadgeAppearance, size: BadgeSize) -> BadgeSpec {
    let t = theme.tokens();
    let c = &t.color;
    let (background, foreground) = match appearance {
        BadgeAppearance::Brand => (c.brand_background, c.neutral_foreground_on_brand),
        BadgeAppearance::Danger => (c.palette_red_background_3, c.neutral_foreground_on_brand),
        BadgeAppearance::Success => (c.palette_green_background_3, c.neutral_foreground_on_brand),
        BadgeAppearance::Warning => (c.palette_yellow_background_3, c.neutral_foreground_1),
        BadgeAppearance::Important => (c.neutral_foreground_1, c.neutral_foreground_on_brand),
        BadgeAppearance::Informative => (c.neutral_background_5, c.neutral_foreground_1),
        BadgeAppearance::Subtle => (c.neutral_background_1, c.neutral_foreground_1),
    };
    let (height, min_width, padding_x, font_size) = match size {
        BadgeSize::Small => (px(16.), px(16.), px(4.), t.font_size.base_200),
        BadgeSize::Medium => (px(20.), px(20.), px(6.), t.font_size.base_200),
        BadgeSize::Large => (px(24.), px(24.), px(8.), t.font_size.base_300),
    };
    BadgeSpec {
        height,
        min_width,
        padding_x,
        font_size,
        background,
        foreground,
    }
}

/// Fluent `ProgressBar` appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProgressAppearance {
    #[default]
    Default,
    Success,
    Warning,
    Error,
}

/// Resolved progress bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressSpec {
    pub track: Hsla,
    pub bar: Hsla,
    pub height: Pixels,
    pub radius: Pixels,
}

/// Resolves a progress bar.
pub fn progress(theme: &FluentTheme, appearance: ProgressAppearance) -> ProgressSpec {
    let t = theme.tokens();
    let c = &t.color;
    let bar = match appearance {
        ProgressAppearance::Default => c.compound_brand_background,
        ProgressAppearance::Success => c.palette_green_background_3,
        ProgressAppearance::Warning => c.palette_dark_orange_background_3,
        ProgressAppearance::Error => c.palette_red_background_3,
    };
    ProgressSpec {
        track: c.neutral_background_6,
        bar,
        height: px(4.),
        radius: t.radius.medium,
    }
}

/// Avatar sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvatarSize {
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
}

/// Resolved avatar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvatarSpec {
    pub size: Pixels,
    pub font_size: Pixels,
    pub background: Hsla,
    pub foreground: Hsla,
}

/// Resolves an avatar.
pub fn avatar(theme: &FluentTheme, size: AvatarSize) -> AvatarSpec {
    let t = theme.tokens();
    let c = &t.color;
    let (side, font) = match size {
        AvatarSize::Small => (px(24.), px(10.)),
        AvatarSize::Medium => (px(32.), px(12.)),
        AvatarSize::Large => (px(48.), px(16.)),
        AvatarSize::ExtraLarge => (px(64.), px(20.)),
    };
    AvatarSpec {
        size: side,
        font_size: font,
        background: c.brand_background_static,
        foreground: c.neutral_foreground_on_brand,
    }
}

/// Resolved tooltip surface.
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipSpec {
    pub background: Hsla,
    pub foreground: Hsla,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub padding_x: Pixels,
    pub padding_top: Pixels,
    pub padding_bottom: Pixels,
    pub radius: Pixels,
    pub max_width: Pixels,
    pub shadow: [gpui::BoxShadow; 2],
}

/// Resolves the Fluent tooltip surface (dark neutral, static colors).
pub fn tooltip(theme: &FluentTheme) -> TooltipSpec {
    let t = theme.tokens();
    let c = &t.color;
    TooltipSpec {
        background: c.neutral_background_1,
        foreground: c.neutral_foreground_1,
        font_size: t.font_size.base_200,
        line_height: t.line_height.base_200,
        padding_x: px(11.),
        padding_top: px(4.),
        padding_bottom: px(6.),
        radius: t.radius.medium,
        max_width: px(240.),
        shadow: t.shadow.s4.clone(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use fluent_tokens::Tokens;

    #[test]
    fn badge_sizes_match_fluent() {
        let theme = FluentTheme::web_light();
        assert_eq!(
            badge(&theme, BadgeAppearance::Brand, BadgeSize::Small).height,
            px(16.)
        );
        assert_eq!(
            badge(&theme, BadgeAppearance::Brand, BadgeSize::Medium).height,
            px(20.)
        );
        assert_eq!(
            badge(&theme, BadgeAppearance::Brand, BadgeSize::Large).height,
            px(24.)
        );
    }

    #[test]
    fn badge_appearances_use_the_status_palette() {
        let t = Tokens::web_light();
        let theme = FluentTheme::web_light();
        assert_eq!(
            badge(&theme, BadgeAppearance::Danger, BadgeSize::Medium).background,
            t.color.palette_red_background_3
        );
        assert_eq!(
            badge(&theme, BadgeAppearance::Success, BadgeSize::Medium).background,
            t.color.palette_green_background_3
        );
        assert_eq!(
            badge(&theme, BadgeAppearance::Brand, BadgeSize::Medium).background,
            t.color.brand_background
        );
    }

    #[test]
    fn progress_bar_colors_follow_appearance() {
        let t = Tokens::web_light();
        let theme = FluentTheme::web_light();
        assert_eq!(
            progress(&theme, ProgressAppearance::Default).track,
            t.color.neutral_background_6
        );
        assert_eq!(
            progress(&theme, ProgressAppearance::Default).bar,
            t.color.compound_brand_background
        );
        assert_eq!(
            progress(&theme, ProgressAppearance::Error).bar,
            t.color.palette_red_background_3
        );
        assert_eq!(progress(&theme, ProgressAppearance::Default).height, px(4.));
    }

    #[test]
    fn avatar_sizes_scale() {
        let theme = FluentTheme::web_light();
        assert_eq!(avatar(&theme, AvatarSize::Small).size, px(24.));
        assert_eq!(avatar(&theme, AvatarSize::Medium).size, px(32.));
        assert_eq!(avatar(&theme, AvatarSize::ExtraLarge).size, px(64.));
    }
}
