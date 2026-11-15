//! Fluent 2 `Menu` metrics and colours.

use crate::gpui_compat::{Hsla, Pixels, px};
use fluent_theme::FluentTheme;
use gpui::{Point, Size, point};

/// What the caller can vary on a menu item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuItemProps {
    /// Destructive action: red label, red hover.
    pub danger: bool,
    pub disabled: bool,
}

/// Resolved appearance of one menu item.
#[derive(Clone, Copy, Debug)]
pub struct MenuItemSpec {
    pub height: Pixels,
    pub padding_x: Pixels,
    pub radius: Pixels,
    pub gap: Pixels,
    pub icon_size: Pixels,
    /// Leading slot reserved for an icon or a check mark.
    ///
    /// Fluent keeps this slot in the layout even when a row has neither, so
    /// labels line up down the menu instead of shifting row by row.
    pub indicator_slot: Pixels,
    pub font_size: Pixels,
    pub line_height: Pixels,
    /// Height of a row that carries a secondary line under its label.
    pub two_line_height: Pixels,
    pub secondary_font_size: Pixels,
    pub secondary_line_height: Pixels,
    /// Muted colour of the secondary line.
    pub secondary_foreground: Hsla,
    pub background: Hsla,
    pub foreground: Hsla,
    pub hover_background: Hsla,
    pub hover_foreground: Hsla,
    /// Fill of a parent row whose submenu is open. Distinct from the current-row
    /// fill: it marks the path to the open level, not another row Enter can run.
    pub path_hint_background: Hsla,
    pub disabled_foreground: Hsla,
}

/// The floating panel a menu lives in.
#[derive(Clone, Copy, Debug)]
pub struct MenuSurfaceSpec {
    pub radius: Pixels,
    pub padding: Pixels,
    pub min_width: Pixels,
    pub background: Hsla,
    pub foreground: Hsla,
}

/// Resolves one menu item.
pub fn item(theme: &FluentTheme, props: &MenuItemProps) -> MenuItemSpec {
    let t = theme.tokens();
    let c = &t.color;

    let (foreground, hover_background, hover_foreground) = if props.danger {
        (
            c.palette_red_foreground_1,
            c.neutral_background_1_hover,
            c.palette_red_foreground_1,
        )
    } else {
        (
            c.neutral_foreground_1,
            c.neutral_background_1_hover,
            c.neutral_foreground_1,
        )
    };

    MenuItemSpec {
        height: px(32.),
        padding_x: px(12.),
        radius: px(4.),
        gap: px(8.),
        icon_size: px(20.),
        indicator_slot: px(20.),
        font_size: px(14.),
        line_height: px(20.),
        // 20px label + 16px secondary + 6px above and below. The single-line
        // row keeps 32px so a menu of plain items stays as tight as Fluent's.
        two_line_height: px(48.),
        secondary_font_size: px(12.),
        secondary_line_height: px(16.),
        secondary_foreground: c.neutral_foreground_2,
        background: c.transparent_background,
        foreground,
        hover_background,
        hover_foreground,
        path_hint_background: c.neutral_background_1_selected,
        disabled_foreground: c.neutral_foreground_disabled,
    }
}

/// Height of one row.
///
/// A row with a secondary line grows to fit it; everything else keeps the
/// single-line height. The menu's own padding and the 2px row gap are outside
/// this, so a panel's height is the sum of its rows plus that chrome.
pub fn row_height(spec: &MenuItemSpec, has_secondary: bool) -> Pixels {
    if has_secondary {
        spec.two_line_height
    } else {
        spec.height
    }
}

/// Background of one row.
///
/// Takes no hover input on purpose. Inside a menu the pointer reports into the
/// same current-row state the keyboard writes, so exactly one row can be painted
/// as current; a second hover-driven fill is what produced two rows that both
/// looked selected while Enter used only one of them.
pub fn row_background(spec: &MenuItemSpec, active: bool) -> Hsla {
    if active {
        spec.hover_background
    } else {
        spec.background
    }
}

/// Background of a parent row whose submenu is open.
///
/// This is a path hint, not a second "current" row: the keyboard target is
/// always the current row of the deepest open level, and it uses
/// [`row_background`]. Keeping the two fills different means a user cannot
/// mistake the parent row for the row Enter will run.
pub fn path_hint_background(spec: &MenuItemSpec) -> Hsla {
    spec.path_hint_background
}

/// Resolves the panel.
pub fn surface(theme: &FluentTheme) -> MenuSurfaceSpec {
    let t = theme.tokens();
    MenuSurfaceSpec {
        radius: px(4.),
        padding: px(4.),
        min_width: px(160.),
        background: t.color.neutral_background_1,
        foreground: t.color.neutral_foreground_1,
    }
}

/// The tallest a menu panel may be before its rows scroll.
///
/// A menu never outgrows the window: the same margin that keeps a popover off
/// the window edge is subtracted at the top and the bottom, and whatever is
/// left is the panel's ceiling. Sizing against the window rather than a fixed
/// cap means a long menu on a short screen still opens at its first row
/// instead of overflowing the display.
pub fn max_height(viewport_height: Pixels, viewport_margin: Pixels) -> Pixels {
    (viewport_height - viewport_margin - viewport_margin).max(px(0.))
}

/// The top-left corner a context menu opens at for a press at `pointer`.
///
/// The panel hangs down and to the right of the pointer, like the WinUI context
/// menu. When that would put it past the inset window edge on an axis, it flips
/// to the other side of the pointer so the press stays against the panel's edge
/// the menu grew from; a panel larger than the window is clamped to the margin
/// instead of flipping into a negative origin.
pub fn context_menu_origin(
    pointer: Point<Pixels>,
    panel: Size<Pixels>,
    viewport: Size<Pixels>,
    margin: Pixels,
) -> Point<Pixels> {
    let place = |at: Pixels, panel: Pixels, viewport: Pixels| {
        let limit = (viewport - panel - margin).max(margin);
        let flipped = at - panel;
        if at + panel > viewport - margin {
            flipped.clamp(margin, limit)
        } else {
            at.clamp(margin, limit)
        }
    };
    point(
        place(pointer.x, panel.width, viewport.width),
        place(pointer.y, panel.height, viewport.height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> FluentTheme {
        FluentTheme::web_light()
    }

    #[test]
    fn items_are_32px_with_12px_inset() {
        let spec = item(&theme(), &MenuItemProps::default());
        assert_eq!(spec.height, px(32.));
        assert_eq!(spec.padding_x, px(12.));
        assert_eq!(spec.radius, px(4.));
    }

    #[test]
    fn danger_items_use_the_red_ramp() {
        let t = theme();
        let plain = item(&t, &MenuItemProps::default());
        let danger = item(
            &t,
            &MenuItemProps {
                danger: true,
                disabled: false,
            },
        );
        assert_ne!(plain.foreground, danger.foreground);
        assert_eq!(danger.foreground, t.tokens().color.palette_red_foreground_1);
    }

    #[test]
    fn the_panel_uses_the_neutral_surface() {
        let t = theme();
        let spec = surface(&t);
        assert_eq!(spec.background, t.tokens().color.neutral_background_1);
        assert_eq!(spec.radius, px(4.));
    }

    #[test]
    fn only_the_current_row_is_drawn_as_current() {
        let t = theme();
        let spec = item(&t, &MenuItemProps::default());
        assert_eq!(row_background(&spec, true), spec.hover_background);
        assert_eq!(row_background(&spec, false), spec.background);
        assert_ne!(spec.background, spec.hover_background);
    }

    #[test]
    fn an_open_parent_is_a_path_hint_not_a_second_current_row() {
        let t = theme();
        let spec = item(&t, &MenuItemProps::default());
        assert_eq!(path_hint_background(&spec), spec.path_hint_background);
        assert_ne!(
            path_hint_background(&spec),
            row_background(&spec, true),
            "the expanded parent must not use the current-row fill"
        );
        assert_ne!(spec.path_hint_background, spec.background);
    }

    #[test]
    fn a_two_line_row_is_taller_and_its_extra_height_holds_the_second_line() {
        let spec = item(&theme(), &MenuItemProps::default());
        let two = row_height(&spec, true);
        assert_eq!(two, spec.two_line_height);
        assert_eq!(row_height(&spec, false), spec.height);
        assert!(two > spec.height);
        assert_eq!(
            two,
            spec.line_height + spec.secondary_line_height + px(12.),
            "the extra height must be the secondary line plus even padding"
        );
    }

    #[test]
    fn the_secondary_line_is_muted_but_still_readable() {
        let t = theme();
        let spec = item(&t, &MenuItemProps::default());
        assert_eq!(
            spec.secondary_foreground,
            t.tokens().color.neutral_foreground_2
        );
        assert_ne!(spec.secondary_foreground, spec.foreground);
    }

    #[test]
    fn a_menu_is_capped_to_the_window_less_both_margins() {
        assert_eq!(max_height(px(600.), px(8.)), px(584.));
    }

    #[test]
    fn a_window_shorter_than_the_margins_never_yields_a_negative_cap() {
        assert_eq!(max_height(px(10.), px(8.)), px(0.));
    }

    fn context_menu_probe() -> (Point<Pixels>, Size<Pixels>, Size<Pixels>, Pixels) {
        (
            point(px(200.), px(120.)),
            Size {
                width: px(160.),
                height: px(240.),
            },
            Size {
                width: px(600.),
                height: px(400.),
            },
            px(8.),
        )
    }

    #[test]
    fn a_context_menu_opens_down_and_right_from_the_pointer() {
        let (pointer, panel, viewport, margin) = context_menu_probe();
        assert_eq!(
            context_menu_origin(pointer, panel, viewport, margin),
            point(px(200.), px(120.))
        );
    }

    #[test]
    fn a_context_menu_flips_left_when_it_would_leave_the_window() {
        let (_, panel, viewport, margin) = context_menu_probe();
        let origin = context_menu_origin(point(px(560.), px(120.)), panel, viewport, margin);
        assert_eq!(
            origin.x,
            px(400.),
            "the panel keeps its right edge at the press"
        );
        assert_eq!(origin.y, px(120.));
    }

    #[test]
    fn a_context_menu_flips_up_when_it_would_leave_the_window() {
        let (_, panel, viewport, margin) = context_menu_probe();
        let origin = context_menu_origin(point(px(200.), px(360.)), panel, viewport, margin);
        assert_eq!(origin.x, px(200.));
        assert_eq!(
            origin.y,
            px(120.),
            "the panel keeps its bottom edge at the press"
        );
    }

    #[test]
    fn a_context_menu_larger_than_the_window_sits_on_the_margin() {
        let panel = Size {
            width: px(600.),
            height: px(400.),
        };
        let viewport = Size {
            width: px(600.),
            height: px(400.),
        };
        assert_eq!(
            context_menu_origin(point(px(300.), px(200.)), panel, viewport, px(8.)),
            point(px(8.), px(8.)),
            "both axes clamp instead of flipping to a negative origin"
        );
        let huge = Size {
            width: px(900.),
            height: px(700.),
        };
        assert_eq!(
            context_menu_origin(point(px(300.), px(200.)), huge, viewport, px(8.)),
            point(px(8.), px(8.)),
            "a panel that cannot fit is pinned to the margin, never off-screen"
        );
    }
}
