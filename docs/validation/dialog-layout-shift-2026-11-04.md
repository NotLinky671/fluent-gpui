# Dialog / AlertDialog 布局跳动修复（2026-11-04）

基线：HEAD `bdcf1b9` + 未提交的 T-047/T-048 改动。由 Sheet 的同类问题（用户录屏中按钮下方色块跳动 18 物理像素 @150%，即 12 逻辑像素）复查发现。

## 修改前 → 修改后（可观察行为）

- **之前**：Dialog / AlertDialog 放在 flex 列里时，打开会让下方内容上跳 12px（父级 flex 少一个 item 及其 `gap`），关闭再跳回。Gallery 的 dialog 分区位于滚动内容中，滚到底部时关闭还会因滚动内容高度变化产生同样的偏移。
- **之后**：打开/关闭对话框不再改变周围布局；下方内容的位置保持不变。

## 原因

gpui-base 的 Dialog 宿主：

- 关闭：`render` 返回一个空的 `div()`，它是 **in-flow** 的零高度 flex item；
- 打开：返回 `deferred(anchored(...))`，宿主是 **absolute**，不参与父级 flex 布局。

wrapper 之前直接返回宿主，于是打开时父级少一个 flex item（连同 12px gap），内容上跳；关闭时恢复。Sheet 在 `deferred` 宿主外已经用零高度在流占位解决了同一问题，Dialog 没有。

## 修复

[dialog.rs](../../crates/fluent-components/src/dialog.rs) 在 `render_dialog` 末尾始终用同一个零高度在流占位包住宿主：

```rust
let host = if alert { finish!(...) } else { finish!(...) };
div().h(px(0.)).child(host).into_any_element()
```

打开与关闭两种状态的布局槽一致；`deferred` 宿主仍按窗口坐标绘制，焦点陷阱、scrim 与关闭路径不变。`AlertDialog` 与 `Dialog` 共用这段 wrapper，因此同时修复。

## 证据

| 测试 | 断言 |
|---|---|
| `dialog::tests::opening_does_not_shift_the_content_below` | flex 列中打开/关闭 Dialog，下方标记的 y 不变；旧实现打开时上跳 12px |
| `dialog::tests::opening_does_not_shift_a_scrolled_page` | 滚动容器滚到底部时开关 Dialog，标记位置不变；旧实现关闭时偏移 12px |

**定向破坏**：临时去掉零高度占位后，两条测试分别报“打开上跳 12px”和“滚到底部关闭偏移 12px”；恢复后通过。

## 验证

```text
cargo test -p fluent-components --lib dialog --locked          19 passed（含新增 2 条）
cargo clippy --workspace --all-targets --locked -- -D warnings OK
cargo test --workspace --locked                                OK（components 110、core 87 等）
cargo fmt --all --check                                        OK
cargo doc --workspace --no-deps --locked                       OK
cargo test -p fluent-gallery --features screenshot --test baselines --locked
                                                               20 fixtures OK（dialog-open 基线未变）
```

日志：`target/dialog-layout-fix.log`。

## 未验证 / 说明

- 本次只在自动化窗口里验证了布局槽；真实窗口的视觉确认由用户复核。
- `AlertDialog` 与 `Dialog` 共用 wrapper，但没有单独的布局测试；行为测试覆盖的是 `Dialog`。
- 没有改动 gpui-base；修复只在本库的 wrapper 层。
- 未提交；结果对应当前未提交工作区，不是某个提交的 CI 结果。
