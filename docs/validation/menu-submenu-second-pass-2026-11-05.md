# 菜单子菜单第二轮边界修复验证记录（2026-11-05）

## 目标与范围

- 问题/关联记录：第二轮审查，基准 `ea7e4e0762ad2477753fa529d4d5c2a5b6e4eb50`（当前 HEAD 未前进）。
  本记录只覆盖第二轮新增的三个边界问题；第一轮六项修复仍保留在工作区，记录见
  [menu-submenu-six-fixes-2026-11-05.md](menu-submenu-six-fixes-2026-11-05.md)。
- 原始复现：`target/menu-second-review-repro.rs` / `target/menu-second-review-repro.log`。
  4 条复现中 3 条失败、1 条对照通过：
  - `same_id_on_consecutive_openers_does_not_block_keyboard`：面板数 2，期望 3；
  - `typeahead_restarts_after_leaving_and_reentering_same_branch`：执行 Beta，期望 Alpha；
  - `moving_to_panel_padding_cancels_the_row_hover`：面板数 2，期望 1；
  - `control_three_distinct_levels_are_visible`：通过（对照）。
- 本次修改与明确不改的内容：
  - 修改：`open_below` 的身份判据、`MenuState.level_epoch` 与 typeahead 归属、面板级实际
    MouseMove/离开时的 pending 取消。
  - 不改：`MenuItem::submenu(Menu)` 数据树方向、根容器单焦点、父菜单统一管理整棵树、
    叶子项激活由调用方决定关闭、受控 `open`、Popover 的定位/焦点行为。
- 可观察行为（修改前 → 修改后）：
  - 相邻两层触发行同为 `more`：修改前第二次 →/Enter 被裸 ID 比较挡住，只画两层；
    修改后按完整路径继续展开第三层，最深层叶子执行一次。
  - 子菜单输入 `b` 后左退/右进（或 Esc 后右进）同一分支再输入 `a`：修改前拼成 `ba` 并停在 Beta；
    修改后新访问从 `a` 重新匹配 Alpha；同一层内连续输入仍正常累积。
  - 悬停触发行 150ms 后移到同一面板的 padding/gap/自定义内容：修改前旧请求仍到期打开子菜单；
    修改后实际 MouseMove 离开触发行即取消，不绘制子菜单。
- 调用方影响与兼容性：
  - 三个公开类型 `Menu` / `MenuItem` / `MenuDropdown` 的 builder、回调与 `open` 语义不变。
  - 本轮没有新增公开 API；`MenuState`、`Typeahead`、`level_epoch` 都是 crate 内部状态。
  - 行为变化只影响此前错误的边界：同名相邻触发行现在可继续打开；同分支重访的 typeahead
    不再继承；padding/gap/custom 上的指针移动会取消待打开请求。
  - 第一轮的 path hint 填充、跨层同 ID 叶子、受控关闭与点击归属均保留。

## 三个问题的原因、修复与永久测试

| # | 复现（修复前） | 原因 | 修复 | 永久测试 |
|---|---|---|---|---|
| 1 | 相邻两层触发行 ID 都为 `more`；`↓ → →` 后面板仍为 2，第三层不画 | `open_below` 用 `state.open.last().parent == row` 比较裸 ID；这是父层与子层的两个不同行，允许同名 | 打开判据改为“`path.len() == open.len()` + 完整路径身份”；移除跨层裸 ID 比较。右键、Enter、悬停共用同一 `open_below` | `identical_opener_ids_on_adjacent_levels_do_not_block_keyboard_descent`、`identical_opener_ids_do_not_block_enter_descent`、`identical_opener_ids_do_not_block_hover_descent`；断言三层面板与 `deep` 只执行一次 |
| 2 | 子菜单 Beta/Alpha：`b` → `←` → `→` → `a` → Enter 执行 Beta | typeahead 只在输入字符时比较最终 path/session；退层再进同一分支后两者都没变，旧 `b` 被拼成 `ba` | `MenuState.level_epoch` 在 open 路径实际增删/截断时递增；Typeahead 归属 path + session + epoch。Esc/Tab 在根层不改变 open 路径，因此显式清空前缀。只在输入时比较，不每帧清空 | `typeahead_restarts_after_left_and_right_reenter_the_same_branch`、`typeahead_restarts_after_escape_reenters_the_same_branch`、`typeahead_still_builds_a_prefix_inside_one_visit` |
| 3 | 悬停 More 150ms 后移到面板顶部 padding，再等 400ms，子菜单打开 | padding/gap/custom 没有行回调，也未离开整个面板，旧 pending 无人生效取消 | 每个面板在**真实 `on_mouse_move`** 中用本层当前帧测得的待打开行边界核对指针；不在行内则清除 pending。面板 `on_hover(false)` 只取消属于该面板路径的 pending；离开整链仍由边界集合判断 | `moving_to_panel_padding_cancels_the_pending_open`、`moving_to_a_row_gap_cancels_the_pending_open`、`moving_to_custom_content_cancels_the_pending_open`、`resting_on_the_opener_row_still_opens_after_the_delay`、`a_padding_move_does_not_clear_a_keyboard_selected_row` |

保留并回归通过的既有测试包括：跨层同 ID 叶子激活、换分支/离开再返回/新会话 typeahead、
从父行进入已打开子面板不误关分支、点击归属、首帧定位与受控关闭。

## 修复前/后证据

- 新增永久测试在**修复前**运行：`target/menu-second-before-fix.log`
  - 30 项中 7 项失败、23 项通过；失败正是上表三组：
    `identical_opener_ids_*` 2 条、`typeahead_restarts_*` 2 条、`moving_to_*` 3 条。
- 修复后同一套件：`target/menu-second-after-fix.log`
  - 30 passed, 0 failed。
- 第二轮审查原始复现修复后：`target/menu-second-review-repro-after.log`
  - 4 passed（修复前 3 failed / 1 passed，见 `target/menu-second-review-repro.log`）。
- 第一轮原始复现仍通过：`target/menu-review-ea7e4e0-after.log`（8 passed）。

## 验证记录

- 被测提交：HEAD `ea7e4e0762ad2477753fa529d4d5c2a5b6e4eb50`；**未提交**。
  工作区包含第一轮与本轮未提交修改，范围见文末。
- 环境：Windows 10.0.26200 / x64；rustc 1.95.0 (59807616e 2026-04-14)；cargo 1.95.0；
  cargo-deny 0.20.2。自动测试运行于 GPUI test-support 窗口；未做原生窗口截图/像素比较。

| 命令或原生操作 | 最终结果（通过/失败/未运行/不适用） | 输出/日志/证据与理由 |
|---|---|---|
| `cargo fmt --all --check` | 通过 | `target/menu-second-review-fmt.log`（无输出） |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 | `target/menu-second-review-clippy.log` |
| `cargo test -p fluent-components --lib menu:: --locked` | 通过 | 76 passed（46 条原有 + 30 条回归）；`target/menu-second-review-menu.log` |
| `cargo test -p fluent-components --lib popover:: --locked` | 通过 | 22 passed；`target/menu-second-review-popover.log` |
| `cargo test -p fluent-core --lib spec::menu --locked` | 通过 | 5 passed；`target/menu-second-review-spec.log` |
| `cargo test --workspace --locked` | 通过 | 155 components lib + 88 core + 2 icons + 33 notes + 6 theme + 9 tokens + 1 integration + 3 doctests（1 ignored）；`target/menu-second-review-test.log` |
| `cargo doc --workspace --no-deps --locked` | 通过 | `target/menu-second-review-doc.log` |
| `cargo xtask gen-tokens --check` / `gen-icons --check` | 通过 | `target/menu-second-review-gen-tokens.log`、`target/menu-second-review-gen-icons.log` |
| `cargo deny --offline check` | 通过 | `target/menu-second-review-deny.log`：`advisories ok, bans ok, licenses ok, sources ok`。使用 2026-11-05 21:42（本地）缓存公告库（commit `26fd2d4`）；在线刷新曾因 GitHub 不可达失败（`target/menu-validation-deny-online-failure.log`），因此不代表最新公告数据 |
| 真实原生窗口：同名相邻触发行展开、退层重进 typeahead、padding/gap 取消悬停 | 未运行 | 本会话没有原生窗口自动输入/截图链路；自动测试直接派发平台事件并断言场景 quad，不能替代真实窗口 |

## 未验证项与后续

- 真实原生窗口中的指针移动、DPI 缩放、GPU 字体、面板命中顺序与长内容未做人工检查。
- 每层独立 focus scope、`aria-activedescendant`、accesskit 禁用态仍不在范围。
- `level_epoch` 只是访问代号，不是持久化协议；如果未来允许调用方直接注入 open 路径，
  需要同时定义 epoch 的推进规则。
- 大量层级/大菜单的每帧边界检查成本未做性能测量。
- 依赖审计只覆盖上述缓存公告库时间点，不代表最新在线数据。

## 工作区状态（交付时）

基准 `ea7e4e0762ad2477753fa529d4d5c2a5b6e4eb50`，**未创建提交**。修改/新增：

```text
 M .gitignore
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
?? docs/validation/menu-submenu-second-pass-2026-11-05.md
```

`.mnemon/` 已加入 `.gitignore`，未修改其内容。
