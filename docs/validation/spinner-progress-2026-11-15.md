# Spinner 与不定态 ProgressBar：从静止到按上游关键帧运动

日期：2026-11-15
任务：T-041
范围：`crates/fluent-core/src/spec/feedback.rs`、`crates/fluent-components/src/feedback.rs`、
`crates/fluent-components/src/feedback/tests.rs`、`crates/fluent-gallery/{src/fixtures.rs,src/main.rs,tests/baselines.rs}`，
以及两张新基线。

## 结论

这两件组件此前在真实应用里**完全不动**。不是"动画不够顺"，是一帧都没变过：

- `Spinner` 用 `Animation::new(1.5s).repeat_synced()` + `.with_animation("spin", …)`，
  `ProgressBar` 的不定态用 `.with_animation(id, …)`。本 GPUI 快照里 `with_animation`
  的进度从不前进（菜单淡入因此回退为时间驱动，见
  `docs/validation/menu-single-surface-2026-11-04.md`），所以两者只画首帧。
- `Spinner` 还有第二层问题：`canvas` 没有固有尺寸，放在 `.size(diameter)` 的 `div`
  里拿到的是 `16×0`，`paint_spinner` 因半径为负提前返回。也就是说即使相位会前进，
  环也从未被画出来过。

修复后两个组件都由时钟驱动：每帧读 `cx.background_executor().now()` 算相位，
仍需运动时 `window.request_animation_frame()`。弧度、周期与颜色改成上游样式表的
实测值（见下表）。

## 复现（旧实现上的真实失败）

`cargo test -p fluent-components --lib feedback::tests --locked`，旧实现：

```text
the_spinner_arc_moves_as_the_clock_advances ... FAILED   （the spinner must paint a ring）
the_indeterminate_bar_moves_as_the_clock_advances ... FAILED
  a quarter of the 2s period must move the segment, but the painted bars did not change
  left: [0.0, 0.0, 0.0, 0.0]   right: [0.0, 0.0, 0.0, 0.0]
```

两条测试读的都是**绘制结果**（`window.painted_paths()` 的顶点、`painted_quads()` 的边缘），
不是"动画对象存在"：相位不前进时两者恒等，只有对照时钟推进后才暴露。

## 上游对照

来源：`@fluentui/react-spinner@9.5.0/lib/components/Spinner/useSpinnerStyles.styles.js`
与 `@fluentui/react-theme@9.1.0/lib/global/curves.js`。

| 项目 | 旧值 | 上游 | 处理 |
|---|---|---|---|
| 周期 | 1.5s（近似） | 1.5s，`linear` | 采纳，写成 `spec::feedback::SPINNER_PERIOD` |
| 描边宽度 | `strokeWidthThick`（2px） | smoke 尺寸用 `strokeWidthThick`（32px 起才是 `Thicker`） | 不变，补注释 |
| 环色 | `neutralStrokeAlpha2` | `--colorBrandStroke2Contrast` | 改 |
| 弧色 | `brandStroke2Contrast` | `--colorBrandStroke1` | 改 |
| 弧度 | 固定 270°（近似） | 容器 `-135→0→225`，两枚 `135deg` 楔形被 `105deg` 遮罩挡住，可见弧 **30°–255°** 往复，`curveEasyEase` | 改，纯函数 `spinner_arc(delta, ease)` |
| reduced-motion | 相位 0 的同一形状 | `conic-gradient(transparent 120deg, …)` → 静态 240°；环只减速到 1.8s | 采纳静态 240°；环**停止**（见下） |
| 不定态条周期 | 2s | 样式表未给 token | 保留 2s，明确标为未证实 |

reduced-motion 下环停止而不是减速，是我们与上游唯一的**有意偏离**：用户要求减少动态时，
一个永不停止旋转的对象正是要抑制的东西；记录在此而不是静默保留差异。

## 修改

| 文件 | 改动 |
|---|---|
| `crates/fluent-core/src/spec/feedback.rs` | 删 `SPINNER_ARC_DEGREES` / `spinner_arc_endpoints` / `circle_point`；新增 `SPINNER_PERIOD`、`spinner_arc(delta, ease) -> (start, sweep)`、`spinner_reduced_arc()`；`spinner()` 颜色按上游对调 |
| `crates/fluent-components/src/feedback.rs` | 新增 `loop_phase(window, cx, id, period)`（keyed 起点 + `request_animation_frame`）；`paint_spinner(bounds, spec, start, sweep)`；`canvas(...).size_full()`；不定态条改为按相位取 insets；删除两个 `with_animation` |
| `crates/fluent-components/src/feedback/tests.rs` | 6 条行为测试：运动、reduced-motion 静止（环与条各一）、渲染冒烟 |
| `crates/fluent-gallery/src/{fixtures.rs,main.rs}` | 新夹具 `spinner-medium`(240×60)、`progress-indeterminate`(240×40)；`FLUENT_GALLERY_REDUCE_MOTION=1` |
| `crates/fluent-gallery/tests/baselines.rs` | 基线渲染统一带 `FLUENT_GALLERY_REDUCE_MOTION=1`（循环永不到达"三帧一致"） |

## 基线

新增 `tests/baselines/windows/light-spinner-medium.png`（360×90）与
`light-progress-indeterminate.png`（360×60），由 gallery 以 reduce-motion 渲染，
日志 `screenshot written … after 3 frames`（静止才可能在 3 帧内稳定）。

`light-menu-panel.png` 同时被**有意**更新：它仍记录着 T-043 之前 `MenuHeader`
未右移的状态，本次全量比对报 `778 px differ (0.655%)`，diff 图显示差异只在 "Actions"
标题的水平位置。这是 T-043 的预期变化，不是本次改动的副作用。

## 命令结果

```text
cargo fmt --all --check                                                  0
cargo clippy --workspace --all-targets --locked -- -D warnings           0（仅 3 条既有 unused workspace dependency 警告）
cargo test --workspace --locked                                          427 passed / 0 failed / 1 ignored
cargo test -p fluent-core --lib spec::feedback --locked                  14 passed
cargo test -p fluent-components --lib feedback --locked                  6 passed
cargo doc --workspace --no-deps --locked                                 0
cargo test -p fluent-gallery --features screenshot --test baselines      3 passed（23 张基线全等）
FLUENT_GALLERY_SMOKE=1 cargo run -p fluent-gallery --locked              0
```

## 定向破坏（每条都应让指定测试失败）

脚本 `target/breakage_t41.py`：patch 一处决策 → 只跑目标测试 `-- --exact` → 还原并提升
mtime 强制重编。

| 破坏 | 目标测试 | 结果 |
|---|---|---|
| Spinner 固定画 reduced 弧 | `the_spinner_arc_moves_as_the_clock_advances` | 失败（预期） |
| 相位恒为 0（等价于旧 `with_animation`） | 同上 | 失败（预期） |
| Spinner 忽略 reduced motion | `the_reduced_motion_spinner_stays_still` | 失败（预期） |
| 不定态条忽略 reduced motion | `the_reduced_motion_bar_stays_still` | 失败（预期） |
| `spinner_arc` 恢复固定 270° | `the_spinner_arc_grows_from_thirty_to_255_degrees` | 失败（预期） |

## 未验证

- **不定态进度条的 2s 周期**：上游样式表没有给出可引用的 token，也没有实测官方录屏；
  数值沿用旧实现，属于近似。
- **真实窗口观感**：所有断言都在测试窗口与静态截图里；没有在原生窗口里目视核对
  转速是否与实际 Fluent 一致（需要录屏比对，与菜单淡入同一套方法）。
- **高 DPI**：`paint_spinner` 用逻辑像素算半径，未检查 125%/150%/200% 下描边是否
  模糊或偏位；`selection`/`switch` 有这类几何测试，Spinner 没有。
- **其他平台基线**：只有 Windows/light 的静态帧；macOS/Linux 未生成。
- **`with_animation` 的结论适用范围**：只在当前 vendored GPUI 快照上验证；升级后
  需要重跑 `the_spinner_arc_moves_as_the_clock_advances`，如果届时动画本身可靠，
  可以改回声明式动画，但时间驱动没有回退的紧迫理由。
