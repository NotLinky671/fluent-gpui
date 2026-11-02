# fluent-ui

A Rust UI component library built to the **Microsoft Fluent 2** visual specification and rendered with **GPUI**. The goal is consistent component appearance, interaction behaviour and developer experience across Windows, macOS and Linux.

**English** | [简体中文](README.zh-CN.md)

> Status: **M1 component foundation is in place; M2 motion and overlay work is in progress**. The API is not stable; current capabilities and unfinished work are listed below.

## Components

This table describes implementation scope, not verification of every platform, interaction or visual state. See the [engineering checklist](docs/code-standards.md) for the separate verification layers.

| Component | Implementation scope | Notes |
|---|---|---|
| `Button` | Foundation | Primary / Secondary / Outline / Subtle / Transparent; size, shape and icon configuration; Toggle / Compound / Menu / Split variants are not implemented |
| `TextInput` | Foundation | Single line, with leading/trailing content, invalid and disabled states; IME has not been tried |
| `Textarea` | Foundation | Multi-line, three appearances |
| `Field` / `Label` | Foundation | Vertical and horizontal layout, hint text, Error/Warning/Success validation messages |
| `Checkbox` / `Radio` / `Switch` | Foundation | Controlled/uncontrolled modes; mouse hover/pressed wiring and colour transitions are implemented; full interaction verification remains pending |
| `Link` / `Divider` | Foundation | Three link palettes, four divider weights |
| `Badge` / `Avatar` | Foundation | Seven badge appearances, four avatar sizes; Avatar renders initials only |
| `ProgressBar` | Partial | Determinate progress and an indeterminate sweep; reduced-motion handling and pixel baselines for the indeterminate mode are pending |
| `Tooltip` | Foundation | `Button::tooltip` uses a root-hosted anchored overlay with a configured 4px gap; hover and flip placement still need manual verification |
| `Popover` | Experimental | Anchored panel, controlled/uncontrolled open state and same-frame scroll positioning; dismissal, focus management and pixel baselines remain unfinished |
| `Menu` / `MenuItem` | Partial | Panel, clickable items, separators and headers; trigger integration, full keyboard/accessibility and submenus remain unfinished |
| `Spinner` | Blocked | Not implemented; the current stroked-arc rendering approach is blocked |

## Quick start

```toml
[dependencies]
fluent-ui = "0.1"
```

```rust
use fluent_ui::{Button, FluentRoot, FluentTheme};
use gpui::prelude::*;

struct App;

impl Render for App {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(Button::new("save").child("Save"))
    }
}

fn main() {
    fluent_ui::application().run(|cx: &mut App| {
        fluent_ui::init(cx);
        FluentTheme::web_light().install(cx);
        cx.open_window(Default::default(), |_, cx| cx.new(|_| App))
            .expect("failed to open window");
    });
}
```

`FluentRoot` applies the theme's background, foreground and font, and hosts the tooltip overlay, so the outermost element of an application should be wrapped in it.

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

Five themes, generated from the official Fluent design tokens:

```rust
FluentTheme::web_light().install(cx);
FluentTheme::web_dark().install(cx);
FluentTheme::teams_light().install(cx);
FluentTheme::teams_dark().install(cx);
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

- `Spinner` is not implemented; the current stroked-arc rendering approach is blocked
- `Avatar` renders initials only; no images, no status badge
- Mouse hover/pressed state wiring and colour transitions are implemented for Checkbox / Radio / Switch; full keyboard and pointer-cancellation behaviour is not yet verified
- `Popover` has no Escape/outside-click dismissal or full focus management; viewport edges clamp the panel rather than flipping it above/below
- `Menu` trigger integration, keyboard navigation, accessibility and submenus are unfinished
- Reduced-motion handling and pixel baselines for indeterminate `ProgressBar` are pending
- `Button::tooltip` delegates flipping to the gpui-base overlay; hover/flip placement still needs manual verification. The generic `tooltip(text)` helper uses GPUI-native positioning without the same anchoring guarantee
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
cargo test -p fluent-gallery --features screenshot --test baselines --locked
```

Pixel comparisons need the `screenshot` feature and a usable GPU surface; they currently run locally, not in CI. The 15 committed baselines cover Windows / light only, not dark/high-contrast or hover/focus/pressed/disabled. Directories currently distinguish OS only; DPI, fonts and rendering environment must also match. Set `FLUENT_BASELINE_UPDATE=1` only to accept an intentional visual change, review the image diff, then rerun with update mode unset. Do not re-record baselines instead of investigating a regression.

## License

Apache-2.0