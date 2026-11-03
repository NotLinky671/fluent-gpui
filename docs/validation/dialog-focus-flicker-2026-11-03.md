# Dialog 页脚按钮闪烁排查记录（footer-flicker）

日期：2026-11-03。基线：HEAD `51460d9`（修复前 `f7baf54` 曾提出错误根因，本文已更正）；
gpui-base 固定 `0.8.1`。

## 报告的现象

图库 `Edit details` 对话框（`Dialog::new("gallery-dialog")`）页脚的 `Cancel`（Subtle）
按钮在**聚焦外观与未聚焦外观之间逐帧闪烁**。同一页脚模式也被 `fluent-notes` 的
`render_pending_dialogs` 复用。

## 真实窗口证据（`FLUENT_FOCUS_TRACE=1`，`cargo run -p fluent-gallery`）

为按真实渲染管线取证，`button.rs` 临时对 `gallery-dialog-cancel` 打印每帧
`hovered`/`pressed`/`kbd`/`focused`/`bg`，`main.rs` 临时自动打开对话框、把焦点驱进
页脚并把指针停驻在 Cancel 正中（560×360，完全落在按钮内）。

**旧代码（`f7baf54` 及更早，`on_hover` 驱动 `hovered`）**，指针死点停驻、
focus 恒定 `FocusId(35v1)`、`kbd=false`：

```text
f=0..19  hovered=true
f=20..35 hovered=false   ← 指针未移动却报 false，bg 整段淡出
f=36..   hovered=true    ← 再报 true，bg 整段淡入 → 可见闪烁
```

**最终代码（`c5d4ea3` 锁存的 `on_hover`：`false` 边沿仅在指针真正移出
bounds 时采纳）**，同一点位同条件，逐帧 `[flicker]` 跟踪：

```text
f=0..1   hovered=false pressed=false kbd=false focused=FocusId(32v1) bg.a=0.0
f=2..19  hovered=true  pressed=false kbd=false focused=FocusId(35v1)
         bg.a 单调 0→1.0 后稳定 —— 无 true→false→true 循环
```
（完整逐帧见 `target/flicker-logs/real-window-trace.txt`，两段 trace 分别标
注了所跑 build。）

结论：**focus 从未变化**（35v1 恒定），modality 恒定 —— 此前的“focus ping-pong /
was_open 重触发”根因是错的。真正抖动的是 `hovered`，进而 `state_color` 动画把
背景整段淡出/淡入 = 肉眼所见“聚焦/未聚焦外观”闪烁。

## 根因

`gpui` 的 `on_hover`/`is_hovered` 引导阶段（`gpui-pre-0.3.10/src/elements/div.rs:3212`）
在 prepaint 时读 `window.mouse_hit_test` —— 那是**上一帧**的命中测试。对话框子树在
`deferred` 下每帧重 prepaint，按钮的 `HitboxId` 每次重建，新 id 不在旧 `mouse_hit_test`
里 → `is_hovered` 返回 `false` → `on_hover` 发出一帧假的 `false` 边沿 → 下一个鼠标
事件再翻回 `true`。每帧一次假的 hover-out/hover-in，动画色整段循环 = 闪烁。

同一 stale 读也影响 `gpui` 自身的 `.hover()` 样式（`div.rs:2569` 同样读旧
`mouse_hit_test`），所以这是上游快照缺陷，无法在 gpui 内修。

## 修复（`crates/fluent-components/src/button.rs`）

`hovered` 仍由 `on_hover` 写回 keyed-state `hover_state`（保留 occlusion 感知的真实
enter/leave 边沿），但**对 `false` 边沿加锁存守卫**：`on_children_prepainted` 把按钮的
绘制 bounds 存入 keyed-state cell；`on_hover` 回调里仅当实时 `window.mouse_position()`
落在 bounds **之外**时才接受 `false`，指针仍在 bounds 内则把它判为 deferred 重绘产生的
假边沿而忽略（`genuine = reported || inside`）。`on_hover` 仍触发 `window.refresh()`
驱动 tooltip 的真实边沿。

- 颜色源 `hovered` = `hover_state`（对 stale `false` 免疫）；动画 `state_color`/`TransitionId` 不变。
- `pressed`、focus-ring、tooltip、a11y 语义不变；新增 bounds cell 与永久 wrapper div。
- **为何不直接命中测试**：曾尝试在 prepaint 插入探测 `Hitbox` 并在 render 时用
  `Hitbox::is_hovered_at` 查询 `rendered_frame`，但该 oracle 受 `rendered_frame` 与
  `next_frame`/`hover_hitbox_count` 分类影响，在真实窗口仍报假 `false`，故弃用（见下“未验证项”）。

**已知限制（如实记录）**：几何守卫是 **occlusion-blind**——当按钮被模态 scrim 覆盖但
指针仍在其几何 bounds 内时，`hovered` 仍报 `true`（`on_hover` 报 false 而 inside=true →
genuine=true）。gpui 的 `is_hovered` 在该场景为 false。此差异不可见，因为 scrim 在按钮
之上重绘；但偏离严格 occlusion 语义，由 `a_modal_dialog_paints_its_scrim_over_a_button`
固定现状。

## 回归测试

- `dialog::tests::a_parked_pointer_keeps_a_buttons_hover_colour`（新增）
  —— 指针死点停驻真实 `Button`，连续重绘下悬停背景始终绘制。
- `dialog::tests::a_keyboard_event_does_not_clear_a_parked_hover`（新增）
  —— 指针停驻时按 Tab，modality 翻成 keyboard 后悬停背景仍绘制（hover 不因
  modality 闪烁）。
- `dialog::tests::a_modal_dialog_paints_its_scrim_over_a_button`（新增，原
  `a_covered_button_still_reports_geometry_based_hover` —— 名字夸大了可观测面；
  它只断言 scrim 全屏 quad 覆盖按钮点位，occlusion 警示留在这份记录里）。
- `dialog::tests::an_open_dialogs_footer_buttons_hold_state_across_repaints`（新增）
  —— 真实 `Button` 页脚在连续重绘下 focused 句柄与卡片区四边形逐帧一致。
- 删除 `a_stationary_pointer_reports_a_steady_hover`：harness 内 `on_hover` 边沿
  本就稳定（无法复现 HitboxId churn），该断言对真实缺陷无约束力。

**红色证明**：确定性 `#[gpui::test]` 无法复现缺陷（harness 的 deferred prepaint 不
churn `HitboxId`，`mouse_hit_test` 语义与真实合成器不同），上述测试在新旧代码下
均通过 —— 它们锁定正确契约，非红测。真实的 before/after 在上面的窗口 trace：
旧 `hovered` 在死点停驻下 true→false→true，新代码恒定 true。

## 命令与结果

```text
cargo test -p fluent-components --lib dialog --locked   → 17 passed（+4 新测，-1 删测）
cargo test -p fluent-components --lib button --locked   → ok
cargo test -p fluent-components --lib motion --locked   → 2 passed
cargo test -p fluent-notes --locked                     → 33 passed
cargo clippy -p fluent-components --all-targets -- -D warnings → ok
cargo deny check                                   → advisories/bans/licenses/sources ok
```

日志/trace 见 `target/flicker-logs/`。`cargo deny` 按要求未运行。

## 未验证项
- `hovered` 由 `on_hover` 写回 `hover_state` 并对其 `false` 边沿做几何守卫；真实
  enter/leave 仍由 `on_hover` 触发 `window.refresh()` 重绘，若某路径漏发 mouse event
  （极少见）hover 色可能滞后一帧。
- **occlusion-blind 已知限制**：scrim 覆盖按钮而指针仍在其几何 bounds 内时 `hovered`
  仍报 true（`is_hovered` 为 false）；因 scrim 在其上重绘故不可见，已由
  `a_modal_dialog_paints_its_scrim_over_a_button` 固定。
- `window.refresh()` 与 keyed-state `cx.notify()` 一并保证重绘 —— 已验证在 gallery
  正常；其它宿主未逐一回归。
- 笔记应用 `render_pending_dialogs` 复用同一 `Button`，同获益；未单独跑真实窗口。
- 真实窗口 trace：合成 `dispatch_event` 不能移动真实 OS 指针，故停驻点按真实光标
  实际落点判定；两次运行均未见旧式 true↔false 振荡。
- `gpui-base` `test-support` feature 未启用，按钮 focused flag 改以四边形等价取证。
