//! Fluent brand ramps: 16 stops from 10 to 160.

use gpui::{Hsla, rgba};

/// A Fluent brand ramp.
///
/// Stops run from 10 (darkest) to 160 (lightest) in increments of 10, matching
/// `BrandVariants` in `@fluentui/tokens`.
///
/// Values are stored as opaque `0xRRGGBB` literals so ramps stay `const`;
/// conversion to [`Hsla`] happens on access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrandVariants {
    stops: [u32; 16],
}

impl BrandVariants {
    /// Builds a ramp from 16 opaque hex literals ordered 10, 20, … 160.
    pub const fn from_hex(stops: [u32; 16]) -> Self {
        Self { stops }
    }

    /// Returns the stop for `stop` (10..=160, multiple of 10).
    ///
    /// # Panics
    ///
    /// Panics when `stop` is outside the ramp or not a multiple of 10.
    pub fn stop(&self, stop: u16) -> Hsla {
        assert!(
            (10..=160).contains(&stop) && stop.is_multiple_of(10),
            "brand stop must be 10..=160 in steps of 10, got {stop}"
        );
        Hsla::from(rgba((self.stops[((stop - 10) / 10) as usize] << 8) | 0xff))
    }

    /// `brandWeb` from `@fluentui/tokens` — the Fluent 2 default brand.
    pub const WEB: Self = Self::from_hex([
        0x061724, 0x082338, 0x0a2e4a, 0x0c3b5e, 0x0e4775, 0x0f548c, 0x115ea3, 0x0f6cbd, 0x2886de,
        0x479ef5, 0x62abf5, 0x77b7f7, 0x96c6fa, 0xb4d6fa, 0xcfe4fa, 0xebf3fc,
    ]);

    /// `brandTeams` from `@fluentui/tokens`.
    pub const TEAMS: Self = Self::from_hex([
        0x2b2b40, 0x2f2f4a, 0x333357, 0x383966, 0x3d3e78, 0x444791, 0x4f52b2, 0x5b5fc7, 0x7579eb,
        0x7f85f5, 0x9299f7, 0xaab1fa, 0xb6bcfa, 0xc5cbfa, 0xdce0fa, 0xe8ebfa,
    ]);

    /// `brandOffice` from `@fluentui/tokens`.
    pub const OFFICE: Self = Self::from_hex([
        0x29130b, 0x4d2415, 0x792000, 0x99482b, 0xa52c00, 0xc33400, 0xe06a3f, 0xd83b01, 0xdd4f1b,
        0xfe7948, 0xff865a, 0xff9973, 0xe8825d, 0xffb498, 0xf4beaa, 0xf9dcd1,
    ]);

    /// `brandTeamsV21` from `@fluentui/tokens`.
    pub const TEAMS_V21: Self = Self::from_hex([
        0x29274f, 0x2f2a5e, 0x352e70, 0x3b3185, 0x44359e, 0x4d3aba, 0x5a40db, 0x654cf5, 0x7769fa,
        0x887dff, 0x9791ff, 0xaba8ff, 0xbab8ff, 0xc8c7ff, 0xdcdbff, 0xe8e8ff,
    ]);
}
