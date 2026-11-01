//! Fluent 2 design tokens.
//!
//! Source of truth is the pinned `@fluentui/tokens` snapshot vendored under
//! `vendor/`; the `Tokens` struct in `generated/` is produced by
//! `cargo xtask gen-tokens` and must not be edited by hand.
//!
//! This crate owns values only. It performs no theming and no rendering.