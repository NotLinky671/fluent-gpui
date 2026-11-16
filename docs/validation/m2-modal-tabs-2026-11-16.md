# M2：嵌套模态与 Tabs 当帧几何修复

## 问题与结果

基线：`b3e6436f304dfcb2dcf34f90d2924c902f584189`，本记录对应其上的未提交修复。报告来自 M2 审核的 GPUI 实际事件与绘制探针，不是原生窗口人工操作报告。

| 触发路径 | 修改前 | 修改后 |
|---|---|---|
| Dialog/Sheet 内嵌 Sheet，按 Escape | 子 Sheet 的 Cancel 继续传播，内外层一起请求关闭 | 子 Sheet 消费 Cancel，仅当前层请求关闭；受控调用方拒绝后重复 Escape 仍只请求当前层 |
| Dialog/AlertDialog 内打开 Sheet | Sheet 的 deferred priority 为 0，父 Dialog 的 10 导致父遮罩盖住子抽屉 | 共用 `gpui_base::MODAL_PRIORITY`（10），相同 priority 稳定排序，父内容、子遮罩、子内容按嵌套顺序绘制 |
| 首次挂载 Tabs，或改变前方标签长度 | 指示条首次不画；布局变化后用上一帧边界，直到额外重绘才更新 | 当帧 `on_children_prepainted` 读取标签边界并采样位置/长度过渡，同帧 canvas 绘制 |

修复不改变组件 builder、受控/非受控约定、Tabs 焦点模型或动画时长。低层 `Dialog::layer` 仍可显式偏移优先级；本次保证的是 Fluent Dialog/AlertDialog/Sheet 默认层的嵌套顺序，不是全局浮层协调器。

## 实际路径与同类检查

- gpui-base Sheet 的 Cancel handler 不再调用 `cx.propagate()`；关闭请求与回调路径保持原样。Dialog 已消费自己的 Cancel，无需同步修改关闭逻辑。
- gpui-base Dialog/AlertDialog 默认使用共享模态优先级，Fluent Sheet 显式采用同值。GPUI deferred prepaint 与 paint 都稳定排序，同优先级的嵌套子层在父层之后调度。覆盖 Dialog→Sheet、AlertDialog→Sheet、Sheet→Dialog 及 Dialog→Sheet→Dialog。
- TabList 移除跨帧 Geometry store。每次 render 的局部槽只连接本帧预绘制与 canvas；位置/长度仍分别使用既有 x/y、w/h motion channel。空列表不画指示条；原有键盘、禁用项、越界选择及宽度动画测试一并回归。
- 中英文 README、子系统导航、浮层专题与本地 T-047/T-048 记录已同步；历史 Tabs 交付记录保留原文并标明已被本修复替换。

## 回归证据

新增 `sheet/nested_tests.rs` 七条与 `tabs/geometry_tests.rs` 三条 GPUI 测试。Sheet 通过真实 KeyDown 分发检查关闭回调，并按实际 painted quads 逐层断言遮罩与内容交错顺序。Tabs 从未挂载状态开始，在一次 `draw` 后立即断言；动态标签变化比较事件首帧与后续稳定帧，期间不推进时钟。

在修复生产代码前运行新测试：Sheet 2 通过、5 失败；Tabs 0 通过、3 失败。Escape 回调为 `[inner, outer]`，受控拒绝后为 `[inner, outer]` 而非 `[inner, inner]`；子 Sheet 内容在父遮罩之前绘制。横/纵指示条首帧缺失；动态标签变化首帧为 `(57, 75)`，额外重绘后为 `(292, 75)`。修复后组件完整测试 288 通过，包含上述 10 条回归。

原生夹具新增 `dialog-sheet-open`（640×420 逻辑像素），父 Dialog 中固定打开宽 260px 的子 Sheet，避免只靠普通独立 Sheet 基线推断嵌套遮挡。截图检查只证明静态 GPU 输出，不能代替 Escape、受控拒绝或首帧几何测试。

## 验证记录

环境：Windows AMD64；原生截图 150%（640×420 逻辑像素 → 960×630 物理像素），light/dark/high-contrast，reduced motion；使用本机主题字体与 GPUI Windows GPU surface，未单独记录 GPU 适配器型号。`rustc 1.98.0 (88d9e12ae 2026-08-18)`、`cargo 1.98.0 (797e8a9bc 2026-08-05)`。

| 命令/操作 | 结果 | 证据与范围 |
|---|---|---|
| 新回归对旧实现 | 通过（成功复现缺陷） | 10 条中 8 条失败，失败行为见上文 |
| `cargo test -p fluent-components --lib --locked` | 通过 | 288 passed，含 10 条新回归 |
| `cargo fmt --all --check` | 通过 | 完整检查入口已通过格式检查 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 | 完整检查入口已通过 clippy |
| `cargo test --workspace --locked` | 通过 | 全部通过；组件 288 passed，既有 tooltip doctest 1 ignored |
| `cargo doc --workspace --no-deps --locked` | 通过 | workspace 文档构建结束，退出 0 |
| `cargo xtask gen-tokens --check` / `gen-icons --check` | 通过 | 两份生成物均 up to date |
| `cargo xtask tasks --check` | 通过 | 39 项，本地索引 up to date |
| Windows/light 原生基线 | 通过 | `cargo test -p fluent-gallery --features screenshot --test baselines --locked`：3 passed，25 个夹具比对通过；仅新增嵌套夹具基线，原有 24 张未重录 |
| 嵌套模态原生主题检查 | 通过（静态） | light/dark/high-contrast 三张新截图均检查，子 Sheet 明亮表面与内容绘制在被遮罩压暗的父 Dialog 之上；150% DPI，reduced motion |
| `git diff --check` 与最终格式检查 | 通过 | 最终补丁检查均退出 0 |
| `cargo deny check` | 未通过（环境阻塞） | 公告库 `C:\Users\nothh\.cargo\advisory-dbs\db.lock` 报 `attempted to take an exclusive lock on a read-only path`；相同命令申请扩大权限后工具等待 120s 超时，没有新建后台任务，不能记作审计通过 |

原生证据：版本控制内的 [Windows/light 嵌套基线](../../crates/fluent-gallery/tests/baselines/windows/light-dialog-sheet-open.png)；本地 [dark 截图](../../target/m2-fix-native/dark-dialog-sheet-open.png)与 [high-contrast 截图](../../target/m2-fix-native/high-contrast-dialog-sheet-open.png) 位于忽略目录，未作为跨平台基线。普通现有 Tabs 横/纵基线也通过；首帧时序由 GPUI 测试证明，不由稳定截图推断。

## 未验证项与提交状态

本次未验证 macOS/Linux 原生窗口、真实物理 Escape/焦点操作、屏幕阅读器、系统 reduced-motion 和动画主观观感。GPUI 新几何/嵌套测试使用 reduced motion；已有 Tabs 宽度过渡测试继续覆盖非 reduced-motion 插值。原生像素基线不能证明事件时序与无障碍完整性。

提交状态：未提交；用户本次授权为修复。
