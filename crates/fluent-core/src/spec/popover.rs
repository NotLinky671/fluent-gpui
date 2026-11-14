//! Popover surface and anchor geometry. Positioning applies these values in prepaint.
use fluent_theme::FluentTheme;
use gpui::{Pixels, px};

#[derive(Clone, Copy, Debug)]
pub struct PopoverSpec {
    pub anchor_gap: Pixels,
    pub viewport_margin: Pixels,
    pub min_width: Pixels,
    pub radius: Pixels,
    pub padding: Pixels,
}

pub fn resolve(theme: &FluentTheme) -> PopoverSpec {
    let t = theme.tokens();
    PopoverSpec {
        anchor_gap: px(4.),
        viewport_margin: px(8.),
        min_width: px(160.),
        radius: t.radius.medium,
        padding: t.spacing.horizontal.m,
    }
}
