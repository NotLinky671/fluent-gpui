# 审核修复：触发器的开合请求、关闭时的焦点交接、滚轮后的键盘定位

日期：2026-11-15。范围：`6614ce2` 审核提出的三个问题，以及它们各自的回归测试。
相关源码：

- `crates/fluent-components/src/menu.rs`（`MenuTrigger::render`、`MenuDropdown::restore_focus`、
  `MenuState::nav`、`navigate`、`PanelScroll::follow`）
- `crates/fluent-components/src/popover.rs`（`restore_focus`、关闭帧的焦点判据）
- `crates/fluent-components/src/menu/trigger_tests.rs`（新增，9 条）
- `crates/fluent-components/src/menu/scroll_tests.rs`（+2 条滚轮回归）

## 三个问题的根因

### 1. 受控触发器按内部镜像取反

`MenuTrigger::render` 渲染用的是 `self.open.unwrap_or(store)`，点击却写 `let next = !*store.read(cx)`。
受控时 store 只是**上一次请求**的镜像：调用方拒绝请求后它和画面分叉，第二次点击会请求
一个已经显示过的状态。复现：始终 `.open(false)`、`on_open_change` 只记录，两次点击得到
`[true, false]`；预期 `[true, true]`。

修法：点击时先取“屏幕上的值”（受控读 `open`，非受控读 store），再取反提交。

### 2. 关闭时提前抢焦点，Tab 交接失效

旧代码在 `on_open_change(false)` 里立刻 `button_focus.focus(window, cx)`。
`Popover` 的关闭分支用 `panel_focus.contains_focused(window, cx)` 判断“焦点是否还在面板里”，
只有为真才恢复上一个控件并执行 Tab 交接。抢焦点把这个判据变成假，于是菜单关了、键盘停在了
触发按钮上，既不前进也不后退。

修法：`MenuTrigger` 不再自己移动焦点，改为把按钮句柄交给 `Popover::restore_focus`：

- 打开时 `previous = window.focused(cx).or_else(restore_focus)`——没有“打开前焦点”的场景
  （程序化打开）也有地方可回，否则关闭会走 `missing_restore` 分支把键盘送往下个控件；
- 关闭时仍由 Popover 判断 `inside`：焦点在面板里就恢复并交接；用户点了别的控件就什么都不做；
  调用方在关闭回调里主动移动了焦点，`inside` 为假，同样不撤销。

### 3. 滚轮之后同名行不会重新定位

`PanelScroll::follow` 只比较行 id：`if shown == row { return; }`。
滚轮把当前行滚出视野后，再按同一个 End/Home，行 id 没变，`scroll_to_item` 被跳过，行仍在视口外。

修法：`MenuState` 增加 `nav: Option<NavRequest> { depth, epoch }`，`navigate` 每次成功移动
都递增 epoch 并记录层级深度；`follow(row, index, request)` 在“行变了”或“收到本层尚未服务过的
请求”时滚动。普通重绘不带请求，因此不会把用户滚到的位置拽回当前行。

顺带核实：`overflow_y_scroll` 本身已经提供滚轮——`vendor/gpui-pre/src/elements/div.rs:3412`
的 `paint_scroll_listener` 在调用过 `track_scroll` 时注册鼠标滚轮监听并累加 `offset.y`。
因此没有另外实现滚轮处理，回归测试直接派发 `PlatformEvent::Scroll`。

## 修改前后

| 项 | 修改前 | 修改后 |
|---|---|---|
| 受控点击 | 取反内部 store，调用方拒绝后请求与画面不一致 | 取反屏幕上的值；拒绝两次得到 `[true, true]` |
| 关闭焦点 | `on_open_change` 里直接聚焦按钮 | 交给 `Popover::restore_focus`，由关闭帧的 `inside` 判据决定 |
| Tab/Shift+Tab | 关闭后焦点停在按钮，不前进 | 仍按方向交接给相邻控件 |
| 无“打开前焦点” | 关闭时把键盘送到下个控件 | 回到触发按钮（`restore_focus` 兜底） |
| 外部点击关闭 | 可能抢焦点 | 焦点留在被点的控件上 |
| 滚轮后同一行 | 不再重新定位 | 明确的键盘请求一定把该行带回视野 |

## 命令与结果

`RUSTUP_TOOLCHAIN=stable`（本机 1.98.0 缺 `cargo.exe`）。

```text
cargo fmt --all --check                                        → exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings → exit 0（仅既有 3 条 unused dependency 警告）
cargo test -p fluent-components --lib menu:: --locked          → 112 passed
cargo test --workspace --locked                                → 397 passed, 1 ignored
cargo doc --workspace --no-deps --locked                       → exit 0
```

菜单相关：`menu::trigger_tests` 9 条、`menu::scroll_tests` 4 条（其中 2 条为本次新增的滚轮回归），
其余菜单测试 99 条。

## 定向破坏证据

脚本 `target/breakage_review.py`：改一处 → 只跑目标测试（`-- --exact`）→ 还原并 `os.utime`
强制重编。四条全部按预期变红。

| 破坏 | 目标测试 | 结果 |
|---|---|---|
| 受控点击改为取反 store | `a_rejected_controlled_request_is_asked_again`、`a_controlled_trigger_toggles_the_state_on_screen`、`props_the_internal_default_disagrees_with_are_what_the_button_toggles` | 3 条失败（请求序列与预期不符） |
| 关闭回调里立即 `button_focus.focus` | `tab_hands_the_keyboard_to_the_next_control`、`shift_tab_hands_the_keyboard_backwards` | 2 条失败（焦点停在触发按钮） |
| Popover 去掉 `restore_focus` 兜底 | `escape_with_no_previous_focus_returns_to_the_trigger` | 失败（键盘被送往下一控件，Enter 打不开） |
| `follow` 只看行 id、忽略键盘请求 | `the_keyboard_brings_the_current_row_back_after_the_wheel`、`home_brings_the_first_row_back_after_the_wheel` | 2 条失败（滚回后当前行仍在视口外） |

## 未验证项

- **真实原生窗口**：本轮全部结论来自测试窗口里派发的平台事件与场景 quad 断言，
  没有在 Windows/macOS/Linux 原生窗口上用真实滚轮、触控板或 Tab 键复验。
- **滚轮惯性/高精度滚动**：测试只发单次 `ScrollDelta::Pixels`；触控板惯性、`Lines` 单位、
  横向滚动未覆盖。
- **嵌套滚动容器**：菜单自身滚动已验证；把它放进页面滚动容器时的滚轮归属未验证。
- **屏幕阅读器**：`aria-haspopup` / 展开状态仍不可表达（accesskit 0.24.1 无 `MenuButton`），
  见 [无障碍清单](../a11y-checklist.md) 缺口 9。
- **性能**：`follow` 的请求判断是常数开销，但长菜单每帧重建测量与边界集合的成本仍未测量。
