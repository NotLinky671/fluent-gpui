//! Fluent 2 `Accordion` geometry, colours and open-state arithmetic.
//!
//! Values come from `@fluentui/react-accordion@9.12.5`
//! (`AccordionHeader/useAccordionHeaderStyles.styles.js` and
//! `AccordionPanel/useAccordionPanelStyles.styles.js`), which is what the design
//! guidance on fluent2.microsoft.design describes:
//!
//! - the header button is `min-height: 44px`, `font-size: var(--fontSizeBase300)`
//!   (`line-height: var(--lineHeightBase300)`), `padding: 0 var(--spacingHorizontalM) 0
//!   var(--spacingHorizontalMNudge)` when the chevron sits at the start;
//! - `size="small"` drops to `min-height: 32px` and `var(--fontSizeBase200)`, while
//!   `large` and `extra-large` only change the type ramp;
//! - the chevron is a `ChevronRight` at `var(--fontSizeBase500)`, padded by
//!   `var(--spacingHorizontalS)`, pointing down when the panel is open;
//! - the panel is `margin: 0 var(--spacingHorizontalM)`, with no padding of its own;
//! - the focus indicator is a 2px `var(--colorStrokeFocus2)` rectangle at
//!   `var(--borderRadiusMedium)`.
//!
//! The expanding panel comes from `Collapse`, which is what
//! `AccordionPanel` renders through `collapseMotion` with `visible: open` and
//! `unmountOnExit: true`. `@fluentui/react-motion-components-preview@0.15.10`
//! defaults to `duration = motionTokens.durationNormal` (200 ms),
//! `easing = motionTokens.curveEasyEaseMax`, `staggerDelay = 0` and
//! `animateOpacity: true`, so the height and the opacity move on one clock and
//! one curve.
//!
//! One thing cannot be reproduced in this GPUI snapshot and is recorded in
//! `docs/validation/accordion-2026-11-17.md`: there is no element transform, so
//! the chevron is swapped rather than rotated.

use std::time::Duration;

use fluent_theme::FluentTheme;
use fluent_tokens::CubicBezier;
use gpui::Hsla;

use crate::gpui_compat::{Pixels, px};

/// `motionTokens.durationNormal`: a collapsing panel takes 200 ms both ways.
pub const PANEL_MOTION: Duration = Duration::from_millis(200);

/// The size and the opacity of a collapsing panel at `progress` (0..=1).
///
/// Both ride `curveEasyEaseMax` because `staggerDelay` is zero, so the panel is
/// half revealed in both senses at the same moment. Returning them together
/// keeps the two in lockstep: a caller cannot animate one without the other.
pub fn panel_motion(progress: f32, ease: CubicBezier) -> (f32, f32) {
    let eased = ease.sample(progress.clamp(0., 1.));
    (eased, eased)
}

/// The header size ramp, mirroring Fluent's `size` prop.
///
/// `Medium` is the default; `Large` and `ExtraLarge` keep the 44px row and only
/// move up the type scale, exactly like the upstream stylesheet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccordionSize {
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
}

/// One header row's geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccordionHeaderMetrics {
    /// Row height; the header is `min-height` in the stylesheet, and every
    /// Fluent header is a single line, so it is also the height here.
    pub height: Pixels,
    /// Leading padding, before the chevron.
    pub padding_start: Pixels,
    /// Trailing padding, after the label.
    pub padding_end: Pixels,
    /// Gap between the chevron and the label.
    pub gap: Pixels,
    pub chevron: Pixels,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub radius: Pixels,
    /// Focus rectangle thickness, drawn inside the row.
    pub focus_thickness: Pixels,
}

/// The panel's geometry. React only gives the panel horizontal margins; vertical
/// rhythm is the caller's, so it stays zero here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccordionPanelMetrics {
    pub margin_start: Pixels,
    pub margin_end: Pixels,
    pub padding: Pixels,
}

/// Colours one header paints with in a given state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccordionHeaderColors {
    pub foreground: Hsla,
    pub background: Hsla,
}

/// Everything the renderer needs for one accordion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccordionSpec {
    pub header: AccordionHeaderMetrics,
    pub panel: AccordionPanelMetrics,
    pub rest: AccordionHeaderColors,
    pub hover: AccordionHeaderColors,
    pub pressed: AccordionHeaderColors,
    pub disabled: AccordionHeaderColors,
    pub focus_ring: Hsla,
}

/// Resolves the accordion's spec for one size.
pub fn spec(theme: &FluentTheme, size: AccordionSize) -> AccordionSpec {
    let t = theme.tokens();
    let c = &t.color;

    let mut header = AccordionHeaderMetrics {
        height: px(44.),
        padding_start: t.spacing.horizontal.m_nudge,
        padding_end: t.spacing.horizontal.m,
        gap: t.spacing.horizontal.s,
        chevron: t.font_size.base_500,
        font_size: t.font_size.base_300,
        line_height: t.line_height.base_300,
        radius: t.radius.medium,
        focus_thickness: t.stroke_width.thick,
    };

    match size {
        AccordionSize::Small => {
            header.height = px(32.);
            header.font_size = t.font_size.base_200;
            header.line_height = t.line_height.base_200;
        }
        AccordionSize::Medium => {}
        AccordionSize::Large => {
            header.font_size = t.font_size.base_400;
            header.line_height = t.line_height.base_400;
        }
        AccordionSize::ExtraLarge => {
            header.font_size = t.font_size.base_500;
            header.line_height = t.line_height.base_500;
        }
    }

    let colors = |foreground: Hsla, background: Hsla| AccordionHeaderColors {
        foreground,
        background,
    };

    AccordionSpec {
        header,
        panel: AccordionPanelMetrics {
            margin_start: t.spacing.horizontal.m,
            margin_end: t.spacing.horizontal.m,
            padding: px(0.),
        },
        rest: colors(c.neutral_foreground_1, c.transparent_background),
        hover: colors(c.neutral_foreground_1, c.subtle_background_hover),
        pressed: colors(c.neutral_foreground_1_pressed, c.subtle_background_pressed),
        disabled: colors(c.neutral_foreground_disabled, c.transparent_background),
        focus_ring: c.stroke_focus_2,
    }
}

/// The colours a header paints with.
///
/// Disabled outranks every pointer state, and pressed outranks hover, matching
/// the rest of the library.
pub fn header_colors(
    spec: &AccordionSpec,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> AccordionHeaderColors {
    if disabled {
        return spec.disabled;
    }
    if pressed {
        return spec.pressed;
    }
    if hovered {
        return spec.hover;
    }
    spec.rest
}

/// Whether `value` is among the open items.
pub fn is_open(open: &[usize], value: usize) -> bool {
    open.contains(&value)
}

/// The open set after toggling `value`.
///
/// Mirrors `updateOpenItems` in `useAccordion.js`: only one panel may be open
/// unless `multiple`, and the last one may only close when `collapsible`.
pub fn toggled(open: &[usize], value: usize, multiple: bool, collapsible: bool) -> Vec<usize> {
    if multiple {
        if open.contains(&value) {
            if open.len() > 1 || collapsible {
                return open.iter().copied().filter(|item| *item != value).collect();
            }
            return open.to_vec();
        }
        let mut next = open.to_vec();
        next.push(value);
        next.sort_unstable();
        return next;
    }

    if open.first() == Some(&value) && collapsible {
        return Vec::new();
    }
    vec![value]
}

/// Normalises a caller-supplied default: a non-`multiple` accordion only ever
/// opens the first item it was given.
pub fn normalize_default(open: &[usize], multiple: bool) -> Vec<usize> {
    if multiple {
        let mut next = open.to_vec();
        next.sort_unstable();
        next.dedup();
        return next;
    }
    open.first().copied().into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_theme::{Appearance, FluentTheme};

    fn theme() -> FluentTheme {
        FluentTheme::for_appearance(Appearance::Light)
    }

    #[test]
    fn a_medium_header_is_a_44px_row_with_the_base_300_ramp() {
        let spec = spec(&theme(), AccordionSize::Medium);
        assert_eq!(spec.header.height, px(44.));
        assert_eq!(spec.header.font_size, px(14.));
        assert_eq!(spec.header.line_height, px(20.));
        assert_eq!(spec.header.chevron, px(20.));
        assert_eq!(spec.header.gap, px(8.));
    }

    #[test]
    fn the_small_header_is_shorter_and_uses_the_base_200_ramp() {
        let spec = spec(&theme(), AccordionSize::Small);
        assert_eq!(spec.header.height, px(32.));
        assert_eq!(spec.header.font_size, px(12.));
        assert_eq!(spec.header.line_height, px(16.));
    }

    #[test]
    fn large_sizes_keep_the_row_and_only_move_the_type() {
        let medium = spec(&theme(), AccordionSize::Medium);
        let large = spec(&theme(), AccordionSize::Large);
        let extra = spec(&theme(), AccordionSize::ExtraLarge);
        assert_eq!(large.header.height, medium.header.height);
        assert_eq!(extra.header.height, medium.header.height);
        assert_eq!(large.header.font_size, px(16.));
        assert_eq!(extra.header.font_size, px(20.));
        assert!(medium.header.font_size < large.header.font_size);
        assert!(large.header.font_size < extra.header.font_size);
    }

    #[test]
    fn the_header_padding_matches_the_leading_chevron_layout() {
        let spec = spec(&theme(), AccordionSize::Medium);
        assert_eq!(spec.header.padding_start, px(10.));
        assert_eq!(spec.header.padding_end, px(12.));
    }

    #[test]
    fn the_panel_only_has_the_horizontal_margins() {
        let spec = spec(&theme(), AccordionSize::Medium);
        assert_eq!(spec.panel.margin_start, px(12.));
        assert_eq!(spec.panel.margin_end, px(12.));
        assert_eq!(spec.panel.padding, px(0.));
    }

    #[test]
    fn pressed_outranks_hover_and_disabled_outranks_both() {
        let spec = spec(&theme(), AccordionSize::Medium);
        assert_eq!(header_colors(&spec, true, true, false), spec.pressed);
        assert_eq!(header_colors(&spec, true, true, true), spec.disabled);
        assert_ne!(spec.hover.background, spec.rest.background);
        assert_ne!(spec.disabled.foreground, spec.rest.foreground);
    }

    #[test]
    fn the_focus_rectangle_is_the_two_pixel_focus_stroke() {
        let spec = spec(&theme(), AccordionSize::Medium);
        assert_eq!(spec.header.focus_thickness, px(2.));
        assert_eq!(spec.focus_ring, theme().tokens().color.stroke_focus_2);
        assert_eq!(spec.header.radius, px(4.));
    }

    #[test]
    fn a_single_accordion_moves_the_open_panel() {
        assert_eq!(toggled(&[0], 1, false, false), vec![1]);
        // The open one stays open unless the accordion is collapsible.
        assert_eq!(toggled(&[1], 1, false, false), vec![1]);
        assert_eq!(toggled(&[1], 1, false, true), Vec::<usize>::new());
    }

    #[test]
    fn a_multiple_accordion_adds_and_removes_independently() {
        assert_eq!(toggled(&[0], 2, true, false), vec![0, 2]);
        assert_eq!(toggled(&[0, 2], 2, true, false), vec![0]);
        // The last panel only closes when the accordion is collapsible.
        assert_eq!(toggled(&[2], 2, true, false), vec![2]);
        assert_eq!(toggled(&[2], 2, true, true), Vec::<usize>::new());
    }

    #[test]
    fn a_default_only_keeps_one_panel_unless_multiple() {
        assert_eq!(normalize_default(&[3, 1], false), vec![3]);
        assert_eq!(normalize_default(&[3, 1, 3], true), vec![1, 3]);
    }

    #[test]
    fn a_panel_runs_for_the_normal_motion_duration() {
        // `motionTokens.durationNormal`, which this token set does not carry.
        assert_eq!(PANEL_MOTION, Duration::from_millis(200));
    }

    #[test]
    fn a_panel_starts_and_ends_at_the_resting_frames() {
        let ease = theme().tokens().curve.easy_ease_max;
        // The curve is sampled by bisection, so the ends are exact to within a
        // rounding error rather than bit-exact.
        let near =
            |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3;
        assert!(near(panel_motion(0.0, ease), (0.0, 0.0)));
        assert!(near(panel_motion(1.0, ease), (1.0, 1.0)));
        // Out-of-range progress is clamped rather than extrapolated.
        assert!(near(panel_motion(-1.0, ease), (0.0, 0.0)));
        assert!(near(panel_motion(2.0, ease), (1.0, 1.0)));
    }

    #[test]
    fn a_panel_eases_on_curve_easy_ease_max() {
        let ease = theme().tokens().curve.easy_ease_max;
        // cubic-bezier(0.8, 0, 0.2, 1) is symmetric, so the midpoint is 0.5,
        // while the first quarter lags behind linear (the curve starts slowly).
        let (half_height, half_opacity) = panel_motion(0.5, ease);
        assert!((half_height - 0.5).abs() < 1e-3, "midpoint: {half_height}");
        assert_eq!(half_height, half_opacity);
        assert!(panel_motion(0.25, ease).0 < 0.25);
        // The curve this must not be: the gentler sibling is close to linear at
        // the same point, and would make the panel arrive much earlier.
        let gentle = panel_motion(0.25, theme().tokens().curve.easy_ease).0;
        assert!(
            panel_motion(0.25, ease).0 < gentle,
            "curveEasyEaseMax lags curveEasyEase at the quarter mark: {} vs {gentle}",
            panel_motion(0.25, ease).0
        );
    }

    #[test]
    fn a_panel_never_shrinks_while_it_opens() {
        let ease = theme().tokens().curve.easy_ease_max;
        let mut previous = 0.0;
        for step in 0..=40 {
            let (height, opacity) = panel_motion(step as f32 / 40.0, ease);
            assert!(height >= previous, "progress must not go backwards");
            assert_eq!(height, opacity, "size and opacity stay in lockstep");
            previous = height;
        }
    }
}
