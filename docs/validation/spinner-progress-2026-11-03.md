# Spinner 与 ProgressBar 不定态验证记录（2026-11-03）

- 基准 SHA：`b4d1306`（`fix(badge): keep badge text legible in dark and high contrast`）
- 工作区：本次交付前的未提交改动即本记录所列文件

## 变更文件

- `crates/fluent-core/src/spec/feedback.rs` — 新增 `SpinnerSize`/`SpinnerSpec`/`spinner()`；新增纯函数 `spinner_arc_endpoints`、`indeterminate_segment_insets`、`indeterminate_animation_allowed`、`indeterminate_segment_reduced_insets`、`spinner_animation_allowed`；新增 spec 单测。
- `crates/fluent-components/src/feedback.rs` — 新增 `Spinner` 组件（`canvas` + `PathBuilder::stroke` 画 track 整圈与约 270° 指示弧，`Animation::repeat_synced` 旋转）；ProgressBar 不定态改为 `cx.reduce_motion()` 时渲染居中静态分段；新增 `feedback/tests.rs` GPUI 行为测试。
- `crates/fluent-components/src/lib.rs`、`crates/fluent-ui/src/lib.rs` — 导出 `Spinner` / `SpinnerSize`。
- `crates/fluent-gallery/src/main.rs` — feedback 区块新增四档 Spinner 与一个带 `label` 的示例；未加入 `fixtures.rs` / 像素基线。
- `README.md`、`README.zh-CN.md`、`docs/subsystems/README.md`、`plan/specs/feedback.md`、`task/M2/T-041-spinner-progress.md` — 状态与限制说明同步。

## 修改前 → 修改后

- 修改前：`Spinner` 不存在；`ProgressBar::indeterminate` 在任何 reduced-motion 设置下都持续循环动画。
- 修改后：`Spinner::new()` 渲染一个描边环 + 品牌色扫弧，`.size(SpinnerSize)` 四档、`.label(...)` 提供 `Role::ProgressIndicator` 下的 `aria_label`；`cx.reduce_motion()` 为真时 Spinner 冻结在相位 0、ProgressBar 不定态渲染居中静态分段，均不再循环。reduced-motion 为假时行为与此前一致。

## token

- track：`neutral_stroke_alpha_2`（对应上游 `colorNeutralStrokeAlpha2`）
- 指示器：`brand_stroke_2_contrast`（对应上游 `colorBrandStroke2Contrast`，存在，无需替代）

## 近似值（未与官方核对）

- 描边宽度 1.5 / 2 / 3 / 3.5 px（约直径的 1/10）
- 指示弧跨度 270°
- 旋转周期 1.5s

## 验证命令与结果

- `cargo test -p fluent-core --lib spec::feedback --locked` — 10 passed
- `cargo test -p fluent-components --lib --locked` — 62 passed（含 `feedback::tests::spinner_renders_every_size` 与 `..._under_reduced_motion`）
- `cargo xtask tasks --check` — up to date
- workspace 矩阵结果见下方"运行记录"

## 未验证

- 真实窗口 / 像素输出（无 GPU surface；未加入像素基线夹具）
- 真实 OS reduced-motion 设置（测试用 `cx.set_reduce_motion(true)` 模拟）
- reduced-motion 树中"无动画元素"未做结构化断言；测试只保证渲染不 panic，动画缺席由 `spinner_animation_allowed`/`indeterminate_animation_allowed` 纯函数单测间接覆盖
- Spinner 与官方 Fluent 截图 / 录屏的视觉核对（描边宽度、弧度、周期均为近似值）
- `Spinner::label` 的 accesskit 节点内容未被测试断言（渲染路径存在，节点内容需原生 a11y 树检查）
