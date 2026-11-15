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

## 审核修复（第二次）：受控状态与重定位回调

被测版本：`5633a2e` + 本次未提交改动。基线：`cargo test -p fluent-components --lib --locked` 为 250 项通过。

| 问题 | 根因 | 修改 |
|---|---|---|
| 页面拒绝首次打开后，再次右键不再上报 | 区域层用 `*open_store.read(cx)` 判断"是否已打开"；受控时 store 只是上一次请求的镜像，拒绝后它仍为 `true` | 改成 `self.open.unwrap_or_else(|| *open_store.read(cx))`：受控读属性，未受控读内部状态 |
| 页面主动关闭后无法重新打开 | 同上，store 停在 `true` | 同上 |
| 区域内再次右键的回调是 `[true, false, true]` | `Popover` 的外部按下判据在**捕获阶段**先关闭，区域层的右键在**冒泡阶段**后打开 | `Popover` 新增公开 builder `ignore_outside_press(filter)`；`ContextMenu` 用"右键 + 落在附着区域边界内"作为过滤器，同一次按下不再产生关闭回调，只更新锚点 |

区域边界由挂载层在 `on_children_prepainted` 里取第 0 个子元素（区域层）的窗口边界，每帧刷新。过滤器只影响关闭判据：区域内、面板外的**左键**仍关闭，区域外的按下仍关闭，面板与子菜单内的交互不受影响；不安装过滤器时其他调用方的行为不变。

`on_children_prepainted` 是 `Div` 的固有方法，必须在 `.id()` 之前调用——`.id()` 会把类型转成 `Stateful`，之后调用会报 `E0599`。

### 新增测试（5 条）

| 测试 | 断言 |
|---|---|
| `a_rejected_request_is_asked_again_on_the_next_press` | 拒绝后再右键得到 `[true, true]`，菜单始终关闭 |
| `a_menu_the_page_closed_can_be_asked_for_again` | 页面关闭后右键重新请求并真正打开，回调 `[true, true]` |
| `a_second_press_moves_the_menu_without_reporting_a_close` | 区域内重定位只上报 `[true]`，面板原点移到第二个按下点 |
| `a_controlled_move_reports_only_the_opening` | 受控重定位同样不产生多余的 `false`/`true` |
| `a_left_press_outside_the_attached_area_still_closes` | 区域外左键仍关闭并只上报一次 `false` |

### 定向破坏

| 破坏 | 失败测试 | 失败信息（摘录） |
|---|---|---|
| 区域层改回读内部状态 | `a_rejected_request_is_asked_again_on_the_next_press` / `a_menu_the_page_closed_can_be_asked_for_again` | `left: [true]`，缺第二次请求 |
| 过滤器恒为假 | `a_second_press_moves_the_menu_without_reporting_a_close` / `a_controlled_move_reports_only_the_opening` | `left: [true, false, true]` |

### 本轮命令结果

```text
cargo test -p fluent-components --lib context_menu --locked -> 13 passed
cargo test -p fluent-components --lib --locked              -> 255 passed
cargo test --workspace --locked                             -> exit 0，414 passed / 1 ignored
cargo doc --workspace --no-deps --locked                    -> exit 0
FLUENT_GALLERY_SMOKE=1 cargo run -p fluent-gallery --locked -> exit 0
cargo fmt --all --check                                     -> exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings -> exit 0
```

未验证项与上一节相同；新增的"重定位"路径同样只在测试窗口里验证，没有在原生窗口里用真实鼠标右键确认。

## 审核修复（第三次）：附着区域的可见范围

被测版本：`5633a2e` + 前两轮未提交改动 + 本轮改动。基线：`cargo test -p fluent-components --lib context_menu --locked` 为 13 项通过。

| 问题 | 根因 | 修改 |
|---|---|---|
| 被祖先裁剪掉的附着区域仍算“可重定位区域”，区域外右键关不掉菜单 | 过滤器用 `on_children_prepainted` 记下的**布局矩形**判断“是否在附着区域内”。祖先 `overflow_hidden` 或滚动容器裁掉的那一段，区域层收不到按下，过滤器却跳过关闭，于是既没关闭也没重定位 | 记录时与 `window.content_mask().bounds` 相交；命中判断用的正是这层裁剪，油漆子元素时它已在栈上，因此每帧拿到的就是可见范围。交集为空则整个区域不忽略任何按下 |

过滤器只在“右键 + 落在**可见**附着区域内”时为真；被裁掉的部分和区域外一样按外部按下处理。

### 新增测试（4 条，`crates/fluent-components/src/context_menu/tests.rs`）

夹具新增 `Clip::{None, Hidden, Scrolled}`：宿主 300×160 放进一个 300×80 的容器，`Hidden` 用 `overflow_hidden`，`Scrolled` 用 `overflow_y_scroll` + `ScrollHandle`（滚动用真实的 `PlatformInput::Scroll` 派发）。可见范围是 y 56..136，宿主布局矩形到 y 216。

| 测试 | 断言 |
|---|---|
| `a_press_in_the_clipped_away_part_of_the_region_closes_the_menu` | 在 (260,180)（布局矩形内、裁剪外）右键关闭菜单，回调 `[true, false]` |
| `a_press_in_the_visible_part_of_a_clipped_region_moves_the_menu` | 受控模式下在 (250,120) 右键只移动面板，回调 `[true]` |
| `a_scrolled_clip_drops_the_part_of_the_region_that_scrolled_away` | 打开后滚动 120px，(260,130) 右键关闭，回调 `[true, false]` |
| `a_visible_press_after_a_scroll_still_moves_the_menu` | 滚动后 (250,70) 仍在区域上，右键只移动，回调 `[true]` |

### 定向破坏

| 破坏 | 结果 |
|---|---|
| 记录时不做裁剪（回到布局矩形） | `a_press_in_the_clipped_away_part_of_the_region_closes_the_menu` 失败 |
| 边界只在第一次写入、之后不刷新 | `a_scrolled_clip_drops_the_part_of_the_region_that_scrolled_away` 失败 |

另外两条（可见部分移动、滚动后可见部分移动）在两种破坏下都通过，它们是防回归的守卫，不承担判别职责。

### 本轮命令结果

```text
cargo test -p fluent-components --lib context_menu --locked -> 17 passed
cargo test -p fluent-components --lib --locked              -> 259 passed（审核前 255）
cargo fmt --all --check                                     -> exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings -> exit 0
```

未验证：原生窗口中真实鼠标与真实滚动条的裁剪命中；高 DPI 下 `content_mask` 与布局矩形的取整差异；嵌套滚动容器（测试只覆盖一层裁剪）。

## 审核修复（第四次）：兄弟遮挡与真实命中

### 问题与结果

- 被测版本：`5633a2e848bc7da914fd04705b2bc3f1cc901c34` + 前三轮未提交修复 + 本轮改动；未提交。环境：Windows / AMD64，`rustc 1.98.0 (88d9e12ae 2026-08-18)`、`cargo 1.98.0 (797e8a9bc 2026-08-05)`。
- 原始路径：在未遮挡位置打开菜单，再在面板外、附着区域内但被上层 `occlude` 兄弟挡住的位置右键。旧过滤器只看布局矩形与裁剪交集，跳过关闭；区域处理器却收不到事件，菜单既不关闭也不移动。
- 修改后：仅在右键实际命中附着区域时跳过关闭。被兄弟挡住的位置右键会关闭菜单并上报一次 `false`；移除遮挡后，同一点恢复重定位，且不多报关闭/打开回调。
- `ContextRegion` 委托区域 `Stateful<Div>` 的元素 ID、布局、预绘制及绘制，在预绘制时发布该区域原有的 `Hitbox`。关闭过滤器和区域鼠标处理器使用同一个 `hitbox.is_hovered(window)` 判据，直接遵循 GPUI 对裁剪、滚动及兄弟遮挡的命中结果；每次渲染重新发布，不保留旧帧几何。
- `Popover::ignore_outside_press` 的过滤器现在接收事件和 `&Window`；仓库内唯一调用方为 `ContextMenu`，已同步。专题、接口注释和本地 T-044 验证入口已更新。README 的 ContextMenu 用法不变，无需修改；未改 GPUI vendor、定位 spec 或基线。

### 行为回归

`crates/fluent-components/src/context_menu/tests.rs` 新增两条真实 `PlatformInput` 回归，分别覆盖未受控与受控模式：

| 测试 | 断言 |
|---|---|
| `a_right_press_on_an_occluding_sibling_closes_the_menu` | 在 (60,100) 打开后，(280,180) 被兄弟遮挡且位于面板外；右键关闭，回调 `[true, false]`。重开并移除遮挡后，同一点只移动，累计回调 `[true, false, true]` |
| `a_right_press_on_an_occluding_sibling_closes_a_controlled_menu` | 页面接受开合请求的受控模式下执行同一路径，断言相同 |

红灯证据：先在旧几何过滤器上加入这两种遮挡场景，两条均在“被遮挡位置的右键应关闭”断言失败（`a blocked region cannot receive a move, so the press must close`）。修复后全部 19 条 ContextMenu 测试通过，其中包含此前的受控拒绝/外部关闭、重定位回调、祖先裁剪和滚动回归。

### 本轮命令结果

| 命令 | 结果与证据 |
|---|---|
| `cargo test -p fluent-components --lib context_menu --locked` | 通过，19 passed / 0 failed |
| `cargo fmt --all --check` | 通过，exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过，exit 0 |
| `cargo test --workspace --locked` | 通过，exit 0；包含全部组件回归；tooltip 的既有 doctest 为 ignored |
| `cargo doc --workspace --no-deps --locked` | 通过，exit 0 |
| `cargo deny check` | 通过，exit 0；有锁文件既存的 `warning[duplicate]`，本轮未改依赖。首次因沙箱内公告库锁文件只读而失败（`attempted to take an exclusive lock on a read-only path`），获准后以所需权限重跑原命令通过 |
| `cargo run -p xtask --locked -- gen-tokens --check` | 通过，生成物 up to date |
| `cargo run -p xtask --locked -- gen-icons --check` | 通过，生成物 up to date |
| `FLUENT_GALLERY_SMOKE=1 cargo run -p fluent-gallery --locked` | 通过，exit 0；仅验证原生窗口启动和自动退出，不证明右键交互 |
| `git diff --check` | 通过，exit 0 |

### 证据边界

以上交互来自 GPUI 测试窗口与模拟平台事件，未进行原生窗口真实鼠标/滚动条操作，也未新增截图或像素比对。高 DPI、多显示器、嵌套滚动及其他操作系统的交互仍未验证；Shift+F10 / Apps 键入口仍未接线。本轮结论不扩展到这些场景。

