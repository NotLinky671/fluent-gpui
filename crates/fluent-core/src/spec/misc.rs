//! Divider and Link spec.
//!
//! Ported from `react-divider/.../useDividerStyles.styles.ts` and
//! `react-link/.../useLinkStyles.styles.ts`.

use fluent_theme::FluentTheme;
use gpui::{Hsla, Pixels};

/// Fluent `DividerAppearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DividerAppearance {
    #[default]
    Default,
    Subtle,
    Strong,
    Brand,
}

/// Resolved divider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DividerSpec {
    pub color: Hsla,
    pub thickness: Pixels,
}

/// Fluent `LinkAppearance` (the `brand`/`inverted` background variants map to
/// `Inverted` here).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinkAppearance {
    #[default]
    Default,
    Subtle,
    Inverted,
}

/// Resolved link colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkSpec {
    pub color: Hsla,
    pub hover_color: Hsla,
    pub pressed_color: Hsla,
    pub font_size: Pixels,
    pub line_height: Pixels,
}

/// Resolves a divider.
pub fn divider(theme: &FluentTheme, appearance: DividerAppearance) -> DividerSpec {
    let t = theme.tokens();
    let c = &t.color;
    let color = match appearance {
        DividerAppearance::Default => c.neutral_stroke_2,
        DividerAppearance::Subtle => c.neutral_stroke_3,
        DividerAppearance::Strong => c.neutral_stroke_1,
        DividerAppearance::Brand => c.brand_stroke_1,
    };
    DividerSpec {
        color,
        thickness: t.stroke_width.thin,
    }
}

/// Resolves a link for its disabled state.
pub fn link(theme: &FluentTheme, appearance: LinkAppearance, disabled: bool) -> LinkSpec {
    let t = theme.tokens();
    let c = &t.color;
    let (color, hover, pressed) = if disabled {
        (
            c.neutral_foreground_disabled,
            c.neutral_foreground_disabled,
            c.neutral_foreground_disabled,
        )
    } else {
        match appearance {
            LinkAppearance::Default => (
                c.brand_foreground_link,
                c.brand_foreground_link_hover,
                c.brand_foreground_link_pressed,
            ),
            LinkAppearance::Subtle => (
                c.neutral_foreground_2_link,
                c.neutral_foreground_2_link_hover,
                c.neutral_foreground_2_link_pressed,
            ),
            LinkAppearance::Inverted => (
                c.neutral_foreground_inverted_link,
                c.neutral_foreground_inverted_link_hover,
                c.neutral_foreground_inverted_link_pressed,
            ),
        }
    };

    LinkSpec {
        color,
        hover_color: hover,
        pressed_color: pressed,
        font_size: t.font_size.base_300,
        line_height: t.line_height.base_300,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_tokens::Tokens;

    #[test]
    fn divider_colors_follow_the_appearance_ladder() {
        let t = Tokens::web_light();
        let theme = FluentTheme::web_light();
        assert_eq!(
            divider(&theme, DividerAppearance::Default).color,
            t.color.neutral_stroke_2
        );
        assert_eq!(
            divider(&theme, DividerAppearance::Subtle).color,
            t.color.neutral_stroke_3
        );
        assert_eq!(
            divider(&theme, DividerAppearance::Strong).color,
            t.color.neutral_stroke_1
        );
        assert_eq!(
            divider(&theme, DividerAppearance::Brand).color,
            t.color.brand_stroke_1
        );
        assert_eq!(
            divider(&theme, DividerAppearance::Default).thickness,
            gpui::px(1.)
        );
    }

    #[test]
    fn link_uses_brand_colors_and_underlines_on_interaction() {
        let t = Tokens::web_light();
        let spec = link(&FluentTheme::web_light(), LinkAppearance::Default, false);
        assert_eq!(spec.color, t.color.brand_foreground_link);
        assert_eq!(spec.hover_color, t.color.brand_foreground_link_hover);
        assert_eq!(spec.pressed_color, t.color.brand_foreground_link_pressed);
    }

    #[test]
    fn subtle_and_inverted_links_switch_palette() {
        let t = Tokens::web_light();
        let subtle = link(&FluentTheme::web_light(), LinkAppearance::Subtle, false);
        assert_eq!(subtle.color, t.color.neutral_foreground_2_link);
        let inverted = link(&FluentTheme::web_light(), LinkAppearance::Inverted, false);
        assert_eq!(inverted.color, t.color.neutral_foreground_inverted_link);
    }

    #[test]
    fn disabled_link_uses_the_disabled_foreground() {
        let t = Tokens::web_light();
        let spec = link(&FluentTheme::web_light(), LinkAppearance::Default, true);
        assert_eq!(spec.color, t.color.neutral_foreground_disabled);
        assert_eq!(spec.hover_color, t.color.neutral_foreground_disabled);
    }
}
