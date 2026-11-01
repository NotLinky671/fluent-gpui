//! Microsoft Fluent 2 for Rust, rendered by GPUI.
//!
//! This is the only crate an application needs to depend on.
//!
//! # Getting started
//!
//! ```no_run
//! fluent_ui::application().run(|cx| {
//!     fluent_ui::init(cx);
//!     // open windows here
//! });
//! ```
//!
//! # Do not depend on `gpui` directly
//!
//! This crate is built against the `gpui-pre` snapshot that `gpui-base` pins.
//! Use [`gpui`] re-exported here instead of adding `gpui` to your own
//! `Cargo.toml`; two different gpui crates have incompatible types.