//! Fluent System Icons (MIT), embedded as raw SVG bytes.
//!
//! This crate is pure data: it has no GPUI dependency, so it can be used from
//! any layer. The rendering element lives in `fluent-components::icon` and
//! feeds [`FluentIcon::svg`] into `svg().data(..)`, which needs no
//! `AssetSource` registration and therefore cannot clash with the host
//! application's own assets (ADR-0005).
//!
//! Only icons the components actually use are embedded; add more by editing
//! `scripts/snapshot-icons.mjs` and re-running the generator.

mod generated;

pub use generated::FluentIcon;

/// Fluent icon style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IconStyle {
    #[default]
    Regular,
    Filled,
}

impl FluentIcon {
    /// Raw SVG bytes for one style.
    pub fn bytes(self, style: IconStyle) -> &'static [u8] {
        self.svg(style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_both_styles() {
        assert!(!FluentIcon::ALL.is_empty());
        for icon in FluentIcon::ALL {
            for style in [IconStyle::Regular, IconStyle::Filled] {
                let svg = icon.svg(style);
                assert!(
                    svg.starts_with(b"<svg"),
                    "{} {:?} is not an svg",
                    icon.name(),
                    style
                );
                assert!(svg.len() > 100, "{} {:?} looks empty", icon.name(), style);
            }
        }
    }

    #[test]
    fn names_are_stable_snake_case() {
        assert_eq!(FluentIcon::ChevronDown.name(), "chevron_down");
        assert_eq!(FluentIcon::MoreHorizontal.name(), "more_horizontal");
        assert_eq!(FluentIcon::CheckmarkCircle.name(), "checkmark_circle");
    }
}