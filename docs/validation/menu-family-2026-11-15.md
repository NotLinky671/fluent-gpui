# 菜单族（T-043）：触发器、分组、两行行高、高度上限与滚动

日期：2026-11-15。范围：`MenuTrigger`、`MenuGroup`、`MenuItem::secondary`、
`MenuItemCheckbox` / `MenuItemRadio`、`MenuHeader` 对齐、菜单高度上限与行滚动。
相关源码：

- `crates/fluent-core/src/spec/menu.rs`（`max_height`、`row_height`、次要行颜色与字号）
- `crates/fluent-components/src/menu.rs`（面板与行、`MenuTrigger`、分组展开、滚动跟随）
- `crates/fluent-components/src/menu/family_tests.rs`、`menu/scroll_tests.rs`
- `crates/fluent-gallery/src/sections/controls.rs`（演示）

## 修改前后

| 项 | 修改前 | 修改后 |
|---|---|---|
| 触发器 | 只有 `MenuDropdown`，调用方自己持有一个 `Button` 和开合状态 | `MenuTrigger::new(id, label, menu)` 自带非受控开合状态，可受控；键盘 Enter/Space 走按钮激活路径 |
| 菜单焦点 | 依赖调用方 `track_focus`；没传就没有键盘 | 调用方没传时按触发器 id 建 `tab_stop(true)` 焦点句柄，交给 `MenuDropdown` 跟踪；关闭后键盘回按钮 |
| 分组 | 只能 `header()` 手写，键盘会停在标题之外的行上无保证 | `MenuGroup` 在构建期展开成标题 + 行，标题不是键盘停点 |
| 两行行 | 只有 32px 单行 | `secondary()` 生成 48px（20 标签 + 16 说明 + 12 留白），字号/颜色来自 spec |
| 勾选/单选 | 只有 `MenuItem::checked/selected` | 追加 `MenuItemCheckbox` / `MenuItemRadio` 类型包装，渲染同一套行 |
| 高度 | 面板可以超出窗口，长菜单不可达 | `max_height = 视口高 − 2×边距`，超出时行区域滚动 |
| 滚动归属 | 无 | 只有行区域滚动；面板本身不滚动 |

## 命令与结果

`RUSTUP_TOOLCHAIN=stable`（本机 1.98.0 缺 `cargo.exe`）。

```text
cargo fmt --all --check                                      → exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings → exit 0
cargo test -p fluent-components --lib menu --locked          → 102 passed
cargo test --workspace --locked                              → 386 passed, 1 ignored
cargo doc --workspace --no-deps --locked                     → exit 0
cargo deny check                                             → advisories/bans/licenses/sources ok
cargo xtask gen-tokens --check / gen-icons --check           → exit 0
FLUENT_GALLERY_SMOKE=1 cargo run -p fluent-gallery --locked   → exit 0（编译 1m38s）
```

菜单相关分项：`menu::family_tests` 7 条、`menu::scroll_tests` 2 条、
`menu::submenu_regression_tests` 30 条、其余菜单测试 63 条。

`cargo deny check` 在本机沙箱下会以
`failed to acquire advisory database lock: … attempted to take an exclusive lock on a read-only path`
失败：它要写 `C:\Users\nothh\.cargo\advisory-dbs\db.lock`，而沙箱只允许写工作区
（同目录手写探针文件同样被拒）。放开写权限后同一命令退出码 0，因此这是环境限制，不是依赖问题；
CI 不受影响。

## 定向破坏证据

每条都是“改一处 → 只跑目标测试 → 还原（并 touch 文件防止 cargo 跳过重编）”。

| 破坏 | 目标测试 | 结果 |
|---|---|---|
| `snapshot_levels` 把标题也当成行 | `the_keyboard_walks_a_group_from_its_first_row` | 失败：`the first keyboard stop in a group must be its first row` |
| `row_height` 永远返回单行高 | `a_two_line_row_makes_the_menu_taller` | 失败：`three two-line rows must add 48px, got 0px (108px vs 108px)` |
| `MenuItemCheckbox::new` 直接禁用行 | `a_checkable_row_runs_its_handler_from_the_keyboard` | 失败：`a checkable row must activate like any other row` |
| 去掉 `MenuDropdown::track_focus(&menu_focus)` | `a_trigger_without_a_menu_focus_still_takes_the_keyboard` | 失败：`... must still leave the keyboard on its menu` |
| 去掉关闭时回焦按钮 | `the_trigger_closes_on_escape_and_reopens_from_the_keyboard` | 失败：`Enter on the focused trigger must reopen its menu` |
| 去掉面板 `.max_h(max_height)` | `a_menu_taller_than_the_window_stops_at_the_window` | 失败：`the panel must stop at the window edge, got 822px for a 24 row menu` |
| `PanelScroll::follow` 不调用 `scroll_to_item` | `the_keyboard_scrolls_the_current_row_into_view` | 失败：当前行没有被绘制 |

## 测出来的两个设计约束

1. **点击后不需要显式移交焦点。** 起初在触发器点击回调里加了 `focus.focus(window, cx)`，
   但把这段删掉以后 `a_trigger_without_a_menu_focus_still_takes_the_keyboard` 仍然通过：
   面板 `track_focus` 之后，弹层打开本身就拿到键盘。既然没有可观察差异，这段代码已删除，
   不留“看起来有用”的分支。
2. **滚动容器会裁掉子菜单。** 把 `overflow_y_scroll()` 放在整块面板上时，
   作为面板子元素、定位到面板之外的子菜单面板被一起裁掉，`submenu_regression_tests` 11 条立即失败
   （`one active row in the submenu: []`、`the third level must be painted on the first frame after Right`）。
   因此滚动只包行区域，`SubmenuPositioner` 仍挂在面板上。这条在 `docs/subsystems/menus.md` 第 7 节有记录。

## 未验证

- 真实原生窗口中的滚轮、触控板惯性、滚动条外观与 DPI 缩放；自动测试只覆盖 `max_h` 与
  `scroll_to_item` 的逻辑位置。
- 菜单放在页面滚动容器里时的嵌套滚动。
- `MenuTrigger` 在工具栏/标题栏（`WindowControlArea` 覆盖区）里的命中，以及按住 Space 的重复激活。
- 本版 accesskit 没有 `MenuButton` 角色，`div` 没有 `aria_haspopup`：触发器对屏幕阅读器只是一个普通按钮，
  不播报“弹出菜单/已展开”。已记入 `docs/a11y-checklist.md` 缺口 9。
- 分组标题的视觉间距只做了行高与 2px 间隔断言，未与 Fluent 截图逐像素比对。
