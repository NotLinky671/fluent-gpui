# Dialog 页脚动作的外观

日期：2026-11-03。基线：HEAD `28a6c47` + 未提交修改。

## 现象

图库 `Edit details` 对话框页脚的 `Cancel` 没有边框，官方 Fluent 的对话取消按钮是**有边框的**。

## 原因

这不是组件缺陷，是**调用方选错了外观**。`Dialog` 只提供 `footer` / `footer_child`，
不决定每个动作的外观；图库与便签应用当时给取消类动作用了 `Subtle`（无边框、静止透明），
而 Fluent 的对话动作契约是：

| 动作 | 外观 |
|---|---|
| 确认（Save / 保存并继续） | `Primary`（品牌填充） |
| 取消 / 留在当前（放弃型） | `Secondary`（中性填充 + 1px 边框） |

`Subtle` 在 Fluent 里表示"第三优先级或无边框工具动作"，出现在对话页脚会被读成另一个层级。

## 修改

| 位置 | 改动 |
|---|---|
| `crates/fluent-gallery/src/main.rs` | `gallery-dialog-cancel`：`Subtle` → `Secondary` |
| `crates/fluent-notes/src/app.rs` | `notes-pending-stay`（留在当前）：`Subtle` → `Secondary` |
| `crates/fluent-components/src/dialog/tests.rs` | 测试夹具同步为 `Secondary`，与真实用法一致 |
| `crates/fluent-components/src/dialog.rs` | `footer_child` 文档写明该契约，避免下一位调用方再选错 |

`Save` / `保存并继续` 保持 `Primary` 不变。

## 证据

- 截取图库对话框页脚（`FLUENT_GALLERY_SCREENSHOT` + 临时把 `dialog_open` 置 true，截完已还原）：
  `Cancel` 显示 1px 中性边框与圆角，`Save` 为主色填充。
- `cargo test --workspace --locked` 通过；对话框页脚稳定性测试
  `an_open_dialogs_footer_buttons_hold_state_across_repaints` 改夹具后仍通过。

## 与前一记录的关系

[`hover-fade-transparent`](hover-fade-transparent-2026-11-03.md) 里"悬停时深灰闪一下"的根因是
透明色混色（`mix` 按原始通道插值）。把 `Cancel` 改成 `Secondary` 后其静止色不再透明，
该按钮**同时**也不再经过那条路径；但混色缺陷本身对任何透明底控件都成立，修复与
`button/hover_tests.rs` 的逐帧断言保留。

## 未验证

- 图库/便签应用在真实窗口中的目视确认（本轮只有截图与测试）
- dark / high-contrast 下 Secondary 页脚按钮的对比度（沿用既有 Secondary 规范）
- 三平台 CI
