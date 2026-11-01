//! Fluent style infrastructure shared by every component.
//!
//! - [`spec`]: pure `resolve(theme, props) -> Spec` functions, unit-testable
//!   without rendering (see plan/01-decisions.md ADR-0002).
//! - `focus`: the focus ring, painted outside element bounds (ADR-0003) — T-007.
//! - `gpui_compat`: the single place that re-exports GPUI APIs we depend on,
//!   so a snapshot bump touches one file (ADR-0001) — T-008.

pub mod spec;