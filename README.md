# fluent-ui

A Rust UI component library built to the **Microsoft Fluent 2** visual specification and rendered with **GPUI**. The goal is consistent component appearance, interaction behaviour and developer experience across Windows, macOS and Linux.

**English** | [简体中文](README.zh-CN.md)

> Status: **M1 (component foundation)**. The API is not stable yet; expect breaking changes before 0.1.

## Components

| Component | Status | Notes |
|---|---|---|
| `Button` | ✅ | Primary / Secondary / Subtle / Transparent, plus Toggle, Compound, Menu and Split variants |
| `TextInput` | ✅ | Single line, with leading/trailing content, invalid and disabled states |
| `Textarea` | ✅ | Multi-line, three appearances |
| `Field` / `Label` | ✅ | Vertical and horizontal layout, hint text, Error/Warning/Success validation messages |
| `Checkbox` / `Radio` / `Switch` | ✅ | Controlled and uncontrolled modes |
| `Link` / `Divider` | ✅ | Three link palettes, four divider weights |
| `Badge` / `ProgressBar` / `Avatar` | ✅ | Seven badge appearances, four avatar sizes |
| `Tooltip` | ✅ | Anchored to the trigger, flips above/below, 4px gap |
| `Spinner` | ⏸ | Waiting on a rotation animation foundation |

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

**Styling is a pure function.** `spec::<component>::resolve(&FluentTheme, &Props) -> Spec` needs no window, so visual decisions are unit-testable. Components only paint the `Spec` they are given, which lets "is this correct?" be pinned down without rendering anything.

**Rendered output outranks CSS constants.** When the CSS values in Fluent UI React v9 disagree with what Fluent 2 actually renders, the rendering wins (see the ADR-0002 addendum in `plan/01-decisions.md`). The switch thumb and the radio dot were both corrected against measured pixels.

GPUI is pinned to an exact snapshot (`gpui-pre = 0.3.10`); a caret requirement would drift onto an incompatible API.

## Known limitations

- `Spinner` and the indeterminate `ProgressBar` are not implemented — both need looping animation
- `Avatar` renders initials only; no images, no status badge
- Hover states are wired for Checkbox / Radio / Switch; pressed states are not yet
- No RTL support, no letter-spacing control, no CSS `outline`
- No live regions; nothing is announced to screen readers on change
- High contrast has to be selected manually; there is no system-level auto-detection
- IME composition has not been tested

## Development

```bash
cargo test --workspace          # unit tests
cargo xtask gen-tokens --check  # generated tokens are up to date
cargo xtask gen-icons --check   # generated icons are up to date
cargo xtask tasks --check       # task index is up to date
cargo deny --offline check      # licence and security audit
cargo test -p fluent-gallery --features screenshot --test baselines   # pixel baselines
FLUENT_GALLERY_SECTION=buttons cargo run -p fluent-gallery            # single component preview
```

Pixel baselines need the `screenshot` feature and a GPU surface, so they currently run locally only. After an intentional visual change, regenerate them with `FLUENT_BASELINE_UPDATE=1` and review the image diff.

## License

Apache-2.0