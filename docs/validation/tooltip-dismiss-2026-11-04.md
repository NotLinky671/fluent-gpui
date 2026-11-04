# Tooltip 移开后不消失

日期：2026-11-04。基线：HEAD `e2501be`（`main` 已推送到此）。

## 现象

图库里 `Hover for tooltip` 按钮：指针移开后，`Saves the current document` 气泡一直留在屏幕上。

## 根因

`Button` 的 tooltip 接线用 `on_hover` 边沿驱动 `TooltipOverlay::request_show` / `request_hide`，
但「是否已经请求过显示」这个标志是**render 内部的局部变量**：

```text
let last_requested = Rc::new(Cell::new(false));   // 每次重绘都新建
...
if genuine && !requested.get()      { requested.set(true);  request_show }
else if !genuine && requested.get() { requested.set(false); request_hide }
```

流程是：

1. 指针进入 → `genuine = true`，`requested` 为 false → 请求显示，并把标志置 true。
2. 标志变更触发重绘 → **render 重新执行，新建的 `requested` 又是 false**。
3. 指针离开 → `genuine = false`，但 `requested.get()` 已是 false → 隐藏分支永远不成立。

所以显示能发出去、隐藏发不出去，气泡滞留。这也解释了为什么"移开不消失"只在真正显示过之后出现。

## 修复

`crates/fluent-components/src/button.rs`：把该标志放进 window keyed state
（`button-tooltip-shown-<id>`），跨重绘保持。

同时把 `self.id` 在交给 `gpui_base::Button::new` 之前克隆为 `element_id`，供 keyed state 使用。

## 回归证据

新增 `crates/fluent-components/src/button/tooltip_tests.rs`：

- 静止帧记录绘制块数量 → 悬停 → 推进 700ms（`show_delay` 为 500ms）→ 断言绘制块**变多**（气泡出现）
- 指针移到窗口外 → 推进 500ms（`grace_period` 为 300ms）→ 断言绘制块数量**回到静止值**

把标志临时改回「每帧当作未请求」后，测试失败并报
`the tooltip must be gone after the pointer leaves, still painting 3 quads`；恢复后通过。

## 验证

| 命令 | 结果 |
|---|---|
| `cargo fmt --all --check` | 通过 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 |
| `cargo test --workspace --locked` | 通过，210 个测试 |
| `cargo doc --workspace --no-deps --locked` | 通过 |

## 未验证

- 真实窗口目视（只有测试窗口的绘制块计数）
- 键盘焦点触发 tooltip、滚动跟随、翻转定位（既有缺口，见浮层专题）
- 按钮在 tooltip 显示期间被卸载时的行为
- 三平台 CI
