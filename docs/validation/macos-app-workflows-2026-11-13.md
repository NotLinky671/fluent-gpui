# macOS 应用/说明工作流修复（2026-11-13）

## 目标与范围

- 问题/关联记录：macOS 真机验收中的 B6（Gallery 两组默认 `TabList` 缺 `track_focus`，Tab 不可达）、D1（Notes 正文不在“标题 → 正文 → 操作按钮”的 Tab 路径）、B3（字体回退与验收预期）、C1（列表读取已保存 Note 的契约与错误注释）。组件焦点陷阱/CJK、拖选与 VoiceOver 由并行工作流负责。
- 原始复现：用户在 macOS 真机按 33 项验收清单观察。原始总数 24 通过 / 7 失败 / 2 未测；C2/C3/C4 中文输入复测通过；F2/F3 未测。本节只保留修复前原始结果，不是修复后结论；未擅自改分。
- 基准提交：`aa1cf6c0243e2f669a6004eef24da9db390574a6`。独立 worktree `/private/tmp/fluent-gpui-native-app-repairs`（detached）；未创建提交。
- 工作区差异范围：`Cargo.toml`（仅 `[patch.crates-io]` 路径补丁）、`vendor/gpui-base`、`vendor/gpui-pre`、`vendor/VENDORED.md`、`crates/fluent-gallery/src/main.rs`、`src/fixtures.rs`、`examples/titlebar_probe.rs`、`crates/fluent-gallery/Cargo.toml`（仅 test-support dev-dependency）、`crates/fluent-notes/src/app.rs`、`src/main.rs`（仅环境变量触发的 AX 抓取钩子）、`app/tests.rs`、`crates/fluent-components/src/button.rs`（仅 `accessibility_label` 文档注释，不改行为）、`README.md`、`README.zh-CN.md`、`docs/apps/notes.md`、本记录。未升级依赖、未改组件/theme/core 逻辑、未改生成物。
- 后续集成阶段（本记录之后追加）：应用所有文字/图标 `Button` 已补显式 `accessibility_label`（可见文字同名，图标按钮写动作）；
  `Button::accessibility_label` 的文档改为“无自动回退”，与实现一致。真实 AX 树抓取入口为
  `FLUENT_NOTES_A11Y_DUMP`，需要先由 VoiceOver / Accessibility Inspector 激活平台适配器；本轮未运行（Cargo 由 core 占用）。
- 依赖补丁准备（用户授权后由本工作流机械执行）：`vendor/gpui-base` 与 `vendor/gpui-pre` 从本机 registry 原样复制
  `0.8.1` / `0.3.10` 源码，保留 `Cargo.toml`、`Cargo.toml.orig`、`LICENSE-APACHE`、README 与
  `gpui-base/.cargo_vcs_info.json`，未升级版本、未改依赖；排除仅编译库不需要的 `tests/`、`benches/`、`examples/`、
  crate 内 `Cargo.lock`。`Cargo.toml` 增加 `[patch.crates-io]` 路径补丁，上游 `.crate` SHA-256 与 lock 校验和记录在
  `vendor/VENDORED.md`。`Cargo.lock` 未手工编辑，待获得 Cargo 授权后由 Cargo 验证/更新。
  实测复制一致：`diff -r`（排除上述目录）无差异。
- 明确不改：自动保存、数据模型；组件 API 由 core 提供（Notes 只调用 `TextareaTabBehavior::Navigate`）。Notes 不保留窗口级 `intercept_keystrokes` 症状补丁。
- 可观察行为（修改前 → 修改后）：
  - B6：默认 Gallery 的横向/纵向标签条没有属于自己的焦点句柄，不参与 Tab 顺序 → 各持持久 `FocusHandle` 并 `track_focus`，Tab/Shift+Tab 可停在两条 strip 上，方向键/Home/End 经消费者回调改选中；像素 fixture 的两条 strip 也用 keyed 焦点句柄跨重绘保持 stop。
  - D1：布局与视觉顺序改为 标题 → 正文 → 操作栏 → 状态栏（此前 标题 → 操作栏 → 正文），Tab 从标题先到正文。正文现已显式使用 `TextareaTabBehavior::Navigate`（组件默认仍为 `Indent`），Tab/Shift+Tab 移出字段而不改正文。该调用依赖 core 组件补丁合并后才能编译；合并前测试保持启用（不 `#[ignore]`），本记录不声称已转绿。
  - C1：列表标题/预览/字数只读已保存 `Note`；编辑区字数读草稿；输入不更新列表，保存才写回。仅修注释与文档，数据模型未变。
  - B3：文档明确 `SegoeCompat` 依次查找本机 `Segoe UI` / `Selawik`，都缺失时回退 `.SystemUIFont`；仓库未内置字体，macOS 上切换 Font mode 可能没有像素变化。
- 子系统与已核对源码：[焦点](../subsystems/focus.md)、[Tabs](../validation/tabs-2026-11-04.md)、[notes 应用说明](../apps/notes.md)、gpui-base `input/base/state.rs:196-197`。D1 需要组件层契约变化，本 patch 没有改组件机制，因此不写“子系统机制未变即完成”；待 core API 落地后同步 `focus.md`/组件说明。

## 验证记录

- 被测对象：`aa1cf6c` 上的未提交 worktree 修改；无提交、无推送。
- 环境：macOS（darwin 27.0.0，arm64）；工具链按 `rust-toolchain.toml`（Rust 1.98.0）。`CARGO_TARGET_DIR=/Users/linky/Code/rust/fluent-gpui/target` 曾在独占窗口使用，现由 core 排队使用；本轮末尾未再运行 Cargo。
- 证据完整性声明：**首失败完整日志已丢失**。第一次 `cargo test -p fluent-notes --locked` 的 `tee` 文件在第二次全量运行时被覆盖；`notes-test-first-failure.log` 只保留了当时展示的 `tail -40`（含全部失败用例与计数），不是完整输出。`notes-test-second-failure.log` 是第二次全量失败输出（含应用拦截器诊断态）。不得宣称完整保留首失败。

| 命令或原生操作 | 最终结果 | 输出/日志/证据与理由 |
|---|---|---|
| `cargo test -p fluent-notes --locked`（首次，含临时布局改动） | 失败：31 通过 / 2 失败 | `notes-test-first-failure.log`（仅保留尾部，证据不完整）。失败暴露正文吞 Tab/Shift+Tab。 |
| `cargo test -p fluent-notes --locked`（第二次，含临时 intercept） | 失败：31 通过 / 2 失败 | `notes-test-second-failure.log`；测试断言与种子内容/激活时机问题，随后修正。 |
| 临时 `intercept_keystrokes` 期间的两个定向 filter | 通过后被否决 | 日志未单独留存；该路径已按评审删除，不作为交付证据。 |
| `cargo test -p fluent-gallery --locked --features gpui/test-support default_tab_strips_are_stops_and_own_arrow_and_home_end` | 通过 | `gallery-tabs-targeted-pass.log`；默认 Gallery（非 fixture）从 strip 前一个真实 stop Tab 进入横向 strip，Right 跳过禁用项、Home/End 归边，Tab 到纵向 strip、Down 改选中，再 Tab 离开、Shift+Tab 逐级返回。 |
| 定向破坏：删除横向 strip 的 `track_focus` 后重跑 | 失败 | 在 “Shift+Tab from the horizontal strip must land outside the tab strips” 失败；随后恢复。该次输出未落盘，仅为定向证据。 |
| Gallery manifest 增加 `[dev-dependencies] gpui = { workspace = true, features = ["test-support"] }` | 已修改，未运行 | 消除默认 `cargo test --workspace` 缺 test-support 的编译风险；按现有 notes/components 惯例，未新增 `gpui_platform` dev-dep，无版本变更。未验证 Cargo.lock（feature 不改变包图，但本轮未运行 Cargo）。 |
| `rustfmt --edition 2024`（本次修改的 Rust 文件） | 通过 | 未执行 `cargo fmt --all --check`，留到整合后。 |
| workspace clippy/test/doc/deny/xtask、截图、修复后真机、VoiceOver | 未运行 | core 独占 target；未运行项不得写成通过。 |

- 回归测试在旧实现或定向破坏后的结果：B6 已用删除 `track_focus` 证明失败；D1 的正向/反向测试在导入 core 的 `Textarea` tab-navigation 并显式使用 `Navigate` 后转为通过（日志见下节）；输入 AX 的默认角色用定向破坏证明会失败。
- 红灯原因与追踪：gpui-base `0.8.1` 的 Tab/Shift+Tab 绑定曾导致 Textarea 吞键。core 已提供 `TextareaTabBehavior::{Indent, Navigate}` 与 `Textarea::tab_behavior`（默认 `Indent` 保持兼容）；Notes 显式使用 `Navigate`，全局 intercept 已删除。导入 D5 补丁后两个 Tab 过滤均通过，测试未 ignore。
- 未验证项与后续负责人：
  - B6/D1 修复后 macOS 真机键盘路径与约 520 逻辑像素窄窗口视觉未复测。
  - B3 仅文档修正，未做 Font mode 切换真机断言，未内置字体。
  - Gallery fixture 截图/像素基线未跑；VoiceOver/D5/C5/CJK 由并行工作流负责，本记录不声明通过。

## 集成阶段结果（vendor/输入 AX）

- 已导入 core 补丁：`/tmp/fluent-gpui-aa1cf6c-d5-textarea-final.patch`（组件/fluent-ui/专题，无冲突）与
  `/tmp/textarea-a11y-incremental.patch`（仅 `textarea.rs`，`-p3` 应用）。未改 core 的 worktree。
- vendor 本地钩子：`vendor/gpui-base/src/input/base/state.rs` 增加只读 getter
  `InputState::accessibility()`；`vendor/gpui-pre` 测试平台保存真实 `a11y_init` 回调并新增
  `TestAppContext::enable_accessibility()` / `TestWindow::activate_accessibility()`。生产代码没有 AX dump
  环境变量或轮询钩子。
- `TextInput::accessibility_label` 与 `Textarea::accessibility_label` 已接线：无投影时安装默认
  `TextInput` / `MultilineTextInput` 角色；已有投影只补缺失角色；builder 名称覆盖状态名称，但保留
  role/placeholder/accessibility id/value_exposed；仅在合并结果变化时写入。Notes 标题/正文、Gallery 全部输入
  与 Textarea 使用显式名称。
- 本轮窄验证（基准 `aa1cf6c` + 上述未提交改动；`CARGO_TARGET_DIR=/Users/linky/Code/rust/fluent-gpui/target`）：
  Cargo.lock 因 path patch 与 `serde_json` dev-dep 由 Cargo 无 `--locked` 生成一次，之后全部 `--locked`。

| 命令 | 结果 | 日志 |
|---|---|---|
| `cargo test -p fluent-components --lib input::accessibility_tests --locked` | 3 通过 | `/tmp/fluent-gpui-logs-integration/input-ax-final.log` |
| `cargo test -p fluent-notes tab_moves_focus_forward_and_back --locked` | 1 通过 | `/tmp/fluent-gpui-logs-integration/notes-tab-forward-final.log` |
| `cargo test -p fluent-notes tab_from_body_reaches_save_and_back --locked` | 1 通过 | `/tmp/fluent-gpui-logs-integration/notes-tab-body-final.log` |
| `cargo test -p fluent-gallery default_tab_strips_are_stops_and_own_arrow_and_home_end --locked` | 1 通过 | `/tmp/fluent-gpui-logs-integration/gallery-default-tabs-final.log` |
| 定向破坏：去掉已有投影的默认角色填充后重跑 AX 回归 | 失败（AX 树只剩 Window 节点） | `/tmp/fluent-gpui-logs-integration/input-ax-break-rolefill.log` |

- AX 测试读取 `Window::debug_a11y_tree_json` 的真实发布树，不读组件 getter：无配置无 builder 仍发布
  `TextInput` 角色；状态已有名称/placeholder/value_exposed=false 时跨两帧保持；builder 名称只覆盖名称。
- 尚未运行：workspace 全套 fmt/clippy/test/doc/deny/xtask、真实窗口 AX/VoiceOver、Dialog/inert vendor 补丁、
  core 的 C5/CJK 增量。本轮不以窄过滤代替最终矩阵。

### Dialog / Sheet / Menu AX 轮（结果）

- 实现：gpui-pre `aria_modal`/`aria_hidden`（隐藏节点及子树不发布，焦点/active descendant 忽略隐藏祖先）、
  topmost modal 子树保留、deferred 覆盖层 owner 继承（按调度时当前节点归属，不用角色猜归属）、debug JSON
  输出 `modal`/`hidden`；gpui-base `Dialog`/`AlertDialog` 的标题/名称、`aria_modal`（默认 true）和唯一宿主 id
  （Fluent caller id，避免两个 Dialog 都落到 `dialog-0`）；Fluent `Dialog`/`AlertDialog` 标题文本自动成为
  host 名称，`accessibility_label` 仅覆盖；Sheet 同样以标题为名并 `aria_modal(true)`。
- 定向验证（`CARGO_TARGET_DIR=/Users/linky/Code/rust/fluent-gpui/target`，全部 `--locked`）：

| 测试 | 结果 | 日志 |
|---|---|---|
| dialog open/close：标题名、modal、背景消失/恢复 | 1 通过 | `/tmp/fluent-gpui-logs-integration/dialog-open-final.log` |
| primitive nested modal 只留 topmost | 1 通过 | `/tmp/fluent-gpui-logs-integration/dialog-nested-plain-final.log` |
| 真实两个 Fluent Dialog 叠加：top→under→background 依次恢复 | 1 通过 | `/tmp/fluent-gpui-logs-integration/dialog-real-nested-3.log` |
| Dialog 内 MenuDropdown 打开：背景 Menu 消失、模态内 Menu 保留 | 1 通过 | `/tmp/fluent-gpui-logs-integration/dialog-menu-owner-2.log` |
| aria_hidden 子树移除与恢复 | 1 通过 | `/tmp/fluent-gpui-logs-integration/dialog-hidden-ax-2.log` |
| Sheet modal/标题/背景恢复 | 1 通过 | `/tmp/fluent-gpui-logs-integration/sheet-modal-ax.log` |
| Sheet 既有行为 | 18 通过 | `/tmp/fluent-gpui-logs-integration/sheet-modal-tests-2.log` |
| Menu AX：解析 JSON，聚焦 Menu，Down 后 active descendant=Delete，再 Down=Copy，selected 前后正确 | 1 通过 | `/tmp/fluent-gpui-logs-integration/menu-ax-real.log` |
| CJK consumer：marked “shan” 的 Enter 不动作，commit “删” 选中删除行，再 Enter 执行一次 | 1 通过 | `/tmp/fluent-gpui-logs-integration/cjk-consumer-2.log` |
| C5：TextInput/Textarea 选中 “hello world”、Cmd+C、绘制路径等于 Fluent selection token | 2 通过 | `/tmp/fluent-gpui-logs-integration/input-selection-final-2.log` |

- 定向破坏：移除 MenuItem 的 `aria_active_descendant()` 后 Menu AX 测试失败
  （`active_descendant_focus: null`），日志 `/tmp/fluent-gpui-logs-integration/menu-ax-break.log`；恢复后通过。
- 真实 macOS AX 探测：**未运行**。本机 `AXIsProcessTrusted()` 返回 `false`，终端没有辅助功能权限；
  未伪造通过。获得权限后可用独立脚本启动 fluent-notes 子进程并按 PID 读取 AXUIElement 树。
- modal owner 方案已按只读审查收敛：owned deferred popup 真正 reparent 到 owner 子树、selected modal 留在 ROOT；最上层 modal 按 `(deferred priority, push order)` 选择；无 modal 帧跳过 index/DFS。真实 VoiceOver 与主工作区导入/矩阵见下节。

## 主工作区导入与最终矩阵（2026-11-13）

- 导入对象：`/tmp/fluent-gpui-integration-final.patch`，8.2M / 356 files（316 vendor + 40 workspace）。
  在干净主工作区 `aa1cf6c0243e2f669a6004eef24da9db390574a6` 上先 `git apply --check`，再 `git apply`；
  未 commit、未 reset、未 stash。导入后 `git apply --reverse --check` 通过，工作树等于 patch post-image。
- 导入后仅做文档更新（`docs/a11y-checklist.md`、`docs/subsystems/README.md`、`focus.md`、`menus.md`、
  `overlays.md`、`docs/apps/notes.md`、中英 README）；Rust 源码未再变化，因此下列已绿代码检查不被纯文档改动失效。
- 最终矩阵（主工作区，`--locked`；日志目录 `/tmp/fluent-gpui-logs-integration/`）：

| 命令 | 结果 | 日志 |
|---|---|---|
| `cargo fmt --all --check`（导入前同源集成树） | 通过 | `/tmp/fluent-gpui-logs-integration/fmt-check-2.log` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 | `/tmp/fluent-gpui-logs-integration/main-clippy.log` |
| `cargo test --workspace --locked` | 全部 `test result: ok`，0 failed；1 个既有 doctest ignored（`feedback::tooltip`） | `/tmp/fluent-gpui-logs-integration/main-test.log` |
| `cargo doc --workspace --no-deps --locked` | 文档构建通过 | `/tmp/fluent-gpui-logs-integration/main-doc.log` |
| `cargo deny check`（cargo-deny 0.20.2，CI 同版预构建 aarch64-apple-darwin） | 退出 0：`advisories ok, bans ok, licenses ok, sources ok`。存在 `warning[duplicate]`（bitflags/getrandom/hashbrown 等重复条目），不是零警告 | 安装 `/tmp/fluent-gpui-final-validation/cargo-deny-install.log`；检查 `/tmp/fluent-gpui-final-validation/deny-check.log` |
| `cargo xtask gen-tokens --check` | 通过：`generated.rs is up to date` | `/tmp/fluent-gpui-logs-integration/main-gen-tokens.log` |
| `cargo xtask gen-icons --check` | 通过：`generated.rs is up to date` | `/tmp/fluent-gpui-logs-integration/main-gen-icons.log` |

- 两 app 启动 smoke（同一导入树，未在纯文档改动后重复）：Gallery `gallery_exit=0`，Notes `notes_exit=0`；
  日志 `/tmp/fluent-gpui-logs-integration/smoke-gallery.log`、`smoke-notes.log`。该轮只证明 boot smoke
  进程正常退出，未做 WindowServer 观察或真实交互，因此不算交互通过；该轮日志未记录 OS PID，不补写。
  修复后 IME 仍按下述未验证处理。
- 文档同步：`Dialog::title` 现接收 `SharedString`，自定义元素用 `title_element`，`accessibility_label`
  仅覆盖名称；`TextInput`/`Textarea` 默认 `TextInput`/`MultilineTextInput` 角色，`accessibility_label`
  只覆盖名称；`Textarea` Tab 默认缩进，`TextareaTabBehavior::Navigate` 移出字段。中英 README 与
  `docs/a11y-checklist.md` 已写明。
- 明确未验证：真实 macOS VoiceOver/屏幕阅读器（`AXIsProcessTrusted()=false`，未伪造通过）；修复后中文
  IME 真机复测（`aa1cf6c` 用户通过记录保留为修复前/源输入基线）；像素基线；dark/high-contrast 对比度。

## 交付自查

- [x] 区分 spec、GPUI 行为测试与原生视觉证据；未运行项单列。
- [x] 不保留无契约的窗口级 intercept；D1 导航组件契约已集成并定向通过，无 ignore。
- [x] README 中英文只写“未内置字体”，不写法律授权断言；Tab 描述改为“没有句柄时不参与 Tab/键盘导航”。
- [x] `docs/apps/notes.md` 同步布局顺序与“列表=已保存快照”契约。
- [x] 记录明确首失败完整日志丢失，仅保留尾部。
- [x] 未创建提交；交付为明确文件 patch。
