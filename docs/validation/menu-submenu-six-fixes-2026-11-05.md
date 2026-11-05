# 菜单子菜单六项缺陷修复验证记录（2026-11-05）

> 后续第二轮边界修复（同 ID 相邻触发行、typeahead 重访、padding 悬停）见
> [menu-submenu-second-pass-2026-11-05.md](menu-submenu-second-pass-2026-11-05.md)；
> 本记录保留第一轮的基准、命令与未验证范围。

## 目标与范围

- 问题/关联记录：审查基准 `ea7e4e0762ad2477753fa529d4d5c2a5b6e4eb50`
  `feat(menu): submenus with per-level navigation and a hover delay`。
  审查在 `target/menu-review-ea7e4e0-repro.rs` 保留的 8 条复现中，6 条缺陷失败、2 条对照通过
  （完整旧输出：`target/menu-review-ea7e4e0-repro.log`）。
- 原始复现：Windows 10.0.26200 / x64，`cargo test` 测试窗口（600×500 逻辑像素），
  light 主题，直接派发 GPUI `PlatformInput` 的键盘/鼠标事件。复现路径分别是：
  子面板内点击、根层兄弟悬停后 Enter、三层菜单绘制、删除父行后 Esc、离开菜单后的延时悬停、
  叶子行按 → 后 Esc。
- 本次修改与明确不改的内容：
  - 修改：`Menu` 的整树状态归一化与持久化、逐层指针/键盘路由、子菜单逐层渲染与同帧定位、
    Popover 的内容区域关闭判据、父行 path hint 样式、typeahead 会话归属、下拉会话复位。
  - 不改：`MenuItem::submenu(Menu)` 的“子菜单是内容”方向；根容器单焦点模型；
    叶子项激活只执行回调、是否关闭由调用方决定的约定；`MenuDropdown` 的受控 `open` 语义；
    token/theme 分层与 GPUI 精确版本。
- 可观察行为（修改前 → 修改后）：
  - 点击已打开的子菜单叶子：修改前回调执行但同时 `on_open_change(false)`；修改后回调只执行一次，
    不请求关闭，也不穿透到背后控件。点击整条链外仍请求关闭，触发器仍只上报一次。
  - 打开子菜单后悬停根层兄弟再 Enter：修改前 Enter 仍指向父行/子层；修改后根层兄弟成为当前行，
    Enter 执行它。
  - 三层菜单：修改前第三层可被键盘激活但不绘制；修改后所有有效打开层都绘制，
    最深当前项、面板和回调一致。
  - 删除/禁用父行或移除 submenu：修改前只修正渲染快照，第一次 Esc 仍消耗在旧路径上；
    修改后真实状态被截断，Esc 按当前有效层处理。
  - 悬停延时后离开菜单：修改前延迟请求仍会打开旧分支；修改后离开整条链、移到不可选行、
    键盘接管、关闭层级或开始新会话都会取消/替换它。
  - 叶子行按 →：修改前产生不可见伪层级并吞掉 Esc；修改后不改变状态，根层 Esc 正常关闭。
  - 父行样式：父行由“当前项填充”改为独立 path hint 填充（`spec::menu::path_hint_background`），
    不再与最深层的键盘激活目标混淆。
- 调用方影响与兼容性：
  - `Menu` / `MenuItem` / `MenuDropdown` 的现有 builder 与回调签名不变。
  - `MenuItem::submenu` 现在支持任意层；此前“更深行只画箭头不打开”的限制取消。
  - `MenuItemSpec`（`fluent-core::spec::menu`）新增公开字段 `path_hint_background`。
    读取该结构的调用方不受影响；若外部以完整 struct literal 构造 `MenuItemSpec`，需要补这个字段。
  - 叶子项激活仍不自动关闭菜单；受控 `open` 仍完全由调用方决定。
  - `Popover::interior_regions` 为 `pub(crate)`，公共 API 不变；普通 Popover 的外部点击行为不变。
- 涉及的子系统与已核对源码/测试：
  [菜单协议](../subsystems/menus.md)、[浮层](../subsystems/overlays.md)、[焦点](../subsystems/focus.md)、
  [子系统导航](../subsystems/README.md)、[无障碍清单](../a11y-checklist.md)、
  [便签应用说明](../apps/notes.md)、中英文 README。
  源码：`crates/fluent-components/src/menu.rs`、`menu/submenu_tests.rs`、新增
  `menu/submenu_regression_tests.rs`、`crates/fluent-components/src/popover.rs`、
  `popover/dismissal_tests.rs`、`crates/fluent-core/src/spec/menu.rs`。
- 同类实现/下游检查：
  - 普通 `Popover` 仍只按自身 hitbox 判外部按下；额外区域是 opt-in 的 crate 内部接线，
    未改变无内容区域的 Popover。
  - 单独使用的 `MenuItem` 仍保留局部 hover 样式（无上层菜单）。
  - Gallery/Notes 未使用子菜单；Notes 文档改为“组件支持、应用尚未接入”。
  - Sheet/Dialog/Tabs 不共享菜单状态或 Popover 关闭判据，未改动。

## 六个问题的复现、原因、修复与测试

| # | 复现（旧实现） | 原因 | 修复 | 永久测试 |
|---|---|---|---|---|
| 1 | 子菜单叶子中心按下/释放：`actions: ["child-one"]`，同时 `on_open_change: [false]` | Popover 的外部按下按面板自身 hitbox 判断；子面板超出该矩形，虽然绘制在上方仍被判为外部 | 子面板在 prepaint 发布窗口边界到 `InteriorRegions`；`MenuDropdown` 把同一集合交给 `Popover`，外部按下先检查区域；子面板加 `occlude()` 防穿透 | `clicking_a_submenu_child_runs_once_without_a_close_or_fall_through`；Popover `a_press_inside_a_published_content_region_keeps_the_panel_open`、`a_region_not_published_again_is_treated_as_outside`、`a_press_outside_panel_and_regions_still_requests_a_close` |
| 2 | 打开 More 子菜单后悬停根层 Two，再 Enter：`[]`；根层当前项仍是 More | 所有指针回调共用 `set_active`，只写 `open.last_mut()`（最深层），随后清空 open | 指针回调携带完整层路径；写入该层当前行，按协议截断更深分支；锚点行保留分支；键盘始终取最深有效层 | `hovering_a_root_sibling_makes_enter_run_that_sibling`、`switching_between_sibling_submenus_moves_the_path_and_the_target`；绘制断言检查 path hint 与唯一当前项 |
| 3 | 三层中按 → 进入第三层再 Enter：`deep` 回调执行，但第三层面板数量为 0 | 渲染只取 `open.first()` 渲染一层 | 按 open 路径递归渲染所有层；每层面板/行/当前行独立；每层带子菜单的行都可继续打开；同帧定位 | `three_levels_paint_and_the_deepest_row_is_the_target`；断言三层绘制、两行 path hint、最深层当前项与回调一致 |
| 4 | 打开子菜单后删除父行，第一次 Esc 不关闭 (`None`) | `normalize_state` 只改渲染副本；事件仍读旧的 open 路径 | 归一化在任何读取前执行并写回真实 keyed state；仅在变化时写；所有动作/指针/typeahead 用同一归一化协议 | `removing_or_losing_the_parent_row_truncates_the_real_state`、`disabling_an_inner_parent_truncates_only_the_levels_below_it`；首帧绘制断言 |
| 5 | 悬停 More 150ms、移出菜单、推进 400ms：子菜单仍打开，Enter 执行 `child-one` | 待打开请求没有被离开事件清除 | `PendingOpen` 带完整层路径；离开整条链、移到不可选行、键盘接管、关闭层级/新会话都取消；`leave_chain` 用已绘制面板边界判断“仍在链内” | `leaving_the_chain_or_reaching_a_non_selectable_row_cancels_the_pending_open`（离开 + 标题/分隔线/禁用行）、`keyboard_takeover_cancels_a_pending_hover`、`reopening_does_not_revive_the_previous_hover_request` |
| 6 | 根层叶子按 → 再 Esc：`None`，菜单不关 | `open_current` 无条件 push 一个没有 submenu 的层 | 只有当前行存在、可用且带 submenu 时才打开；归一化也会截断没有子菜单的父行 | `right_on_a_leaf_creates_no_level_and_escape_still_closes` |

额外覆盖：首次打开/切分支/窗口边缘/布局变化后的首帧定位
（`three_levels_paint...`、`switching_between...`、`a_submenu_flips_left_on_its_first_frame_at_the_window_edge`、
`a_layout_change_repositions_the_open_submenu_on_the_next_frame`）；父行移入子面板不误关分支
（`moving_from_the_parent_row_into_the_submenu_keeps_the_branch`）；跨层复用同一行 ID
（`the_same_row_id_at_two_levels_keeps_the_paths_distinct`）；Esc/← 逐层与 Tab/Shift+Tab 整链
（`escape_and_left_step_one_level_back_while_tab_closes_the_chain`）；typeahead 换分支/离开再返回
（`typeahead_does_not_inherit_a_prefix_across_branches`、`..._after_leaving_and_returning`）；
子菜单的 header/separator/custom 内容与 `Menu::style` 应用
（`submenu_content_and_style_are_preserved`）。

## 验证记录

- 被测提交 SHA：工作区基于 `ea7e4e0762ad2477753fa529d4d5c2a5b6e4eb50`；
  **本次未提交**。未提交修改范围见文末“工作区状态”。
- 环境：Windows 10.0.26200 / x64；rustc 1.95.0 (59807616e 2026-04-14)；
  cargo 1.95.0；cargo-deny 0.20.2。自动测试运行于 GPUI test-support 窗口；
  未进行原生窗口截图/像素比较。

| 命令或原生操作 | 最终结果（通过/失败/未运行/不适用） | 输出/日志/证据与理由 |
|---|---|---|
| `cargo fmt --all --check` | 通过 | `target/menu-validation-fmt.log`（无输出） |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 | `target/menu-validation-clippy.log`，`Finished dev profile` |
| `cargo test -p fluent-components --lib menu:: --locked` | 通过 | 65 passed（46 条原有 + 19 条新回归）；`target/menu-validation-menu.log` |
| `cargo test -p fluent-components --lib popover:: --locked` | 通过 | 22 passed；`target/menu-validation-popover.log` |
| `cargo test -p fluent-core --lib spec::menu --locked` | 通过 | 5 passed；`target/menu-validation-spec.log` |
| `cargo test --workspace --locked` | 通过 | 144 components lib + 88 core + 2 icons + 33 notes + 6 theme + 9 tokens + 1 integration + 3 doctests（1 ignored）；`target/menu-validation-test.log` |
| `cargo doc --workspace --no-deps --locked` | 通过 | `target/menu-validation-doc.log`；生成 `target/doc/fluent_components/index.html` |
| `cargo deny --offline check` | 通过 | `target/menu-validation-deny.log`：`advisories ok, bans ok, licenses ok, sources ok`。使用 2026-11-05 21:42（本地）更新的缓存告警库（commit `26fd2d4`）。首次在线 `cargo deny check` 于 21:42 通过；22:03 再次在线刷新时 GitHub 不可达，失败日志保留在 `target/menu-validation-deny-online-failure.log`。因此结果只代表该时间点的公告数据，不是最新在线审计 |
| `cargo xtask gen-tokens --check` | 通过 | `crates/fluent-tokens/src/generated.rs is up to date`；`target/menu-validation-gen-tokens.log` |
| `cargo xtask gen-icons --check` | 通过 | `crates/fluent-icons/src/generated.rs is up to date`；`target/menu-validation-gen-icons.log` |
| 旧复现在修复后重跑（8 条） | 通过 | `target/menu-review-ea7e4e0-after.log`：8 passed（旧实现为 6 failed，见 `target/menu-review-ea7e4e0-repro.log`） |
| 真实原生窗口：父子面板间移动、三层展开、逐层关闭、内部/外部点击、边缘定位 | 未运行 | 本会话没有可交互/可截图的原生窗口输入与捕获链路；自动测试直接派发平台事件并断言场景 quad，不能替代真实窗口。未用相似样例冒充 |

### 回归测试在定向破坏后的结果

每条定向破坏只改一处，运行对应测试，确认失败后全部还原；还原后的最终代码通过全部检查。

| 定向破坏 | 运行测试 | 结果与证据 |
|---|---|---|
| 删除 Popover 的 `inside_content` 早返回 | `clicking_a_submenu_child_...` | 失败：`an inside press must not request a close: [false]`；`target/menu-break-1-click-inside.log` |
| 指针改为先写最深层再截断（模拟旧 `set_active`） | `hovering_a_root_sibling_...` | 失败：`the pointer-selected row is the sibling, not the old parent`；`target/menu-break-2-pointer-level.log` |
| 子菜单递归限制为只渲染一层 | `three_levels_paint_...` | 失败：面板数 `left: 2 right: 3`；`target/menu-break-3-third-level.log` |
| 归一化只在渲染副本进行，Esc 读旧 open | `removing_or_losing_...` | 失败：`Escape ... left: None right: Some(false)`；`target/menu-break-4-normalize-snapshot.log` |
| 离开整条链时不清 pending | `leaving_the_chain_or_reaching_...` | 失败：`leaving must cancel the hover: left: 2 right: 1`；`target/menu-break-5-pending-leave.log` |
| 键盘导航不清 pending | `keyboard_takeover_cancels_...` | 失败：`keyboard navigation must replace the pending hover: left: 2 right: 1`；`target/menu-break-5b-pending-keyboard.log` |
| 去掉 `has_submenu` 打开条件并放宽归一化 | `right_on_a_leaf_creates_no_level_...` | 失败：Esc 后 `left: None right: Some(false)`；`target/menu-break-6-leaf-right.log` |

### 首帧定位的验证说明

子面板位置器在 `prepaint` 内取得自身布局尺寸，并在同一帧读取锚点行的测量边界；
测试在每次按键后只绘制一帧就断言面板数量与几何，未用“再补两帧”掩盖首帧错误。
窗口边缘测试让菜单靠右，断言首帧即左翻并与行左边缘对齐；布局变化测试插入 header 后只绘制一帧，
断言子面板跟随锚点行下移。旧实现的两帧推定定位已移除。

## 红灯、未验证项与后续

- 红灯：`cargo deny check` 首次因沙箱只读 `~/.cargo/advisory-dbs/db.lock` 失败；
  以一次性提权重跑通过，未修改项目配置或依赖。随后再次在线刷新公告库时 GitHub 不可达
  （`target/menu-validation-deny-online-failure.log`），改用 `cargo deny --offline check`
  对 21:42 的缓存库检查并通过；这不等于最新在线公告数据。其余最终命令均为通过。
- 未验证：
  - 真实原生窗口中的子面板命中顺序、父子面板间指针移动、DPI 缩放、GPU 字体与长内容；
    本会话无原生窗口自动输入/截图链路，未做人工操作。
  - 屏幕阅读器对多面板 `role=menu`、单焦点范围与 `aria_selected` 的实际播报。
  - accesskit 禁用态：本版 GPUI 无 API，禁用行仍与普通行同语义。
  - 每层独立 focus scope / 嵌套焦点陷阱：明确不在本次范围。
  - 依赖审计只覆盖 2026-11-05 21:42 的缓存公告库；此后在线刷新失败，未证明公告数据仍是最新。
  - 大量层级/长菜单的每帧测量与区域发布成本，未做性能剖析。
  - 普通 Popover 的既有未验证项（多窗口、焦点归还边界等）仍按原文档保留。
- 后续负责人：菜单/浮层维护者；若需要原生验证，应在具备 Windows 原生自动化输入的会话中
  补父子面板移动、三层展开/逐层关闭、内外点击和右缘翻转的截图与操作记录。

## 交付自查

- [x] 已按验证矩阵区分 spec、GPUI 行为与原生视觉证据；原生窗口项明确标为未运行。
- [x] 已沿实际运行路径检查渲染、键盘、指针、离开与延迟打开；状态归属/绘制阶段/公开字段变化已同步
      `menus.md`、`overlays.md`、`focus.md`、无障碍清单、Notes 说明与中英文 README。
- [x] 已同步中英文 README、公开 API 文档与长期限制；验证记录在版本控制内。
- [x] 记录了最终代码版本（基准 SHA + 未提交差异）与所有执行结果；取消/未运行项未计为通过。
- [x] 无视觉基线更新（Path hint 是新颜色，但当前像素夹具不含子菜单/父行展开态；未重录或扩大容差）。

## 工作区状态（交付时）

基准 `ea7e4e0762ad2477753fa529d4d5c2a5b6e4eb50`，**未创建提交**。修改/新增：

```text
 M README.md
 M README.zh-CN.md
 M crates/fluent-components/src/menu.rs
 M crates/fluent-components/src/menu/submenu_tests.rs
 M crates/fluent-components/src/popover.rs
 M crates/fluent-components/src/popover/dismissal_tests.rs
 M crates/fluent-core/src/spec/menu.rs
 M docs/a11y-checklist.md
 M docs/apps/notes.md
 M docs/subsystems/focus.md
 M docs/subsystems/menus.md
 M docs/subsystems/overlays.md
?? crates/fluent-components/src/menu/submenu_regression_tests.rs
?? docs/validation/menu-submenu-six-fixes-2026-11-05.md
```

`.mnemon/` 为本会话外部的未跟踪目录，未修改、未纳入本任务。
