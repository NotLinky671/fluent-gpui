# fluent-ui

A Rust UI component library built to the **Microsoft Fluent 2** visual specification and rendered with **GPUI**. The goal is consistent component appearance, interaction behaviour and developer experience across Windows, macOS and Linux.

**English** | [简体中文](README.zh-CN.md)

> Status: **M1 component foundation is in place; M2 motion and overlay work is in progress**. The API is not stable; current capabilities and unfinished work are listed below.

## Components

This table describes implementation scope, not verification of every platform, interaction or visual state. See the [engineering checklist](docs/code-standards.md) for the separate verification layers.

| Component | Implementation scope | Notes |
|---|---|---|
| `Button` | Foundation | Primary / Secondary / Outline / Subtle / Transparent; size, shape and icon configuration; Toggle / Compound / Menu / Split variants are not implemented |
| `TextInput` | Foundation | Single line, with leading/trailing content, invalid and disabled states; pressing elsewhere clears focus, and Tab moves between fields; IME has not been tried |
| `Textarea` | Foundation | Multi-line, three appearances; pressing elsewhere clears focus |
| `Field` / `Label` | Foundation | Vertical and horizontal layout, hint text, Error/Warning/Success validation messages |
| `Checkbox` / `Radio` / `Switch` | Foundation | Controlled/uncontrolled modes; mouse hover/pressed wiring and colour transitions are implemented; full interaction verification remains pending |
| `Link` / `Divider` | Foundation | Three link palettes, four divider appearances |
| `Badge` / `Avatar` | Foundation | Seven badge appearances, four avatar sizes; Avatar renders initials only |
| `ProgressBar` | Partial | Determinate progress and an indeterminate sweep; both honour reduced motion; pixel baselines for the indeterminate mode are pending |
| `Tooltip` | Foundation | `Button::tooltip` uses a root-hosted anchored overlay with a configured 4px gap; hover and flip placement still need manual verification |
| `Popover` | Experimental | Anchored panel, controlled/uncontrolled open state, same-frame scroll positioning, outside-click dismissal, Escape, focus handover and a Tab focus trap when `track_focus` is supplied; the trap cycles between panel items and never reaches the page behind |
| `Menu` / `MenuItem` | Partial | Panel, leading icons, check and radio rows, separators, headers, `MenuDropdown` trigger integration through `Popover`, arrow/Home/End navigation with disabled rows skipped, typeahead, and closing on Tab; submenus remain unfinished |
| `Spinner` | Foundation | Four sizes (16/20/28/32), accessible label via `Spinner::label`, honours reduced motion; stroke widths are approximations pending verification against official screenshots |

## Quick start

```toml
[dependencies]
fluent-ui = "0.1"
```

```rust
use fluent_ui::{Button, FluentRoot, FluentTheme};
use fluent_ui::gpui::{App, AppContext, Context, IntoElement, Render, Window};
use fluent_ui::prelude::*;

struct HelloView;

impl Render for HelloView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(Button::new("save").child("Save"))
    }
}

fn main() {
    fluent_ui::application().run(|cx: &mut App| {
        fluent_ui::init(cx);
        FluentTheme::web_light().install(cx);
        cx.open_window(Default::default(), |_, cx| cx.new(|_| HelloView))
            .expect("failed to open window");
    });
}
```

`FluentRoot` applies the theme's background, foreground and font, and hosts the tooltip overlay, so the outermost element of an application should be wrapped in it.

## Integration app

[`fluent-notes`](crates/fluent-notes) is a small native notes application that combines the current components in real flows: drafts, themes, menus, popovers, focus and confirmation bars. It complements the gallery's single-component matrix.

```text
cargo run -p fluent-notes
```

Notes live in memory only and are lost when the window closes; there is no networking, account or persistence. Delete and unsaved-switch confirmation use in-page confirm/cancel bars until Dialog exists. Usage, the repeatable manual checklist, known limitations and M2 hookup points are in [docs/apps/notes.md](docs/apps/notes.md); this round's actual verification is in [docs/validation/notes-app-2026-11-03.md](docs/validation/notes-app-2026-11-03.md).

## Controlled and uncontrolled

The selection controls (`Checkbox`, `Radio`, `Switch`) support both modes, mirroring `checked` / `defaultChecked` in Fluent React:

```rust
// Controlled: the caller owns the state.
Switch::new("wifi")
    .checked(self.wifi_on)
    .on_change(cx.listener(|this, value, _, cx| {
        this.wifi_on = *value;
        cx.notify();
    }))

// Uncontrolled: the component keeps its own state.
Switch::new("wifi").default_checked(true)
```

## Theming

The token data includes five official palettes. `FluentTheme` currently exposes these three built-in theme constructors, not `teams_light()` / `teams_dark()`:

```rust
FluentTheme::web_light().install(cx);
FluentTheme::web_dark().install(cx);
FluentTheme::high_contrast().install(cx);
```

To follow the operating system appearance instead: `FluentTheme::follow_system(window)`.

## Architecture

```
fluent-tokens     Fluent design tokens (5 themes × 459), generated from the official data
fluent-theme      FluentTheme: tokens + brand ramp + fonts, projected onto gpui_base::Theme
fluent-icons      Fluent System Icons as static SVG data (no GPUI dependency)
fluent-core       GPUI snapshot compatibility layer + pure-function style resolution (spec)
fluent-components Component implementations
fluent-ui         Public facade
```

**Style decisions are pure functions.** `spec::<component>::resolve(&FluentTheme, &Props) -> Spec` needs no window, so token mappings, dimensions, colours and state precedence are unit-testable. This does not prove event wiring or final rendering: GPUI behaviour tests and native-window/pixel checks are separate verification layers.

**Rendered output outranks CSS constants.** When the CSS values in Fluent UI React v9 disagree with what Fluent 2 actually renders, the rendering wins (ADR-0002). The switch thumb and the radio dot were both corrected against measured pixels.

GPUI is pinned to an exact snapshot (`gpui-pre = 0.3.10`); a caret requirement would drift onto an incompatible API.

## Known limitations

- `Avatar` renders initials only; no images, no status badge
- Mouse hover/pressed state wiring and colour transitions are implemented for Checkbox / Radio / Switch; full keyboard and pointer-cancellation behaviour is not yet verified
- `Popover` dismissal is covered by behaviour tests: an outside press reports `false`, presses on the panel body or its interactive children do not, a press on the trigger reports one change, and a controlled caller that ignores the request stays open. With `track_focus`, opening moves focus into the panel and closing returns it when focus is still inside; Escape only fires while focus is inside. Focus lands on the panel container, not the first control, and viewport edges clamp the panel rather than flipping it above/below. The container does not have to be a Tab stop; if the caller marks it as one it joins the item cycle
- `Badge` foregrounds are picked by contrast: each appearance uses whichever of the theme's static light/dark label tokens contrasts more with the badge background, so every appearance stays legible in light, dark and high contrast (previously Warning was white-on-yellow in dark and white-on-white in high contrast). The mapping is token-derived and verified by spec tests; native-window visual checks for the affected appearances are still pending. See the [fix record](docs/validation/badge-contrast-2026-11-03.md)
- `Textarea` does not handle the mouse wheel in this GPUI snapshot; long bodies scroll with the caret/keyboard, and the scrollbar appears once the caret moves past the viewport
- `MenuDropdown` composes `Menu` into `Popover`, so a menu inherits anchored placement and dismissal. Arrow keys, Home/End, Enter/Space and disabled-row skipping work, and mouse movement writes the same current row the keyboard does; layout changes under a stationary pointer do not change selection, and at most one row looks current. Typeahead matches a character prefix starting after the current row (600ms reset, repeated characters cycle), and Tab or Shift+Tab inside the menu closes it via `Dismiss`. Activating an item does not close the menu; the caller's action must close it explicitly (the notes app does this in every item). Submenus are unfinished, and this GPUI version exposes no accesskit disabled state
- Pixel baselines for indeterminate `ProgressBar` and `Spinner` are pending; `Spinner` stroke widths and arc span are approximations not yet checked against official Fluent screenshots
- `Button::tooltip` delegates flipping to the gpui-base overlay; hover/flip placement still needs manual verification. The generic `tooltip(text)` helper uses GPUI-native positioning without the same anchoring guarantee. Tests cover only that the root publishes the service, that its entity identity survives redraws, and that callers see `None` without a root
- No RTL support, no letter-spacing control, no CSS `outline`
- No live regions; nothing is announced to screen readers on change
- High contrast has to be selected manually; there is no system-level auto-detection
- IME composition has not been tested

## Development

Before editing, read the [agent entry point](AGENTS.md) and [engineering standards, component checklist and verification matrix](docs/code-standards.md). Use the [PR verification record](.github/pull_request_template.md) for delivery. Documentation-only changes and code changes have different verification requirements.

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
cargo deny check
cargo xtask gen-tokens --check
cargo xtask gen-icons --check
```

`cargo deny check` may need network access to refresh advisories; disclose data-freshness limits for offline audits. If the local task index exists, additionally run `cargo xtask tasks --check`; it depends on local-only notes that are not distributed with the repository and is not a current CI gate.

```text
cargo run -p fluent-gallery
cargo run -p fluent-notes
cargo test -p fluent-gallery --features screenshot --test baselines --locked
```

Pixel comparisons need the `screenshot` feature and a usable GPU surface; they currently run locally, not in CI. The 15 committed baselines cover Windows / light only, not dark/high-contrast or hover/focus/pressed/disabled. Directories currently distinguish OS only; DPI, fonts and rendering environment must also match. Set `FLUENT_BASELINE_UPDATE=1` only to accept an intentional visual change, review the image diff, then rerun with update mode unset. Do not re-record baselines instead of investigating a regression.

## License

Apache-2.0