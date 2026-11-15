# 右键菜单（T-044）验证记录 — 2026-11-15

被测版本：`0b0d1e5` + 本次未提交改动（分支 `main`）。
范围：`ContextMenu` 组件、`Popover` 的指针锚点、spec 的 `context_menu_origin`、图库分区与文档。

## 修改前后

| 场景 | 修改前 | 修改后 |
|---|---|---|
| 在区域内右键 | 无实现 | 面板以指针位置为左上角打开，只在打开时上报一次 |
| 指针贴着窗口右下 | 无实现 | 面板沿该轴翻转，落在指针上方/左侧并留在窗口内 |
| 面板外左键按下 | 无实现 | 关闭并归还键盘 |
| 第二次右键 | 无实现 | 面板移动到新指针处，不叠第二层 |
| 挂在固定卡片上右键 | 只有下半张卡片有反应 | 整张卡片都能右键（挂载层覆盖宿主，不参与其布局） |
| 区域内右键后按 Esc | 无实现 | 关闭；焦点仍在面板内时还给打开前的控件 |

## 实现要点

1. `ContextMenu` 的区域层捕获右键：`stop_propagation()`（嵌套区域不连父级一起打开）→ 写
   `PointAnchor` → 未打开时打开并把请求交给 `on_open_change`。
2. `Popover` 的定位器在 `prepaint` 里优先读指针锚点：
   `fluent_core::spec::menu::context_menu_origin(pointer, panel, viewport, margin)`；
   只有触发器时仍走原有的 `panel_origin`；两者都没有就不绘制。
3. 区域层用 `.absolute().top_0().left_0().right_0().bottom_0()` 铺满宿主，**不用 `size_full()`**。
4. 区域层不作为 `Popover` 的触发器——否则整片区域都算"按在触发器上"，外部按下永不关闭。

## 测出来的两个约束

- **`size_full()` 让区域层塌成 0 高**。第一版区域用 `div().size_full()`，右键完全没反应。
  诊断：给区域层加背景色后 `painted_quads()` 里找不到它；把区域层换成固定 `h(px(120.))/w(px(300.))`
  立刻出现面板。原因是在 flex 列里，百分比高度要对着一个仍在协商的高度解析。改成绝对定位铺满后稳定。
- **挂载层用 `relative().size_full()` 会和宿主的兄弟平分高度**。图库里菜单挂在一张 280×160 的卡片上，
  卡片里还有一个 `size_full()` 的居中提示；首次实测"右键没反应"，复现后发现只有卡片下半部能触发——
  两个 100% 高的兄弟在 flex 列里各拿一半。改成绝对定位覆盖宿主后整张卡片都能触发。
  回归测试直接压在卡片上半部：`a_right_click_inside_a_fixed_card_still_opens_the_menu`，把实现改回
  `relative().size_full()` 即失败。
- **区域层不能当触发器**。`Popover` 的关闭逻辑里，"按在触发器内"的按下会被跳过以免和 toggle
  重复；区域层是整片可点击区域，一当触发器，面板外的按下也被算成触发器，菜单关不掉。

## 命令结果

```text
cargo test -p fluent-core --lib spec::menu --locked          -> 13 passed
cargo test -p fluent-components --lib context_menu --locked -> 8 passed
cargo test -p fluent-components --lib menu:: --locked       -> 112 passed
cargo check -p fluent-gallery --locked                      -> exit 0
```

全套矩阵结果见文末"交付前矩阵"一节。

## 定向破坏矩阵

每条都把实现改坏一次，只跑目标测试，然后还原文件并刷新 `LastWriteTime` 强制重编。

| 破坏 | 失败的测试 | 失败信息（摘录） |
|---|---|---|
| `context_menu_origin` 不翻转（`if false`） | `a_right_click_near_the_window_edge_keeps_the_menu_on_screen` | 面板右边界越出窗口 |
| 定位器忽略指针锚点 | `a_right_click_opens_the_menu_at_the_pointer` | 面板原点不是按下位置 |
| 区域层不渲染（只剩 popover） | `a_press_outside_the_panel_closes_the_menu` | 外部按下后菜单仍然打开 |
| 打开时不抢焦点 | `escape_returns_the_keyboard_to_the_control_that_had_it` | 焦点仍停在区域层 |

## 未验证

- 原生窗口里的右键事件到 `MouseDown` 的映射，以及系统手势/触控板右键。
- Shift+F10 与 Apps 键唤起：尚未接线，键盘用户目前无法打开右键菜单（已记入
  [无障碍清单](../a11y-checklist.md) 缺口 10）。
- 高 DPI、多显示器、跨屏拖动时的翻转观感；测试只覆盖单窗口 600×400 的逻辑坐标。
- 真实屏幕阅读器对"右键菜单"的朗读：本版 `accesskit` 没有对应角色。

## 交付前矩阵

在 `main`（`0b0d1e5` + 本次改动）上跑完 [验证矩阵](../code-standards.md#验证矩阵)：

| 步骤 | 结果 |
|---|---|
| `cargo fmt --all --check` | exit 0（首次跑出 import 排序差异，`cargo fmt --all` 后复跑通过） |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0（仅工作区既有的 3 条 unused dependency 警告） |
| `cargo test --workspace --locked` | exit 0，409 passed / 1 ignored |
| `cargo doc --workspace --no-deps --locked` | exit 0 |
| `cargo deny check` | exit 0（`advisories ok, bans ok, licenses ok, sources ok`；需在工作区外写 advisory 锁，沙箱内必然失败） |
| `cargo xtask gen-tokens --check` / `gen-icons --check` | exit 0 / exit 0 |
| `FLUENT_GALLERY_SMOKE=1 cargo run -p fluent-gallery --locked` | exit 0（启动了新分区，未做人工点击） |

`test` 步的 409 = 上一版 397 + 右键菜单 8 条 + spec `context_menu_origin` 4 条。

冒烟只证明图库能构建并启动到首帧，不证明右键在真实窗口里的命中与定位；这一项仍列在"未验证"。

