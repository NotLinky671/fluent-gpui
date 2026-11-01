//! Fluent 2 theme engine.
//!
//! Owns the global [`FluentTheme`], the built-in variants (web/teams light,
//! dark, high contrast), brand ramps, density, font mode and text scaling.
//! Projects a one-way mapping into `gpui_base::Theme` so base primitives pick
//! up Fluent colors; `gpui-base` is never the source of truth.