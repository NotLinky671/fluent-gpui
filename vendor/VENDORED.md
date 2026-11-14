# Vendored GPUI patches

These directories are copies of the registry sources, except for the trimmed
trees, manifest target toggles and public-hook patches described below. They are
path patches only: versions, features and dependencies are unchanged, and
nothing is upgraded.

| Directory | Package | Version | Upstream `.crate` SHA-256 | Lock checksum |
|---|---|---|---|---|
| `vendor/gpui-base` | `gpui-base` | 0.8.1 | `d862bc8f2e27710039013d075ac4995892e9687ff207fa530f880cbcd513936f` | `d862bc8f2e27710039013d075ac4995892e9687ff207fa530f880cbcd513936f` |
| `vendor/gpui-pre` | `gpui-pre` | 0.3.10 | `df4b5f8d0c278308df2f11e0bfe0212dd77d7848ce1bdc4b9d513334414396f6` | `df4b5f8d0c278308df2f11e0bfe0212dd77d7848ce1bdc4b9d513334414396f6` |

Source: `~/.cargo/registry/src/index.crates.io-*/\<package\>-<version>/`
(crates.io index `index.crates.io-1949cf8c6b5b557f`).

`gpui-base/.cargo_vcs_info.json` records upstream commit
`99576db58ec8103417c41a092e891b4022d24e7e` (`crates/base`); `gpui-pre` 0.3.10
has no VCS metadata file in its package. Both `LICENSE-APACHE`, `Cargo.toml`,
`Cargo.toml.orig` and the upstream README are kept with the source.

Excluded from the copies because they are not needed to compile the library
targets: `.cargo-ok`, the crate-local `Cargo.lock`, and the `tests/`, `benches/`
and `examples/` trees. The copied manifests set `autotests`, `autoexamples` and
`autobenches` to `false`, disabling those targets; `Cargo.toml.orig` keeps the
upstream target declarations. Dependencies, features and the `[lib]` target are
unchanged. Re-copy from the registry source and re-apply the hook patches if the
pins ever change.

## Reproducible patches

[Exporter/checker](../scripts/vendor-patches.py): Python 3 standard library plus Git. Reads checksum-verified .crate archives from the Cargo cache (or --cache); trims only the directories listed above; writes [gpui-base.patch](patches/gpui-base.patch) and [gpui-pre.patch](patches/gpui-pre.patch). Reconstructs every retained file in a temporary target/vendor-patch-check directory and compares with vendor. No source modification or network downloads. Unified-diff blank context lines require a single space; .gitattributes preserves this format by disabling only blank-at-eol checks for these patch files.

```text
python scripts/vendor-patches.py
python scripts/vendor-patches.py --check
```

A missing or wrong-checksum archive fails explicitly. Run cargo fetch --locked or supply --cache with the exact upstream archives. For a future pin, update version/checksum metadata deliberately, regenerate and review patches. Trimmed upstream tests remain unavailable; Fluent behavior tests cover the hooks it consumes.

## Applied local hooks

- `vendor/gpui-base/src/input/base/state.rs`: added
  `pub fn accessibility(&self) -> Option<&InputAccessibility>` as an inline
  borrow next to `set_accessibility`. No render, dependency, feature or version
  change. Fluent's `TextInput::accessibility_label` uses it to merge a name
  while preserving an existing role, placeholder, accessibility id and
  `value_exposed`; it only writes when the merged projection actually changes.
- `vendor/gpui-pre/src/platform/test/window.rs`: the test window now stores the
  real `a11y_init` callbacks and exposes `activate_accessibility()`.
- `vendor/gpui-pre/src/app/test_context.rs`: `enable_accessibility()` invokes
  that activation once for every existing test window. AX-tree tests then draw
  and read `Window::debug_a11y_tree_json`; there is no production env hook or
  polling loop.
- `vendor/gpui-pre/src/elements/div.rs` + `src/window/a11y.rs`: `aria_modal`
  and `aria_hidden` builders. Hidden nodes and their descendants are suppressed
  from the published tree (layout/paint unchanged); focus/active-descendant
  registration under a hidden ancestor is ignored. While any node is modal,
  the tree keeps only the topmost modal subtree (the last modal node with its
  descendants, reattached to the window root), so the background disappears
  until the modal closes and is rebuilt next frame. Only the explicit
  `aria_modal` flag triggers this; roles such as `Dialog` or `Menu` do not.
- `vendor/gpui-pre/src/window/a11y/debug.rs`: the debug JSON also reports
  `modal` and `hidden` so native checks can assert those states.
- `vendor/gpui-base/src/dialog.rs`, `alert_dialog.rs`: `accessibility_label`
  and `aria_modal` (default `true`) on the host, forwarded to the host div as
  `aria_label` / `aria_modal`. `AlertDialog` forwards both. Fluent's
  `Dialog`/`AlertDialog` use the `title` text as the default host name;
  `accessibility_label` overrides it, while `title_element` custom content
  supplies no name.

- `vendor/gpui-pre/src/platform/test/window.rs`,
  `vendor/gpui-pre/src/app/test_context.rs`: test-only
  `replace_and_mark_text` / `commit_text` drive the stored platform input
  handler, so CJK composition tests exercise the real IME path instead of a
  sink callback.
- `vendor/gpui-pre/src/window.rs`: test-support `painted_paths()` returns the
  last frame's scene paths, and deferred draws capture the current a11y node as
  their owner; `src/window/a11y.rs` uses that owner to keep overlays opened from
  inside a modal instead of guessing by role.
- `vendor/gpui-base/src/input/base/state.rs`: disabled builder/setter updates
  the stored focus handle Tab-stop flag. Disabled text fields leave Tab order
  and rejoin when enabled.
- `vendor/gpui-pre/src/window.rs`: test-support `painted_monochrome_sprites()`
  exposes painted SVG/glyph colours for Fluent icon regression assertions.
- `vendor/gpui-base/src/input/editor/highlighting.rs`: input selection prefers
  `InputEditorStyle::selection`, then the theme's `colors.selection`, and only
  falls back to `colors.accent` (alpha 0.4) when the theme has no selection
  token. This fixes the near-white accent highlight while keeping explicit
  caller styles first.
