# Accordion：规格、行为与验证（2026-11-17）

## 需求

用户要求按 <https://fluent2.microsoft.design/components/web/react/core/accordion/usage> 实现 Accordion，完成后提交。设计页是客户端渲染，`web_fetch` 只拿到导航壳，因此规格以 Fluent UI React 源码为准（`@fluentui/react-accordion` 9.12.5 的 `useAccordionHeaderStyles.styles.js`、`useAccordionPanelStyles.styles.js`、`useAccordion.js`、`useAccordionHeader.js`）。

## 改了什么

| 层 | 文件 | 内容 |
|---|---|---|
| 规格 | `crates/fluent-core/src/spec/accordion.rs` | `AccordionSize`、标题/面板/颜色规格、`header_colors`、`is_open`、`toggled`、`normalize_default` + 10 条单测 |
| 组件 | `crates/fluent-components/src/accordion.rs` | `Accordion` / `AccordionItem` / `AccordionToggle`：keyed 打开集合、逐头 focus handle、点击与方向键、`focus_visible` 焦点环、面板按规格外边距 |
| 导出 | `crates/fluent-components/src/lib.rs`、`crates/fluent-ui/src/lib.rs` | 三个类型与 `AccordionSize` |
| 图库 | `crates/fluent-gallery/src/sections/controls.rs`、`src/main.rs` | `accordion_section`：单选 + 禁用项、多选可折叠、32px 带图标三列；`FLUENT_GALLERY_SECTION=accordion` 可单开 |
| 文档 | `docs/subsystems/accordion.md`、三个 README 入口、`docs/a11y-checklist.md` | 规格来源、状态归属、键盘与焦点语义、差异与未验证项 |

## 两处实测才发现的问题

**1. `Enter` / `Space` 被处理了两次。** 首版在 `on_key_down` 里自己处理 `"enter" | "space"`，同时标题又带 `role(Button)` + `on_click`。把那一支改成永不匹配（`"never-pressed" => {}`）后重新跑 `the_keyboard_toggles_the_focused_header`，测试**仍然通过**——说明 GPUI 自己的点击激活已经覆盖这两个键，我那一支是重复实现，且真按下去有双重切换的风险。已删除该分支，只保留 `↑` / `↓` / `Home` / `End`（移动焦点、不切换、不环绕、跳过禁用项）。

**2. 焦点环一开始是自造的 canvas 描边。** 用户质问为何自建焦点环后核对：`gpui-pre` 没有任何焦点环 API，`gpui-base` 的 `focus_ring()` 只是状态位，仓库里唯一既有的写法是 `Button` 的 `focus_visible` + 内阴影。旧实现有两个缺陷——用 `contains_focused` 判断，鼠标点击也会亮环（Fluent 是 `:focus-visible` 语义）；以及多出一条不共享的绘制路径。改为

```rust
header = header.focus_visible(move |style| {
    style.border_1().border_color(ring_colour).shadow(vec![BoxShadow {
        color: ring_colour,
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: ring_thickness,
        inset: true,
    }])
});
```

`focus_visible` 只在 `focused && window.last_input_was_keyboard()` 时生效（`vendor/gpui-pre/src/elements/div.rs:3517`），这正是要的语义。实测浅色主题 `stroke_focus_2` 是纯黑 `Hsla { h: 0.0, s: 0.0, l: 0.0, a: 1.0 }`，与 Fluent 的 `colorStrokeFocus2` 一致。

## 行为测试（`crates/fluent-components/src/accordion/tests.rs`）

| 测试 | 断言的可观察结果 |
|---|---|
| `clicking_a_header_opens_its_panel` | 点击后绿色面板体出现一次 |
| `a_collapsible_accordion_closes_on_a_second_click` | 再点一次，面板体消失 |
| `a_non_collapsible_accordion_keeps_its_panel_open` | 不可折叠时第二次点击仍有一块 |
| `a_non_collapsible_multiple_accordion_keeps_its_last_panel` | 多选 + 不可折叠：唯一一块不会被点掉 |
| `a_multiple_collapsible_accordion_closes_its_last_panel` | 多选 + 可折叠：可以全部收起 |
| `multiple_keeps_the_first_panel_when_the_second_opens` | 打开第二块时第一块仍在（第二行已下移 40px） |
| `a_disabled_header_ignores_clicks_and_keeps_its_panel_closed` | 禁用标题点击无效 |
| `the_panel_body_sits_inside_the_panel_margins` | 面板体 `origin.x = 12px`、宽度 336px |
| `the_keyboard_toggles_the_focused_header` | Tab 进标题、Enter 打开 |
| `arrow_down_moves_focus_without_toggling` | ↓ 不切换、Enter 打开的是**第二块**（`origin.y ≥ 32 + 88`） |
| `the_focus_ring_appears_only_for_the_keyboard` | Tab 后恰好一个 2px 环，鼠标点击后 0 个 |

## 定向破坏（每条都先改代码、只跑目标测试、再还原）

| 破坏 | 结果 |
|---|---|
| 把焦点环颜色改成没人检查的颜色（`spec.rest.foreground`） | `the_focus_ring_appears_only_for_the_keyboard` 失败：`left: 0, right: 1` |
| 去掉面板左边距（`.ml(px(0.))`） | `the_panel_body_sits_inside_the_panel_margins` 失败：`left: 0px, right: 12px` |
| 多选分支去掉“最后一块需 collapsible”守卫 | `a_non_collapsible_multiple_accordion_keeps_its_last_panel` 失败：`left: 0, right: 1` |
| 方向键 `"down"` 不再计算目标 | `arrow_down_moves_focus_without_toggling` 失败（面板属于第一行） |
| 键盘分支永不匹配 `enter`/`space` | 测试**仍通过** → 说明 GPUI 已激活，该分支是重复实现，已删除 |

## 检查结果

```text
cargo fmt --all --check                                    exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings   exit 0（仅 3 条既有 unused workspace dependency 警告）
cargo test --workspace --locked                            exit 0，446 passed / 1 ignored
cargo test -p fluent-core --lib spec::accordion --locked   10 passed
cargo test -p fluent-components --lib accordion --locked   11 passed
cargo doc --workspace --no-deps --locked                   exit 0
cargo deny check                                           advisories/bans/licenses/sources ok（需 danger-full-access 取 advisory 锁）
cargo xtask gen-tokens --check / gen-icons --check          up to date
FLUENT_GALLERY_SMOKE=1 cargo run -p fluent-gallery --locked exit 0
```

## 原生窗口

- 用 `PrintWindow(.., PW_RENDERFULLCONTENT)` 截取了图库的 `accordion` 分区（1372×971，150% 缩放）：三列渲染正常——Overview 展开、Details 收起、Policy 灰色禁用；第二列两块默认展开；第三列 32px 小字号带图标。标题 44px、面板左边距 12px 都在像素上对得上。
- **焦点环的原生截图没拿到**：新启动的窗口没有任何已聚焦元素，GPUI 的 `focus_next` 在“没有当前焦点”时什么也不做，所以脚本发出的 Tab 不会移动焦点（这是应用外壳的现状，不是 Accordion 的缺陷；已经记成待办）。`ring-tab*.png` 与基准帧逐字节相同，可证明键根本没生效。
- 因此焦点环的正确性目前只有测试级证据：`painted_quads()` 里出现 `border_color == stroke_focus_2`、`border_widths.top == 2px` 的矩形，且仅在键盘输入后出现。

## 仍未验证

- 真实屏幕阅读器对 `aria-expanded` / `role=Button` 的读数；本机 `AXIsProcessTrusted()` 为 false。
- 焦点环在深色与高对比主题下的观感（颜色来自 token，未截图）。
- 嵌套 Accordion、面板内的滚动容器、长内容导致的高度变化。
- macOS / Linux 的渲染与字体度量；本机只有 Windows。
- 与官方设计页的像素比对（设计页无法抓取正文，只能对源码数值）。
