# 悬停淡入时「深灰闪一下」的根因与修复

日期：2026-11-03。基线：HEAD `28a6c47`；未提交修改。上一份记录
[`dialog-focus-flicker`](dialog-focus-flicker-2026-11-03.md) 解决的是另一件事
（`on_hover` 逐帧真假抖动），本文是它修完之后仍然可见的**颜色中间态**。

## 现象

Dialog 页脚的 `Cancel`（Subtle 外观）按钮：指针移上去，先出现一瞬**比静止时更深**的
灰色，再落到正常的悬停色；移开时同样。浅色与深色主题都出现。


> 后续：`Cancel` 已按 Fluent 约定改为 **Secondary**（有边框），见
> [`dialog-footer-actions-2026-11-03.md`](dialog-footer-actions-2026-11-03.md)。
> 混色缺陷独立成立：任何透明底控件做颜色淡入都会命中，故保留修复与测试。
## 根因

`spec::motion::mix` 之前按原始 HSLA 通道插值，而 Subtle / Transparent 按钮的静止色是
Fluent 的 `transparent_black()`（色相 0、饱和度 0、**明度 0**、alpha 0）。

于是 `transparent_black → #f5f5f5` 的淡入在通道空间里同时抬高明度与 alpha：

| 进度 | 宏观观感 |
|---|---|
| 0.25 | 半透明近黑，比静止（全透明）和终态（`#f5f5f5`）都深 |
| 0.5 | 仍是半透明深灰 |
| 1.0 | `#f5f5f5` |

控件级实测：浅色主题淡入过程中绘出 lightness **0.233**，深色主题 **0.053**——都在
两端之外，正是肉眼看到的"深灰闪一下"。

只有透明底的按钮受影响；Secondary（静止 `#ffffff`）、Primary（品牌色）的静止色不透明，
插值不会经过暗值，所以其他按钮看起来正常。

## 修复

`crates/fluent-core/src/spec/motion.rs`：`mix` 改为**预乘 alpha** 权重。

```text
alpha       = lerp(a_from, a_to, p)
weight_to   = a_to   * p       / alpha
weight_from = a_from * (1 - p) / alpha
h, s, l     = 各自按这两个权重混合
```

透明端因此不贡献颜色，淡入退化为"只抬 alpha"，终态颜色从一开始就是正确的。

两个不透明端点时权重退化为 `1 - p` 与 `p`，与旧行为完全一致，其他控件不受影响。

## 回归证据

| 测试 | 旧实现 | 新实现 |
|---|---|---|
| `spec::motion::tests::mix_interpolates_lightness_and_alpha` | **失败**（断言 l=0.5，即缺陷本身，已改写为 l=1.0） | 通过 |
| `spec::motion::tests::fading_out_of_transparency_never_darkens_the_surface` | 失败 | 通过 |
| `spec::motion::tests::opaque_colours_still_mix_linearly` | 通过 | 通过 |
| `button::hover_tests::hovering_a_subtle_button_never_darkens_past_both_ends` | **失败**，绘出 lightness 0.233 | 通过 |
| `button::hover_tests::hovering_a_subtle_button_in_dark_never_darkens_past_both_ends` | **失败**，绘出 lightness 0.053 | 通过 |

控件级测试的做法：记录静止帧的绘制 quad，再逐帧推进时钟（`executor().advance_clock`）
重绘，只统计**相对静止帧发生了变化**的 quad（避免把深色页面底色误判为变暗），断言这些
quad 的 lightness 不低于两端。

**需要说明**：旧的 `mix_interpolates_lightness_and_alpha` 断言的正是缺陷值，属于"测试
锁住了 bug"，已改写并在注释里说明原因。

## 验证

Windows x86_64，串行执行：

| 命令 | 结果 |
|---|---|
| `cargo fmt --all --check` | 通过 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 |
| `cargo test --workspace --locked` | 通过，209 个测试 |
| `cargo doc --workspace --no-deps --locked` | 通过 |
| `cargo deny check` | 通过 |

## 未验证

- 真实窗口的肉眼确认（本轮只有测试窗口的逐帧 quad 断言；未经人工目视）
- 其他透明底控件的同类表现（菜单项、Checkbox 等共用 `mix`，理论上同修，未逐个取证）
- 三平台 CI

