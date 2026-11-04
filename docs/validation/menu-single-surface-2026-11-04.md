# 菜单在 Popover 里被画成两张卡片

日期：2026-11-04。基线：HEAD `e2501be` + 未提交的 tooltip 修复。

## 现象

`MenuDropdown` 打开后：菜单外面多一层白色圆角卡片（带边框和阴影），行因此被内缩一圈，
与官方 Fluent 菜单（单层表面）明显不同。

## 根因

`Popover` 无条件给面板画表面：`min_w(160)`、`rounded(4)`、`bg(surface.background)`、
`p(12)`、`shadow`；而 `Menu` 自己也是一层表面（`p(4)`、圆角、背景、阴影）。
两者叠加就是第二张卡片，12px 内边距把行推进去。

## 修改

| 文件 | 改动 |
|---|---|
| `crates/fluent-components/src/popover.rs` | 新增 `draw_surface` 字段（默认 `true`）与 `draw_panel_surface(bool)` builder；为 `false` 时面板只保留 `whitespace_nowrap`、`occlude`、`key_context`、外部按下关闭等行为，不画卡片 |
| `crates/fluent-components/src/menu.rs` | `MenuDropdown` 调用 `draw_panel_surface(false)`，由 `Menu` 作为唯一表面 |
| `crates/fluent-components/src/menu/surface_tests.rs` | 新增单一表面回归测试 |

`Popover` 的默认行为不变，其它内容（自定义面板）不受影响。

## 回归证据

`menu::surface_tests::the_dropdown_paints_a_single_surface`：统计「宽度 < 400、高度 > 40、不透明」
的绘制卡片数量，断言恰好 1 张。

去掉 `draw_panel_surface(false)` 后失败：

```text
expected one menu surface, got [(184.0, 143.0), (160.0, 119.0)]
     — a panel card around the menu stacks a second surface
```

恢复后通过。截图对比也确认从「外套一层带边框卡片」变为单层表面。

## 验证

| 命令 | 结果 |
|---|---|
| `cargo fmt --all --check` | 通过 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 |
| `cargo test --workspace --locked` | 通过，211 个测试 |
| `cargo doc --workspace --no-deps --locked` | 通过 |
| `cargo deny check` | 通过 |

## 仍未做

- 菜单打开动画（自上而下展开）：尝试用 `with_animation` 驱动高度，但在当前 GPUI 快照里
  动画进度不前进（deferred 与独立菜单都是 0），会把菜单画成高度 0，已回退，未在代码中保留。
- 行高/内边距仍未逐项与官方截图像素比对。
- 真实窗口目视、三平台 CI。

