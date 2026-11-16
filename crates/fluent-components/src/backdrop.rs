//! Asks the compositor for a window material, and makes the window follow the
//! answer.
//!
//! [`fluent_core::spec::backdrop`] decides what to draw from an outcome; this
//! module is the only place that talks to the platform. GPUI's own setter returns
//! nothing — on Windows it changes how the renderer clears the window but leaves
//! the material alone — so a window whose request was refused looks exactly like
//! one that never asked, except that it now composes against the desktop. That is
//! why the request goes out first and the appearance follows the answer, and why
//! a refusal clears the material explicitly instead of trusting the `Opaque`
//! branch to do it.

pub use fluent_core::spec::backdrop::{
    BackdropOutcome, BackdropPolicy, MATERIAL_LAYER_ALPHA, WindowBackdrop, resolve,
    window_appearance,
};

use fluent_core::gpui_compat::{App, Global};
use fluent_theme::{Appearance, FluentTheme};
use gpui::{AnyWindowHandle, WindowBackgroundAppearance};
use std::collections::HashMap;

/// Asks the compositor for `backdrop` and makes the window appearance follow the
/// answer.
///
/// `dark` is part of the request, not a decoration: DWM tints the material from
/// the app mode the window holds, so a dark theme over a light material is a
/// washed-out grey that belongs to neither. It is written before the material so
/// the compositor never has to guess.
///
/// The returned outcome is the platform's answer even when the fallback itself
/// succeeds. The reason a material is missing must not be replaced by the fact
/// that the rollback worked: a window that reports `Failed` and then recovers is
/// still a window whose compositor said no, and that is what the caller and the
/// policy need to see.
pub fn apply(window: &gpui::Window, backdrop: WindowBackdrop, dark: bool) -> BackdropOutcome {
    let outcome = request(window, backdrop, dark);
    let confirmed = outcome == BackdropOutcome::Applied;
    set_appearance(
        window,
        if confirmed {
            window_appearance(backdrop)
        } else {
            WindowBackgroundAppearance::Opaque
        },
    );
    if !confirmed {
        // The `Opaque` branch of GPUI's setter does not clear
        // `DWMWA_SYSTEMBACKDROP_TYPE`, so a refused request would leave the
        // material on the window while the page paints an opaque surface.
        clear(window);
    }
    outcome
}

/// The material the compositor reports for `window`, when the platform can say.
///
/// This reads back what the compositor holds, which is evidence about the
/// request only — whether the desktop actually shows the blur depends on the
/// wallpaper, the system settings and whether the window is active.
pub fn reported(window: &gpui::Window) -> Option<i32> {
    platform::reported(window)
}

/// What each window currently holds, so a repaint does not re-issue the call.
///
/// The app mode is part of the request: DWM tints the material from the mode the
/// window holds, so switching between light and dark has to re-apply it. The
/// outcome is the platform's answer, which a repaint keeps returning — a fallback
/// that worked does not turn a refusal into a success.
///
/// This is keyed by window rather than stored in keyed element state because it
/// has to be readable *outside* the element that asked: a diagnostic, or a test,
/// asks the window what happened, and keyed state would only answer from the same
/// place in the tree.
#[derive(Default)]
struct BackdropRegistry {
    windows: HashMap<AnyWindowHandle, Held>,
}

#[derive(Clone, Copy)]
struct Held {
    asked: (WindowBackdrop, bool),
    outcome: BackdropOutcome,
}

impl Global for BackdropRegistry {}

/// The layer a root should paint for `request`, asking the compositor once per
/// change of request.
///
/// One window owns one backdrop: the request is recorded per window, so two roots
/// in the same window cannot fight over the material, and a rebuilt window starts
/// from its own state. High contrast comes from the theme and revokes the
/// material instead of leaving a translucent layer over the desktop.
pub fn policy_for(window: &gpui::Window, cx: &mut App, request: WindowBackdrop) -> BackdropPolicy {
    let theme = FluentTheme::global(cx);
    let high_contrast = theme.appearance() == Appearance::HighContrast;
    let target = if high_contrast {
        WindowBackdrop::Solid
    } else {
        request
    };
    let dark = theme.appearance() == Appearance::Dark;
    let handle = window.window_handle();
    let cached = cx
        .default_global::<BackdropRegistry>()
        .windows
        .get(&handle)
        .copied();

    let outcome = match cached {
        Some(held) if held.asked == (target, dark) => held.outcome,
        _ => {
            let outcome = apply(window, target, dark);
            cx.default_global::<BackdropRegistry>().windows.insert(
                handle,
                Held {
                    asked: (target, dark),
                    outcome,
                },
            );
            outcome
        }
    };

    resolve(request, outcome, high_contrast, &theme)
}

/// What this window was last asked for and what the compositor answered.
///
/// Diagnostics only: the policy already carries the decision, but when a material
/// does not appear the first question is whether the compositor refused it, and
/// the root cannot answer that from a colour.
pub fn last_request(window: &gpui::Window, cx: &App) -> Option<(WindowBackdrop, BackdropOutcome)> {
    let handle = window.window_handle();
    cx.try_global::<BackdropRegistry>()
        .and_then(|registry| registry.windows.get(&handle).copied())
        .map(|held| (held.asked.0, held.outcome))
}

fn set_appearance(window: &gpui::Window, appearance: WindowBackgroundAppearance) {
    #[cfg(test)]
    record_appearance(appearance);
    window.set_background_appearance(appearance);
}

fn request(window: &gpui::Window, backdrop: WindowBackdrop, dark: bool) -> BackdropOutcome {
    #[cfg(test)]
    if let Some(outcome) = answer(backdrop, dark) {
        return outcome;
    }
    platform::request(window, backdrop, dark)
}

fn clear(window: &gpui::Window) {
    #[cfg(test)]
    if record_clear() {
        return;
    }
    platform::clear(window);
}

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

    fn write(hwnd: HWND, attribute: i32, value: i32) -> i32 {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute as u32,
                &value as *const i32 as *const c_void,
                size_of::<i32>() as u32,
            )
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

    pub(super) fn request(
        window: &gpui::Window,
        backdrop: WindowBackdrop,
        dark: bool,
    ) -> BackdropOutcome {
        let Some(handle) = hwnd(window) else {
            return BackdropOutcome::Unsupported("the window has no Win32 handle");
        };
        // Written first: the material is tinted from the app mode, and a material
        // asked for under the wrong mode comes back in the wrong colour.
        write(handle, DWMWA_USE_IMMERSIVE_DARK_MODE, i32::from(dark));

        let asked = material(backdrop);
        let result = write(handle, DWMWA_SYSTEMBACKDROP_TYPE, asked);
        if result < 0 {
            return BackdropOutcome::Failed(result);
        }

        match reported(window) {
            Some(reported) if reported == asked => BackdropOutcome::Applied,
            Some(reported) => BackdropOutcome::Mismatch { asked, reported },
            None => BackdropOutcome::Unsupported("the compositor would not report the backdrop"),
        }
    }

    pub(super) fn clear(window: &gpui::Window) {
        if let Some(handle) = hwnd(window) {
            write(handle, DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_NONE);
        }
    }

    pub(super) fn reported(window: &gpui::Window) -> Option<i32> {
        let handle = hwnd(window)?;
        let mut current: i32 = 0;
        let result = unsafe {
            DwmGetWindowAttribute(
                handle,
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

    pub(super) fn request(_: &gpui::Window, _: WindowBackdrop, _: bool) -> BackdropOutcome {
        // Nothing is touched before the answer, so an unsupported platform keeps
        // the window it was given: the caller falls back to the opaque surface
        // without ever having made the window transparent.
        BackdropOutcome::Unsupported("Mica is a Windows 11 material")
    }

    pub(super) fn clear(_: &gpui::Window) {}

    pub(super) fn reported(_: &gpui::Window) -> Option<i32> {
        None
    }
}

#[cfg(test)]
use fake::{answer, appearance as record_appearance, clear as record_clear};
#[cfg(test)]
pub(crate) use fake::{Calls as RecordedCalls, calls as fake_calls, install as install_fake};
#[cfg(test)]
mod fake {
    //! A recorded platform, so tests can see the calls a real window receives.
    //!
    //! The test platform has no compositor: its appearance getter always reports
    //! `Opaque` and its setter does nothing. Asserting on a policy alone would
    //! therefore miss exactly the bug this seam exists for — a window left
    //! transparent after a refusal — so the fake records the requests, the
    //! appearance each one produced and the explicit clears, while the production
    //! code path runs unchanged.

    use super::{BackdropOutcome, WindowBackdrop};
    use gpui::WindowBackgroundAppearance;
    use std::cell::RefCell;

    #[derive(Clone, Debug, Default)]
    pub(crate) struct Calls {
        pub(crate) appearances: Vec<WindowBackgroundAppearance>,
        pub(crate) requests: Vec<(WindowBackdrop, bool)>,
        pub(crate) clears: usize,
    }

    #[derive(Default)]
    struct Fake {
        answers: Vec<(WindowBackdrop, BackdropOutcome)>,
        calls: Calls,
    }

    impl Fake {
        fn answer(&mut self, backdrop: WindowBackdrop, dark: bool) -> Option<BackdropOutcome> {
            self.calls.requests.push((backdrop, dark));
            let answer = self
                .answers
                .iter()
                .find(|(asked, _)| *asked == backdrop)
                .map(|(_, outcome)| *outcome);
            // A request nobody configured is treated as confirmed, which is what a
            // healthy Windows 11 compositor does for every value it supports.
            Some(answer.unwrap_or(BackdropOutcome::Applied))
        }
    }

    thread_local! {
        static FAKE: RefCell<Option<Fake>> = const { RefCell::new(None) };
    }

    /// Installs a recorded platform that answers `answers` and confirms the rest.
    pub(crate) fn install(answers: Vec<(WindowBackdrop, BackdropOutcome)>) {
        FAKE.with(|fake| {
            *fake.borrow_mut() = Some(Fake {
                answers,
                calls: Calls::default(),
            });
        });
    }

    pub(crate) fn calls() -> Option<Calls> {
        FAKE.with(|fake| fake.borrow().as_ref().map(|fake| fake.calls.clone()))
    }

    pub(super) fn answer(backdrop: WindowBackdrop, dark: bool) -> Option<BackdropOutcome> {
        FAKE.with(|fake| fake.borrow_mut().as_mut()?.answer(backdrop, dark))
    }

    pub(super) fn appearance(appearance: WindowBackgroundAppearance) {
        FAKE.with(|fake| {
            if let Some(fake) = fake.borrow_mut().as_mut() {
                fake.calls.appearances.push(appearance);
            }
        });
    }

    /// Returns true when a fake is installed, so the caller skips the platform.
    pub(super) fn clear() -> bool {
        FAKE.with(|fake| match fake.borrow_mut().as_mut() {
            Some(fake) => {
                fake.calls.clears += 1;
                true
            }
            None => false,
        })
    }
}
