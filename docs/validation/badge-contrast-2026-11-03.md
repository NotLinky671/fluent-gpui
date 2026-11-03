# Badge：深色与高对比度主题下前景可读性修复

## 目标与范围

- 关联问题：[notes-app-2026-11-03](notes-app-2026-11-03.md#问题与复现) I-1——`BadgeAppearance::Warning` 在 `web_dark`（白字黄底）与 `high_contrast`（白底白字）下对比不足；源码上 `Important` 在 `web_dark` 同样是白底白字。
- 基准 SHA：`9f0ca35`（`feat(notes): cross-component integration app`）。修复与测试在其上以未提交工作区改动进行，随后单独提交。
- 差异范围：`crates/fluent-core/src/spec/feedback.rs`（`badge()` 前景映射 + 模块级 WCAG 辅助函数 + 回归测试）、`plan/specs/feedback.md`、`README.md`、`README.zh-CN.md`、`docs/apps/notes.md`、本文件。
- 不在范围内：原生窗口像素/视觉复核（无 GPU 像素基线）、其余组件的前景策略、`cargo deny`（上一轮刚跑过）。

## 环境

Windows 10.0.26200 x86_64；cargo test/clippy/doc 全部本地 `--locked`。未做真实窗口观察；对比数字全部由 `fluent_tokens` 生成值经 `Hsla::to_rgb` + WCAG 相对亮度公式计算。

## 复现

渲染 `Badge::new("未保存").appearance(BadgeAppearance::Warning)`；旧映射 `foreground = neutral_foreground_1`、`Important` 背景同为 `neutral_foreground_1`：

| 失败对（修复前） | 背景 → 前景 | WCAG 对比 |
|---|---|---|
| Warning / web_dark | `#fde300` → `#ffffff` | 1.30 |
| Important / web_dark | `#ffffff` → `#ffffff` | 1.00 |
| Warning / high_contrast | `#ffffff` → `#ffffff` | 1.00 |

其余 18 个 appearance×theme 组合均 ≥ 4.5（日志：`target/badge-contrast-before.log`，新测试先红后绿）。

## 映射决定

上游 `useBadgeStyles` 对 Warning/Informative/Subtle 固定用 `neutralForeground1`、其余用 `colorNeutralForegroundOnBrand`；本仓 token 集在该映射下失效（HC 调色板退化：所有 badge 背景与 `neutralForeground1` 同为 `#ffffff`）。按 ADR-0002（渲染可读性优先于 CSS 常量），`badge()` 不再抄单个 token：

- 背景仍按上游外观表取 token；
- 前景在主题自带的两个静态标签 token——`neutral_foreground_1_static`（深墨）与 `neutral_foreground_static_inverted`（浅墨）——中选与背景 WCAG 对比更高的一个。
- light/dark 下该规则落到与上游相同的颜色（黄底取深墨、其余取白），light 结果零变化；HC 下落在 `neutral_foreground_1_static`（`#000000`）。
- 两个静态 token 在所有主题中分别恒为深/浅色，故任何 token 退化组合都能保住黑白级对比；若未来 token 集二者皆不达 4.5，测试会红而不是静默降级。

## 修复后对比表（appearance × theme，WCAG 比值）

| Appearance | web_light | web_dark | high_contrast |
|---|---|---|---|
| Brand | 5.38 | 6.66 | 21.00 |
| Danger | 4.93 | 4.93 | 21.00 |
| Success | 5.37 | 5.37 | 21.00 |
| Warning | 11.95 | 11.95 | 21.00 |
| Important | 15.52 | 15.52 | 21.00 |
| Informative | 13.02 | 21.00 | 21.00 |
| Subtle | 15.52 | 14.55 | 21.00 |

唯一改变颜色的正常主题对是 Important/web_dark（`#fff`→`#fff` 不可读 → `#242424`→`#fff`），这是修复所必需。

## 命令与结果

| 命令 | 结果 | 日志 |
|---|---|---|
| `cargo test -p fluent-core --lib spec::feedback --locked`（修复前） | 失败（3 对 < 4.5） | `target/badge-contrast-before.log` |
| `cargo test -p fluent-core --lib spec::feedback --locked`（修复后） | 5 通过 | 命令行输出（见上） |
| `cargo fmt --all --check` | 通过 | `target/badge-fmt.log` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 | `target/badge-clippy.log` |
| `cargo test --workspace --locked` | 通过 | `target/badge-workspace-test.log` |
| `cargo doc --workspace --no-deps --locked` | 通过 | `target/badge-doc.log` |
| `cargo xtask gen-tokens --check` / `gen-icons --check` / `tasks --check` | 全部通过 | `target/badge-gen-*.log`、`target/badge-tasks.log` |

新增测试：`spec::feedback::tests::badge_foreground_contrasts_with_background_on_every_theme`，枚举全部 7 种外观 × 3 个内置主题，断言 `foreground`/`background` WCAG 对比 ≥ 4.5，一次性报告所有失败对。

## 未验证

- 原生窗口/像素基线：本轮无 GPU 视觉验证；受影响外观（Warning、Important）在 dark/HC 下的真实渲染仍待原生窗口复核。
- `cargo deny check` 未在本轮重跑（上一轮刚通过，按任务要求保留给最终会话）。
- 非内置 `custom()` 主题的品牌 ramp 未逐一验证；对比驱动选择对任意 ramp 同样生效，但具体比值未测。
