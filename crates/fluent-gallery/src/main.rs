//! Component gallery: the manual review surface for `fluent-ui`.
//!
//! - `FLUENT_GALLERY_SMOKE=1` opens the window and quits, for CI boot checks.
//! - `FLUENT_GALLERY_SCREENSHOT=<path>` (with `--features screenshot`) writes a
//!   PNG after consecutive rendered frames agree (bounded by frames and time).

mod fixtures;
#[cfg(feature = "screenshot")]
mod screenshot;
mod sections;

use std::time::Duration;

use fluent_ui::gpui::{
    AnyElement, App, Bounds, ClickEvent, Context, FocusHandle, Hsla, Render,
    StatefulInteractiveElement, Subscription, Window, WindowBounds, WindowOptions, div, px, size,
};
use fluent_ui::gpui_base::input::{InputState, TextareaState};
use fluent_ui::prelude::*;
use fluent_ui::{
    AlertDialog, Appearance, Avatar, AvatarSize, Badge, BadgeAppearance, BadgeSize, Button,
    ButtonAppearance, ButtonShape, Checkbox, ControlSize, Dialog, Divider, Field, FluentIcon,
    FluentRoot, FluentTheme, FontMode, InputAppearance, Link, Menu, MenuDropdown, MenuHeader,
    MenuItem, Popover, ProgressAppearance, ProgressBar, Radio, Sheet, SheetSide, Spinner,
    SpinnerSize, Switch, TabItem, TabList, TextInput, Textarea, TextareaAppearance, TitleBar,
    TooltipSurface, ValidationMessage,
};

const APPEARANCES: [ButtonAppearance; 5] = [
    ButtonAppearance::Secondary,
    ButtonAppearance::Outline,
    ButtonAppearance::Primary,
    ButtonAppearance::Subtle,
    ButtonAppearance::Transparent,
];

const SIZES: [ControlSize; 3] = [ControlSize::Small, ControlSize::Medium, ControlSize::Large];

fn appearance_name(appearance: ButtonAppearance) -> &'static str {
    match appearance {
        ButtonAppearance::Secondary => "secondary",
        ButtonAppearance::Outline => "outline",
        ButtonAppearance::Primary => "primary",
        ButtonAppearance::Subtle => "subtle",
        ButtonAppearance::Transparent => "transparent",
    }
}

fn size_name(size: ControlSize) -> &'static str {
    match size {
        ControlSize::Small => "small",
        ControlSize::Medium => "medium",
        ControlSize::Large => "large",
    }
}

/// Root view: theme switcher, font mode, and the button matrix.
struct Gallery {
    appearance: Appearance,
    /// Set by `FLUENT_GALLERY_FIXTURE`: render one component on a fixed canvas.
    fixture: Option<String>,
    clicks: usize,
    inputs: Vec<gpui::Entity<InputState>>,
    textareas: Vec<gpui::Entity<TextareaState>>,
    checkbox_a: bool,
    radio_choice: usize,
    switch_a: bool,
    switch_b: bool,
    dialog_open: bool,
    tab_index: usize,
    /// Focus for the horizontal strip. The strip is one Tab stop; arrows move
    /// the selection. Without `track_focus` the strip is unreachable by Tab.
    tab_focus: FocusHandle,
    vertical_tab_index: usize,
    /// Focus for the vertical strip, kept separate so each strip is its own
    /// stop and the two never share selection state.
    vertical_tab_focus: FocusHandle,
    alert_open: bool,
    /// The edge of the open sheet, or `None` when it is closed.
    sheet_side: Option<SheetSide>,
    /// Keeps the OS appearance observer alive for the window's lifetime.
    _appearance_sub: Option<Subscription>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let fixture = std::env::var("FLUENT_GALLERY_FIXTURE").ok();
        // A forced appearance (screenshots, CI baselines) must not be
        // overwritten by the OS observer, which fires immediately.
        let forced = std::env::var_os("FLUENT_GALLERY_APPEARANCE").is_some();
        let sub = if forced {
            None
        } else {
            Some(FluentTheme::follow_system(window))
        };
        // 4 appearances x 3 sizes, plus disabled / invalid / slotted fields.
        let inputs: Vec<_> = (0..18)
            .map(|index| {
                cx.new(|cx| InputState::new(window, cx).placeholder(format!("Placeholder {index}")))
            })
            .collect();
        inputs[0].update(cx, |state, cx| {
            state.set_value("Hello 世界", window, cx);
            state.focus(window, cx);
        });

        let textareas: Vec<_> = (0..11)
            .map(|index| {
                cx.new(|cx| {
                    TextareaState::new(window, cx).placeholder(format!("Multi-line {index}"))
                })
            })
            .collect();

        Self {
            appearance: FluentTheme::global(cx).appearance(),
            clicks: 0,
            inputs,
            textareas,
            checkbox_a: true,
            radio_choice: 1,
            switch_a: true,
            switch_b: false,
            dialog_open: false,
            tab_index: 0,
            tab_focus: cx.focus_handle().tab_stop(true),
            vertical_tab_index: 0,
            vertical_tab_focus: cx.focus_handle().tab_stop(true),
            alert_open: false,
            sheet_side: None,
            fixture,
            _appearance_sub: sub,
        }
    }

    fn apply(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        self.appearance = appearance;
        FluentTheme::global(cx)
            .with_appearance(appearance)
            .install(cx);
        cx.notify();
    }

    fn toggle_font(&mut self, cx: &mut Context<Self>) {
        let current = FluentTheme::global(cx);
        let next = match current.font_mode() {
            FontMode::SegoeCompat => FontMode::System,
            _ => FontMode::SegoeCompat,
        };
        current.with_font_mode(next).install(cx);
        cx.notify();
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = FluentTheme::global(cx);

        // Baseline mode: one fixture, centred on a canvas the window is sized to.
        if let Some(element) = self
            .fixture
            .as_deref()
            .and_then(|name| fixtures::render(name, window, cx))
        {
            return FluentRoot::new()
                .flex()
                .items_center()
                .justify_center()
                .child(element)
                .into_any_element();
        }

        let t = theme.tokens();

        let swatch = |c: Hsla| div().w(px(44.)).h(px(28.)).rounded(px(4.)).bg(c);

        let matrix = self.matrix(cx, &theme);
        let input_matrix = self.input_matrix(cx, &theme);
        let textarea_matrix = self.textarea_matrix(cx, &theme);
        let field_section = self.field_section();
        let header = self.header(cx, &theme);
        let selection_section = self.selection_section(cx);
        let switch_section = self.switch_section(cx);
        let misc_section = self.misc_section();
        // Tabs: one horizontal strip with a disabled tab, one vertical strip. The
        // selection is controlled so the demo exercises the caller-owned path.
        let tabs_section = self.tabs_section(cx);
        let feedback_section = self.feedback_section();
        let dialog_section = self.dialog_section(cx, &theme);
        let sheet_section = self.sheet_section(cx, &theme);
        // `FLUENT_GALLERY_SECTION` narrows the gallery to one section; useful for
        // screenshots and pixel baselines that would otherwise be below the fold.
        let want = |name: &str| match std::env::var("FLUENT_GALLERY_SECTION") {
            Ok(value) => value == name,
            Err(_) => true,
        };
        let mut sections: Vec<AnyElement> = Vec::new();
        if want("buttons") {
            sections.push(matrix.into_any_element());
        }
        if want("inputs") {
            sections.push(input_matrix.into_any_element());
        }
        if want("textareas") {
            sections.push(textarea_matrix.into_any_element());
        }
        if want("fields") {
            sections.push(field_section.into_any_element());
        }
        if want("selection") {
            sections.push(selection_section.into_any_element());
        }
        if want("switch") {
            sections.push(switch_section.into_any_element());
        }
        if want("misc") {
            sections.push(misc_section.into_any_element());
        }
        if want("feedback") {
            sections.push(feedback_section.into_any_element());
        }
        if want("tabs") {
            sections.push(tabs_section.into_any_element());
        }
        if want("dialog") {
            sections.push(dialog_section.into_any_element());
        }
        if want("sheet") {
            sections.push(sheet_section.into_any_element());
        }
        FluentRoot::new()
            .flex()
            .flex_col()
            // Placeholder artwork until the gallery has a real app logo; it also
            // exercises the leading slot that TitleBar reserves for one.
            .child(TitleBar::new("gallery-titlebar", "Fluent Gallery").icon(FluentIcon::Settings))
            .child(header)
            .child(
                div()
                    .id("matrix-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(sections)
                    .child(
                        div()
                            .p(px(16.))
                            .flex()
                            .gap(px(8.))
                            .child(swatch(t.color.brand_background))
                            .child(swatch(t.color.neutral_background_1_hover))
                            .child(swatch(t.color.neutral_stroke_accessible))
                            .child(swatch(t.color.stroke_focus_2)),
                    ),
            )
            .into_any_element()
    }
}

fn main() {
    if std::env::var_os("FLUENT_GALLERY_LIST_FIXTURES").is_some() {
        for fixture in fixtures::FIXTURES {
            println!("{}\t{}\t{}", fixture.name, fixture.width, fixture.height);
        }
        return;
    }
    fluent_ui::application().run(|cx: &mut App| {
        fluent_ui::init(cx);

        if let Some(value) = std::env::var_os("FLUENT_GALLERY_APPEARANCE") {
            let appearance = match value.to_string_lossy().to_ascii_lowercase().as_str() {
                "dark" => Appearance::Dark,
                "hc" | "high-contrast" => Appearance::HighContrast,
                _ => Appearance::Light,
            };
            FluentTheme::for_appearance(appearance).install(cx);
        }

        // A fixture pins the window to its canvas so the PNG is reproducible.
        let fixture_size = std::env::var("FLUENT_GALLERY_FIXTURE")
            .ok()
            .and_then(|name| fixtures::size(&name));
        let (width, height) = fixture_size.unwrap_or_else(|| {
            std::env::var("FLUENT_GALLERY_SIZE")
                .ok()
                .and_then(|value| {
                    let (w, h) = value.split_once('x')?;
                    Some((w.trim().parse::<f32>().ok()?, h.trim().parse::<f32>().ok()?))
                })
                .unwrap_or((900.0, 640.0))
        });
        let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..TitleBar::window_options("Fluent Gallery")
                },
                |window, cx| cx.new(|cx| Gallery::new(window, cx)),
            )
            .expect("failed to open window");

        #[cfg(feature = "screenshot")]
        if let Some(path) = std::env::var_os("FLUENT_GALLERY_SCREENSHOT") {
            let path = std::path::PathBuf::from(path);
            window
                .update(cx, |_, window, _| screenshot::capture(path, window))
                .expect("schedule screenshot");
        }
        #[cfg(not(feature = "screenshot"))]
        let _ = &window;

        if std::env::var_os("FLUENT_GALLERY_SMOKE").is_some() {
            let executor = cx.background_executor().clone();
            cx.spawn(async move |cx| {
                executor.timer(Duration::from_millis(1500)).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }

        cx.activate(true);
    });
}

#[cfg(test)]
mod tab_stop_tests {
    //! Tab-order regression for the two tab strips in the *default* gallery
    //! view (not a fixture). On the pre-fix code neither strip was wired to a
    //! focus handle, so the strip was skipped by Tab and its arrows only worked
    //! after programmatic focus.
    use super::*;
    use fluent_ui::gpui::{
        Entity, KeyDownEvent, Keystroke, PlatformInput, TestAppContext, WindowHandle,
    };

    type Handle = WindowHandle<Gallery>;

    fn settle(cx: &mut TestAppContext, window: Handle) {
        for _ in 0..2 {
            cx.update_window(window.into(), |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .unwrap();
            cx.run_until_parked();
        }
    }

    fn open(cx: &mut TestAppContext) -> (Handle, Entity<Gallery>) {
        cx.update(fluent_ui::init);
        let window = cx.open_window(size(px(1400.), px(900.)), |window, cx| {
            Gallery::new(window, cx)
        });
        cx.run_until_parked();
        let app = cx
            .update_window(window.into(), |root, _, _| {
                root.downcast::<Gallery>()
                    .expect("the gallery view is the root")
            })
            .unwrap();
        settle(cx, window);
        (window, app)
    }

    fn focus_strip(cx: &mut TestAppContext, window: Handle, focus: &FocusHandle) {
        cx.update_window(window.into(), |_, window, cx| focus.focus(window, cx))
            .unwrap();
        settle(cx, window);
    }

    fn focused(cx: &mut TestAppContext, window: Handle) -> Option<FocusHandle> {
        cx.update_window(window.into(), |_, window, cx| window.focused(cx))
            .unwrap()
    }

    fn press(cx: &mut TestAppContext, window: Handle, key: &'static str) {
        let keystroke = Keystroke::parse(key).unwrap();
        cx.update_window(window.into(), move |_, window, cx| {
            window.dispatch_event(
                PlatformInput::KeyDown(KeyDownEvent {
                    keystroke,
                    is_held: false,
                    prefer_character_input: false,
                }),
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
        settle(cx, window);
    }

    #[gpui::test]
    fn default_tab_strips_are_stops_and_own_arrow_and_home_end(cx: &mut TestAppContext) {
        let (window, app) = open(cx);
        let (horizontal, vertical) = cx.update(|cx| {
            let gallery = app.read(cx);
            (
                gallery.tab_focus.clone(),
                gallery.vertical_tab_focus.clone(),
            )
        });

        focus_strip(cx, window, &horizontal);
        assert_eq!(focused(cx, window), Some(horizontal.clone()));

        // Enter the strip from the real stop before it instead of relying only
        // on programmatic focus: Shift+Tab out, then Tab back in. This proves
        // the horizontal strip is registered in the page's tab order, not just
        // that a handle exists. On the pre-fix code the first Shift+Tab lands on
        // some other stop and the following Tab never reaches this strip.
        press(cx, window, "shift-tab");
        let before = focused(cx, window).expect("a real tab stop precedes the strips");
        assert!(
            before != horizontal && before != vertical,
            "Shift+Tab from the horizontal strip must land outside the tab strips"
        );
        press(cx, window, "tab");
        assert_eq!(
            focused(cx, window),
            Some(horizontal.clone()),
            "Tab from the preceding real stop must enter the horizontal strip"
        );

        // The consumer path: the strip's own handlers update the caller-owned
        // selection, and disabled tabs are skipped.
        press(cx, window, "right");
        assert_eq!(cx.update(|cx| app.read(cx).tab_index), 1);
        press(cx, window, "right");
        assert_eq!(cx.update(|cx| app.read(cx).tab_index), 3);
        press(cx, window, "home");
        assert_eq!(cx.update(|cx| app.read(cx).tab_index), 0);
        press(cx, window, "end");
        assert_eq!(cx.update(|cx| app.read(cx).tab_index), 3);

        // Both default strips are real Tab stops; the vertical one follows the
        // horizontal one in the page's paint order.
        press(cx, window, "tab");
        assert_eq!(
            focused(cx, window),
            Some(vertical.clone()),
            "Tab must leave the horizontal strip and stop on the vertical strip"
        );
        press(cx, window, "down");
        assert_eq!(cx.update(|cx| app.read(cx).vertical_tab_index), 1);

        press(cx, window, "tab");
        let after = focused(cx, window).expect("Tab leaves the strips for the next stop");
        assert!(
            after != horizontal && after != vertical,
            "Tab must continue past the strips instead of wrapping inside them"
        );
        press(cx, window, "shift-tab");
        assert_eq!(
            focused(cx, window),
            Some(vertical),
            "Shift+Tab returns to the vertical strip"
        );
        press(cx, window, "shift-tab");
        assert_eq!(
            focused(cx, window),
            Some(horizontal),
            "Shift+Tab from the vertical strip returns to the horizontal strip"
        );
    }
}
