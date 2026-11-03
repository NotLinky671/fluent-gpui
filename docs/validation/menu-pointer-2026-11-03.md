# 菜单键鼠协调与静止指针回归验证

## 目标与范围

- 关联问题：菜单的键盘当前项与指针高亮不一致；按可选行序号存储当前项会在禁用前置行后指向其他动作。前一轮未提交修复已统一状态并改用 `ElementId`，但使用 `on_hover` 导致真正插入、删除行时仍会改选。
- 基准 SHA：`0fa65a4eb2fb53c1edfaf978ec0ea69a13f63606`。被测对象包含其上的未提交修改；未创建提交。
- 差异范围：中英文 README、`fluent-components/src/menu.rs`、新增 `menu/input_mode_tests.rs`、`fluent-core/src/spec/menu.rs`、焦点/浮层专题及本记录。保留了前一轮未提交修复；本轮主要将菜单内的指针接管从 `on_hover` 改为 `on_mouse_move`，并补充真实增删和绘制断言。
- 原始复现：Windows 测试平台、默认 light 主题、GPUI 测试窗口（坐标按 `scale_factor` 换算）。指针悬停 Two 后静止，调用方删除前面的 One，Three 移到原 Two 坐标；旧回调把布局 hover 当作用户选择，Enter 执行 Three。原生窗口没有执行该操作。
- 修改后：只有行内鼠标移动事件能接管当前项；按住鼠标或活动拖拽时不接管。布局、重绘、插入或删除本身不改选仍存在且可用的当前项。键盘后在同一行内移动鼠标也能重新接管，不要求跨越边界。
- 调用方影响：本轮不修改公开 builder 或回调签名；动态菜单继续要求稳定且唯一的行 ID。菜单出现于静止指针下时不自动选中其下的行，需鼠标移动或键盘导航。单独使用 `MenuItem` 仍保留局部 hover 样式。
- 不在范围内：typeahead、子菜单、Tab 关闭、完整无障碍协议，以及其他浮层既有能力与限制。

## 实际路径与同类检查

- [`Menu` / `MenuItem`](../../crates/fluent-components/src/menu.rs)：菜单持有唯一当前行身份，键盘和行内鼠标移动写入同一状态；绘制使用 spec 的当前项背景，Enter/Space 解析可用行身份后调用处理器。
- [`行为测试`](../../crates/fluent-components/src/menu/input_mode_tests.rs)：`View.rows` 决定真实元素树，删除测试不再以禁用代替删除；`painted_quads` 独立检查行背景数量、颜色及缩放后位置。更新动态行后直接绘制并检查场景，再推进后续帧检查状态保持。
- GPUI `on_hover` 会在布局变化让另一元素移到静止指针下时触发；`on_mouse_move` 只处理输入事件，并沿用 GPUI 的 Bubble 阶段和 hitbox 遮挡检查。
- 同类：单独使用的 MenuItem 没有上层菜单状态，仍需局部 hover；Button 的 hover 同时驱动颜色与 Tooltip，不承担菜单动作身份，因此未改为 mouse-move。Popover 继续负责定位、关闭及焦点陷阱，未改其实现。
- 已同步[焦点专题](../subsystems/focus.md)、[浮层专题](../subsystems/overlays.md)及中英文公开限制。样式 spec 保留前一轮纯函数背景决策；不升级 GPUI，不更改依赖或生成物。

## 回归证据

在本轮事件修复之前，先将真实增删及绘制断言加入正式测试：

- `removing_an_earlier_row_under_a_stationary_pointer_keeps_identity` 失败：期望 Two 上移后的 y=4，实际高亮仍在 y=38（Three）。
- `inserting_before_the_current_row_under_a_stationary_pointer_keeps_identity` 失败：期望 Three 下移后的 y=72，实际高亮在 y=38（Two）。
- 首轮结果：10 通过、2 失败。完整输出保留于本地 `target/menu-fix-before-20261103.log`。此红灯是有意保留旧事件接线时得到的回归证据，原因已由本轮修复解决。
- 改为 mouse-move 后，菜单 18 条行为测试通过。完整输出：本地 `target/menu-fix-after-20261103.log`。
- `keyboard_and_pointer_paint_one_current_row` 检查键盘→指针→键盘→同行内鼠标移动，逐步断言恰一行高亮及最终激活目标；`removing_the_current_row_leaves_no_action` 真正删除 Two，并检查无当前行背景、Enter 无动作。

这些是 GPUI 测试场景的绘制 quad 和事件证据，不是 GPU 栅格化、截图像素比较或原生平台交互验证。

## 验证记录

环境：Windows x86_64，rustc 1.95.0，Cargo 1.95.0。命令在上述基准加未提交修改上运行，重型检查串行执行。

| 命令或操作 | 结果 | 证据及边界 |
|---|---|---|
| `cargo fmt --all --check` | 通过 | 最终测试时序调整后复查 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 | 最终日志：本地 `target/menu-fix-final-clippy-20261103.log` |
| `cargo test --workspace --locked` | 通过 | 最终日志：本地 `target/menu-fix-final-test-20261103.log`；包含组件 46 条测试；workspace 中原有 1 条 ignored 不计通过 |
| `cargo doc --workspace --no-deps --locked` | 通过 | 本地 `target/menu-fix-verify-doc-20261103.log` |
| `cargo deny check` | 通过，有警告 | 本地 `target/menu-fix-verify-deny-20261103.log`；advisories/bans/licenses/sources 均 ok，存在依赖重复版本警告。未使用离线参数；未另行证明远端公告库的实时新鲜度 |
| `cargo xtask gen-tokens --check` | 通过 | 本地 `target/menu-fix-verify-tokens-20261103.log`，未更改生成物 |
| `cargo xtask gen-icons --check` | 通过 | 本地 `target/menu-fix-verify-icons-20261103.log`，未更改生成物 |
| `cargo xtask tasks --check` | 通过 | 本地 `target/menu-fix-verify-tasks-20261103.log`；本地忽略目录的补充检查 |
| 原生菜单交互、dark/high-contrast、GPU 像素检查、三平台 CI | 未验证 | 本轮只有测试平台事件和 scene quad 断言，未记录或更新像素基线 |

完整运行日志位于被忽略的本地构建目录，新 clone 不保证存在；本记录保留命令、结果、复现与失败原因以便重新验证。无未解决的本轮自动检查失败。原生视觉和平台输入验收由维护者后续执行，不能据此记录为发布验收通过。

## 交付自查

- [x] 已区分 spec、GPUI 事件/绘制场景、原生视觉证据。
- [x] 已沿实际事件路径核对并同步公开说明及相关子系统。
- [x] 已记录修复前失败与修复后结果，未以补录像素基线规避问题。
- [x] 未提交、未推送；保留现有未提交修改供用户复核。
