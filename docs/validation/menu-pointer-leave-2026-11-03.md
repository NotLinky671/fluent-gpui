# 菜单：指针离开后清除当前项

## 目标与范围

- 关联问题：鼠标移开菜单后，指针选中的行仍保留「当前项」高亮。
- 基准 SHA：`0fa65a4eb2fb53c1edfaf978ec0ea69a13f63606`。被测对象包含其上的未提交修改；**未创建提交**。
- 差异范围：`crates/fluent-components/src/menu.rs` 的当前项状态与容器事件、`menu/input_mode_tests.rs` 新增测试、`docs/subsystems/focus.md` 规则说明。
- 不在范围内：typeahead、子菜单、Tab 离开关闭、原生视觉验收。

## 实现

当前项状态从「行身份」扩展为「行身份 + 由谁写入」：

```text
CurrentRow { id: ElementId, from_pointer: bool }
```

- 行内鼠标移动 → `from_pointer = true`
- 方向键 / Home / End → `from_pointer = false`
- 菜单容器收到 hover 结束 → 仅当 `from_pointer` 时清除

**为什么按来源区分**：无条件清除会让指针从菜单外划过时抹掉刚按方向键选中的行。无条件保留则是本次要修的原状。两者都不是「谁最后动谁说了算」。

**为什么不用 `on_hover` 行内事件**：`on_hover` 会在布局变化把另一行移到静止指针下时触发，等于把「没有输入」当成用户选择。行内接管沿用既有实现的 `on_mouse_move`（本轮未改动该选择）。

## 可观察行为

| 操作 | 修改前 | 修改后 |
|---|---|---|
| 悬停 Two 后把指针移出菜单 | Two 仍高亮，Enter 执行 Two | 高亮消失，Enter 无动作 |
| 方向键选中 One 后指针在菜单外移动 | One 保持 | **不变**（One 保持） |
| 悬停 Two → 移出 → 按 Down | 从 Two 继续 → Three | 从无选中开始 → One |

调用方影响：无公开 API 变化；动态菜单仍要求稳定且唯一的行 ID。

## 回归证据

新增 3 条测试，均含**真实绘制断言**（`window.painted_quads()` 的行背景数量、颜色、缩放后坐标），不只是回调断言：

- `leaving_the_menu_clears_a_pointer_selected_row`
- `a_pointer_leave_does_not_wipe_out_a_keyboard_selection`
- `keyboard_navigation_after_leaving_starts_from_the_top`

定向破坏：移除容器的「指针离开 → 清除」接线后，`leaving_the_menu_clears_a_pointer_selected_row` **失败于填充断言**（仍存在一块 y=76scaled 的行背景），证明测试能发现高亮残留；恢复后 15/15 通过。

## 追加：指针移到不可选行（标题 / 分隔线）

首轮修复只在**离开菜单容器**时清除，因此指针停在「Actions」标题上时，上一行仍然保持当前项外观——标题在容器内部，容器没有收到「离开」。

- 标题与分隔线各自包一层带 id 的容器，并上报 `on_mouse_move`；收到时按同一规则清除**由指针选中**的当前项。
- 键盘选中不受影响：标题上报只清 `from_pointer` 的行。
- 分隔线仍在其包装层内渲染 `MenuSeparator`，视觉不变。
- 未覆盖：面板 4px 内边距与行间 2px 间隙。指针停在这些位置上仍保留当前项；这属于可继续收紧的边界，本轮未处理。

新增测试（均含绘制断言 `painted_quads`）：

- `hovering_a_header_clears_a_pointer_selected_row`：悬停首行 → 填充数 1；移到标题 → 填充数 0，Enter 无动作。移除标题的指针上报后该测试失败（`moving onto the header must clear the row the pointer selected`，实际仍为 1 块填充）。
- `a_header_does_not_clear_a_keyboard_selection`：方向键选中后移到标题，填充数仍为 1，Enter 仍激活该行。

## 追加：指针移到禁用行

上一轮让标题与分隔线上报指针，但**禁用行**在 `MenuItem` 里被整块跳过（`if !self.disabled` 才安装指针处理），所以移到禁用行时同样无人清除。

- 指针上报不再由行自己决定语义：`Menu` 给可选行传「选中我」，给禁用行传「清除当前项」；`MenuItem` 只负责把指针事件转发出去。
- 禁用行仍不接受点击、不显示手型光标、不会是 `active`、不可被 Enter 激活。
- 单独使用的禁用 `MenuItem` 仍无 hover 样式（`pointer_activate` 为 `None` 时只在启用时应用局部 hover）。

新增测试（含绘制断言）：

- `hovering_a_disabled_row_clears_a_pointer_selected_row`：悬停 Delete（填充 1）→ 移到禁用 Archive（填充 0，且 Archive 未被选中）→ Enter 无动作。把禁用行改回不上报指针后该测试失败。
- `a_disabled_row_does_not_clear_a_keyboard_selection`：方向键选中后移到禁用行，填充仍为 1，Enter 仍激活该行。

## 验证记录

环境：Windows x86_64。命令在基准 + 未提交修改上运行，串行执行。

| 命令 | 结果 |
|---|---|
| `cargo fmt --all --check` | 通过 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 |
| `cargo test --workspace --locked` | 通过，136 个测试、0 失败 |
| `cargo doc --workspace --no-deps --locked` | 通过 |
| `cargo deny check` | 通过（首次运行因拉取公告库瞬时失败，连续两次重跑 exit 0，输出 advisories/bans/licenses/sources 全部 ok） |
| `cargo xtask gen-tokens --check` / `gen-icons --check` | 通过 |
| `cargo test -p fluent-components --lib menu --locked` | 通过，19 条键鼠测试 |
| `FLUENT_GALLERY_SMOKE=1 cargo run -p fluent-gallery` | exit 0 |

## 未验证

- 原生窗口中的真实指针移出行为与视觉（本轮只有测试平台事件和 scene quad 断言，无截图或像素基线）
- dark / high-contrast 下的同一表现
- 三平台 CI

## 交付状态

未提交、未推送；工作区保留供复核。
