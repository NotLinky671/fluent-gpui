//! Root typography shared by Fluent application surfaces.
use fluent_theme::FluentTheme;
use gpui::Pixels;

pub fn font_size(theme: &FluentTheme) -> Pixels {
    theme.tokens().font_size.base_300
}
