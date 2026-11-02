# 焦点：失焦时机与 Tab 顺序

先从[子系统导航](README.md)确认问题类型。本页记录焦点归属、失焦触发与 Tab 遍历的当前机制，以及哪些行为尚未验证。

## 入口与职责

| 入口 | 实际路径 | 不应混同的能力 |
|---|---|---|
| `FluentRoot` | 携带 `ROOT_KEY_CONTEXT`，并处理 `FocusNext` / `FocusPrev` | 不等于 gpui-base 的 `Root` 视图；两者 context 不同 |
| `TextInput` / `Textarea` | 点击自身聚焦；`on_mouse_down_out` 失焦 | 不包含 Tab 遍历与焦点陷阱 |
| `Popover` | 可选 `track_focus`，Escape 依赖它 | 不负责把焦点移入面板或归还 |

## 失焦时机

**GPUI 不会在点击别处时自动清除焦点**，`gpui-base` 的文本引擎同样不做。这一点容易误判：`window.blur(cx)` 在 gpui-base 中出现的位置都是测试代码，不是实现。

因此“点击别处失焦”是组件自身的责任：

```text
点击输入框内部  → on_mouse_down → state.focus()
点击输入框外部  → on_mouse_down_out → is_focused() 为真时 window.blur(cx)
```

`on_mouse_down_out` 在 capture 阶段触发，判据是“指针不在本元素 hitbox 内”，所以点输入框内部不会误触发。先判 `is_focused` 再 blur，避免对未聚焦的字段做无谓调用。

`TextInput` 与 `Textarea` 都按这个形状实现，两者缺一不可——它们是各自独立的组件，没有共享基类。

## Tab 顺序

`gpui_base::root::init` 确实绑定了 `tab` / `shift-tab`，但它限定在 gpui-base 自己 `Root` 视图的 context 上（该常量在 gpui-base 内是私有的）。应用挂载的是 `FluentRoot`，**不会**带上那个 context，所以那条绑定在 Fluent 应用里从不触发。

当前做法：

- `fluent_components` 定义 `FocusNext` / `FocusPrev` 与 `ROOT_KEY_CONTEXT`
- `fluent_ui::init` 把 `tab` / `shift-tab` 绑到这两个 action
- `FluentRoot` 设置 `ROOT_KEY_CONTEXT` 并处理两个 action，转发给 `window.focus_next` / `focus_prev`

不直接复用 gpui-base 的 context，是因为那个字符串是私有实现细节；自己绑定语义明确，也不受上游改动影响。

## 验证入口

```text
cargo test -p fluent-components --lib focus_tests --locked
cargo test -p fluent-components --lib tab_order_tests --locked
```

| 测试入口 | 检查的承诺 |
|---|---|
| `pressing_inside_the_field_focuses_it` | 点击字段内部聚焦 |
| `pressing_outside_the_field_blurs_it` | 点击字段外部失焦；移除接线后该测试失败 |
| `pressing_inside_again_keeps_focus` | 再次点击内部不误失焦 |
| `tab_moves_focus_to_the_next_field` | Tab 在两个字段间前进；移除 `FluentRoot` 的 context 后失败 |
| `shift_tab_moves_focus_backwards` | Shift+Tab 反向移动 |

## 尚未覆盖

- 焦点陷阱与模态（gpui-base 有 `focus_trap`，本项目尚未接入）
- 打开浮层时把焦点移入面板、关闭后归还
- 方向键在菜单项之间的移动、`aria-activedescendant`
- AccessKit role/state 是否随焦点正确上报
- 真实窗口中的 Tab 顺序（测试直接派发按键，不经过平台输入法或原生焦点环）
- 禁用控件是否被跳过（依赖 gpui-base 的 `tab_index`，本项目未设置）

## 修改后要同步什么

改变焦点归属、失焦触发条件、Tab 遍历方式或相关公开 API 时，同步本页、[总导航](README.md)、中英文 README 的对应行与测试。
