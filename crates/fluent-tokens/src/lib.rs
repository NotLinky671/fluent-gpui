//! Fluent 2 design tokens.
//!
//! Source of truth is the pinned `@fluentui/tokens` snapshot; the generated
//! `Tokens` struct will replace the hand-written subset below in T-009.
//!
//! This crate owns values only. It performs no theming and no rendering.

mod brand;
mod tokens;

pub use brand::BrandVariants;
pub use tokens::{
    ColorTokens, CurveTokens, CubicBezier, DurationTokens, FontFamilyTokens, FontSizeTokens,
    FontWeightTokens, LineHeightTokens, RadiusTokens, ShadowTokens, SpacingTokens,
    StrokeWidthTokens, Tokens,
};