//! Asks the compositor for a window material, and says what actually happened.
//!
//! [`fluent_core::spec::backdrop`] decides what to draw from an outcome; this
//! module is the only place that talks to the platform. GPUI's own setter returns
//! nothing — on Windows it calls DWM and swallows the result — so a window whose
//! request was refused looks exactly like one that never asked. Reporting the
//! outcome is what lets the root fall back to a solid background instead of
//! painting a translucent layer over whatever the desktop happens to show.

pub use fluent_core::spec::backdrop::{
    BackdropOutcome, BackdropPolicy, MATERIAL_LAYER_ALPHA, WindowBackdrop, resolve,
    window_appearance,
};

use fluent_core::gpui_compat::{App, ElementId, SharedString};
use fluent_theme::{Appearance, FluentTheme};

/// Asks the compositor for `backdrop` on `window`.
///
/// The renderer's clear colour is part of the request: a window that keeps
/// clearing opaque hides the material behind its own frame, so GPUI is told the
/// appearance first and the platform call then confirms it.
pub fn apply(window: &gpui::Window, backdrop: WindowBackdrop) -> BackdropOutcome {
    window.set_background_appearance(fluent_core::spec::backdrop::window_appearance(backdrop));
    platform::apply(window, backdrop)
}

/// Tells the compositor which app mode the material should follow.
///
/// Mica is tinted by the desktop from the app mode DWM holds, not from the
/// colours we paint: a dark theme over a light material leaves a washed-out grey
/// that is neither theme. Windows only reads this once the window is composed, so
/// it is written whenever the material is requested.
pub fn set_dark_mode(window: &gpui::Window, dark: bool) {
    platform::set_dark_mode(window, dark);
}

/// The material the compositor reports for `window`, when the platform can say.
///
/// This reads back what the compositor holds, which is evidence about the
/// request only — whether the desktop actually shows the blur depends on the
/// wallpaper, the system settings and whether the window is active.
pub fn reported(window: &gpui::Window) -> Option<i32> {
    platform::reported(window)
}

const WINDOW_BACKDROP_KEY: &str = "fluent-window-backdrop";

/// What this window was last asked for, so a repaint does not re-issue the call.
///
/// The app mode is part of the request: Mica follows the mode DWM holds, so
/// switching between light and dark has to re-apply the material.
#[derive(Default)]
struct BackdropState {
    asked: Option<(WindowBackdrop, bool)>,
    outcome: Option<BackdropOutcome>,
}

/// The layer a root should paint for `request`, asking the compositor once per
/// change of request.
///
/// One window owns one backdrop: the request lives in keyed window state, so two
/// roots in the same window cannot fight over the material, and a rebuilt window
/// starts from its own state. High contrast comes from the theme and revokes the
/// material instead of leaving a translucent layer over the desktop.
pub fn policy_for(
    window: &mut gpui::Window,
    cx: &mut App,
    request: WindowBackdrop,
) -> BackdropPolicy {
    let theme = FluentTheme::global(cx);
    let high_contrast = theme.appearance() == Appearance::HighContrast;
    let target = if high_contrast {
        WindowBackdrop::Solid
    } else {
        request
    };

    // The test platform reports `Opaque` for every request, so tests inject the
    // answer a real compositor would have given.
    #[cfg(test)]
    if let Some((_, outcome)) = cx
        .try_global::<SeededOutcome>()
        .and_then(|seed| seed.0)
        .filter(|(seeded, _)| *seeded == request)
    {
        return resolve(request, outcome, high_contrast, &theme);
    }

    let state = window.use_keyed_state(
        ElementId::from(SharedString::from(WINDOW_BACKDROP_KEY)),
        cx,
        |_, _| BackdropState::default(),
    );
    let cached = {
        let current = state.read(cx);
        current.asked.zip(current.outcome)
    };
    let dark = theme.appearance() == Appearance::Dark;
    let outcome = match cached {
        Some(((asked, asked_dark), outcome)) if asked == target && asked_dark == dark => outcome,
        _ => {
            let outcome = apply(window, target);
            set_dark_mode(window, dark);
            state.update(cx, |state, _| {
                state.asked = Some((target, dark));
                state.outcome = Some(outcome);
            });
            outcome
        }
    };

    resolve(request, outcome, high_contrast, &theme)
}

/// Makes the root behave as if the compositor had answered `outcome` for
/// `request`.
///
/// The test platform has no compositor to ask, so without a seeded answer the
/// confirmed branch could never be exercised: every request would come back a
/// refusal. Keyed window state cannot be touched outside a layout pass, which is
/// why the seed lives in an app global instead.
#[cfg(test)]
pub(crate) fn prime(cx: &mut App, request: WindowBackdrop, outcome: BackdropOutcome) {
    cx.set_global(SeededOutcome(Some((request, outcome))));
}

#[cfg(test)]
#[derive(Clone, Copy, Default)]
struct SeededOutcome(Option<(WindowBackdrop, BackdropOutcome)>);

#[cfg(test)]
impl gpui::Global for SeededOutcome {}

#[cfg(target_os = "windows")]
mod platform {
    use super::{BackdropOutcome, WindowBackdrop};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Dwm::{
        DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMSBT_TABBEDWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_USE_IMMERSIVE_DARK_MODE, DwmGetWindowAttribute, DwmSetWindowAttribute,
    };

    fn hwnd(window: &gpui::Window) -> Option<HWND> {
        let handle = HasWindowHandle::window_handle(window).ok()?;
        match handle.as_raw() {
            RawWindowHandle::Win32(win32) => Some(win32.hwnd.get() as HWND),
            _ => None,
        }
    }

    fn material(backdrop: WindowBackdrop) -> i32 {
        match backdrop {
            // Clearing matters: switching back to a solid window has to remove
            // the backdrop attribute, not merely stop asking for it.
            WindowBackdrop::Solid => DWMSBT_NONE,
            WindowBackdrop::Mica => DWMSBT_MAINWINDOW,
            WindowBackdrop::MicaAlt => DWMSBT_TABBEDWINDOW,
        }
    }

    pub(super) fn set_dark_mode(window: &gpui::Window, dark: bool) {
        let Some(hwnd) = hwnd(window) else { return };
        let value: i32 = i32::from(dark);
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                &value as *const i32 as *const c_void,
                size_of::<i32>() as u32,
            );
        }
    }

    pub(super) fn apply(window: &gpui::Window, backdrop: WindowBackdrop) -> BackdropOutcome {
        let Some(hwnd) = hwnd(window) else {
            return BackdropOutcome::Unsupported("the window has no Win32 handle");
        };
        let asked = material(backdrop);
        let result = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE as u32,
                &asked as *const i32 as *const c_void,
                size_of::<i32>() as u32,
            )
        };
        if result < 0 {
            return BackdropOutcome::Failed(result);
        }

        match reported(window) {
            Some(reported) if reported == asked => BackdropOutcome::Applied,
            Some(reported) => BackdropOutcome::Mismatch { asked, reported },
            None => BackdropOutcome::Unsupported("the compositor would not report the backdrop"),
        }
    }

    pub(super) fn reported(window: &gpui::Window) -> Option<i32> {
        let hwnd = hwnd(window)?;
        let mut current: i32 = 0;
        let result = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE as u32,
                &mut current as *mut i32 as *mut c_void,
                size_of::<i32>() as u32,
            )
        };
        (result >= 0).then_some(current)
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::{BackdropOutcome, WindowBackdrop};

    pub(super) fn set_dark_mode(_: &gpui::Window, _: bool) {}

    pub(super) fn apply(_: &gpui::Window, _: WindowBackdrop) -> BackdropOutcome {
        BackdropOutcome::Unsupported("Mica is a Windows 11 material")
    }

    pub(super) fn reported(_: &gpui::Window) -> Option<i32> {
        None
    }
}
