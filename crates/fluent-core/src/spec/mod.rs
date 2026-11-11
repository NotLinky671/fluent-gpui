//! Pure style resolution.
//!
//! Every component answers "what does this look like" with a plain function of
//! `(FluentTheme, Props)`, never by rendering. Tests assert the numbers; the
//! render layer only applies them.

pub mod button;
pub mod dialog;
pub mod feedback;
pub mod field;
pub mod input;
pub mod menu;
pub mod misc;
pub mod motion;
pub mod selection;
pub mod sheet;
pub mod switch;
pub mod tabs;
pub mod textarea;
pub mod titlebar;

/// Shared control sizes, mirroring Fluent's `size` prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlSize {
    Small,
    #[default]
    Medium,
    Large,
}
