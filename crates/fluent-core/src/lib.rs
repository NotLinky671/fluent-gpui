//! Fluent style infrastructure shared by every component.
//!
//! - [`spec`]: pure `resolve(theme, props) -> Spec` functions, unit-testable
//!   without rendering (ADR-0002).
//! - [`gpui_compat`]: the single place that re-exports GPUI APIs we depend on,
//!   so a snapshot bump touches one file (ADR-0001).
//!
//! The focus ring lives in the components that need it: compact controls use
//! `focus_visible` with inset shadows, controls that need a ring outside their
//! bounds use a parent-side painter (ADR-0003, revised).

pub mod gpui_compat;
pub mod spec;

#[cfg(test)]
mod tests {
    /// The committed lockfile is the dependency graph of record.
    ///
    /// `gpui-component` and `gpui-kit` carry their own visual language; pulling
    /// either in would make this library a re-skin rather than an independent
    /// design system (goal G4).
    #[test]
    fn dependency_graph_stays_independent() {
        let lock = include_str!("../../../Cargo.lock");

        for forbidden in ["gpui-component", "gpui-kit", "gpui-component-macros"] {
            assert!(
                !lock.contains(&format!("name = \"{forbidden}\"")),
                "forbidden dependency in Cargo.lock: {forbidden}"
            );
        }

        // The pinned stack must still be what we think it is.
        assert!(lock.contains("name = \"gpui-pre\""));
        assert!(lock.contains("name = \"gpui-base\""));
        assert!(lock.contains("name = \"gpui-pre-platform\""));
    }
}
