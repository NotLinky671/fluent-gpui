//! Fluent 2 `Tabs` geometry and colours.
//!
//! Follows the same layering as the other controls: this module only turns tokens
//! into numbers and colours, so the arithmetic is testable without a window.

use fluent_theme::FluentTheme;
use gpui::Hsla;

use crate::gpui_compat::{Pixels, px};

/// Which way the tab strip runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabsOrientation {
    Horizontal,
    Vertical,
}

impl TabsOrientation {
    /// Whether the indicator sits under (horizontal) or beside (vertical) a tab.
    pub fn is_horizontal(self) -> bool {
        self == TabsOrientation::Horizontal
    }
}

/// One tab's geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabMetrics {
    pub height: Pixels,
    pub padding_x: Pixels,
    pub gap: Pixels,
    pub font_size: Pixels,
    pub line_height: Pixels,
    /// Thickness of the selected indicator bar.
    pub indicator_thickness: Pixels,
    /// Gap between the strip and the following content is the caller's business;
    /// this is the spacing between neighbouring tabs.
    pub tab_gap: Pixels,
    /// Width of the strip in the vertical orientation.
    pub vertical_width: Pixels,
    /// Tab corner radius.
    pub radius: Pixels,
}

/// The colours one tab paints with in a given state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabColors {
    pub foreground: Hsla,
    pub background: Hsla,
}

/// Everything the renderer needs for one tab strip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabsSpec {
    pub metrics: TabMetrics,
    /// Selected indicator colour.
    pub indicator: Hsla,
    pub rest: TabColors,
    pub hover: TabColors,
    pub pressed: TabColors,
    pub selected: TabColors,
    pub disabled: TabColors,
}

/// Resolves the strip's spec.
pub fn list(theme: &FluentTheme, orientation: TabsOrientation) -> TabsSpec {
    let c = &theme.tokens().color;
    let transparent = c.transparent_background;

    let metrics = TabMetrics {
        height: px(40.),
        padding_x: px(12.),
        gap: px(8.),
        font_size: px(14.),
        line_height: px(20.),
        indicator_thickness: px(2.),
        tab_gap: px(4.),
        vertical_width: px(160.),
        radius: px(4.),
    };

    let colors = |foreground: Hsla, background: Hsla| TabColors {
        foreground,
        background,
    };

    TabsSpec {
        metrics: if orientation.is_horizontal() {
            metrics
        } else {
            TabMetrics {
                height: px(36.),
                ..metrics
            }
        },
        indicator: c.brand_stroke_1,
        rest: colors(c.neutral_foreground_2, transparent),
        hover: colors(c.neutral_foreground_2_hover, c.subtle_background_hover),
        pressed: colors(c.neutral_foreground_2_pressed, c.subtle_background_pressed),
        selected: colors(c.neutral_foreground_1, transparent),
        disabled: colors(c.neutral_foreground_disabled, transparent),
    }
}

/// The colours a tab paints with.
///
/// Selected outranks hover: Fluent keeps the selected tab's label emphasised even
/// while the pointer is over it.
pub fn tab_colors(
    spec: &TabsSpec,
    selected: bool,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> TabColors {
    if disabled {
        return spec.disabled;
    }
    if selected {
        return spec.selected;
    }
    if pressed {
        return spec.pressed;
    }
    if hovered {
        return spec.hover;
    }
    spec.rest
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_theme::{Appearance, FluentTheme};

    fn theme() -> FluentTheme {
        FluentTheme::for_appearance(Appearance::Light)
    }

    #[test]
    fn the_strip_uses_fluent_tab_metrics() {
        let spec = list(&theme(), TabsOrientation::Horizontal);
        assert_eq!(spec.metrics.height, px(40.));
        assert_eq!(spec.metrics.padding_x, px(12.));
        assert_eq!(spec.metrics.indicator_thickness, px(2.));
    }

    #[test]
    fn the_vertical_orientation_narrows_the_tabs() {
        let horizontal = list(&theme(), TabsOrientation::Horizontal);
        let vertical = list(&theme(), TabsOrientation::Vertical);
        assert!(vertical.metrics.height < horizontal.metrics.height);
        assert_eq!(vertical.metrics.vertical_width, px(160.));
    }

    #[test]
    fn the_indicator_is_the_brand_stroke() {
        let t = theme();
        let spec = list(&t, TabsOrientation::Horizontal);
        assert_eq!(spec.indicator, t.tokens().color.brand_stroke_1);
    }

    #[test]
    fn selected_outranks_hover() {
        let spec = list(&theme(), TabsOrientation::Horizontal);
        let selected = tab_colors(&spec, true, true, false, false);
        assert_eq!(selected, spec.selected);
        assert_ne!(selected, spec.hover);
    }

    #[test]
    fn disabled_outranks_everything() {
        let spec = list(&theme(), TabsOrientation::Horizontal);
        let disabled = tab_colors(&spec, true, true, true, true);
        assert_eq!(disabled, spec.disabled);
    }

    #[test]
    fn hover_and_pressed_use_the_subtle_fills() {
        let spec = list(&theme(), TabsOrientation::Horizontal);
        assert_ne!(spec.hover.background, spec.rest.background);
        assert_ne!(spec.pressed.background, spec.hover.background);
    }
}
