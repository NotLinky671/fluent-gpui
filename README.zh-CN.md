# fluent-ui

以 **Microsoft Fluent 2** 为视觉规范、用 **GPUI** 渲染的 Rust UI 组件库。目标是在 Windows、macOS、Linux 上提供一致的组件外观、交互行为和开发体验。

[English](README.md) | **简体中文**

> 状态：**M1（组件基础）**。API 尚未稳定，0.1 之前会有破坏性变更。

## 组件

| 组件 | 状态 | 说明 |
|---|---|---|
| `Button` | ✅ | Primary / Secondary / Subtle / Transparent，含 Toggle、Compound、Menu、Split 派生 |
| `TextInput` | ✅ | 单行，含前置/后置内容、错误态、禁用态 |
| `Textarea` | ✅ | 多行，三种外观 |
| `Field` / `Label` | ✅ | 垂直/水平布局、hint、Error/Warning/Success 验证消息 |
| `Checkbox` / `Radio` / `Switch` | ✅ | 受控与非受控两种模式 |
| `Link` / `Divider` | ✅ | 三种 Link 配色、四档 Divider |
| `Badge` / `ProgressBar` / `Avatar` | ✅ | 7 种 Badge 外观、4 档 Avatar |
| `Tooltip` | ✅ | 锚定元素、自动上下翻转、4px 间隙 |
| `Spinner` | ⏸ | 等待旋转动画基础设施 |

## 快速开始

```toml
[dependencies]
fluent-ui = "0.1"
```

```rust
use fluent_ui::{Button, FluentRoot, FluentTheme};
use gpui::prelude::*;

struct App;

impl Render for App {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(Button::new("save").child("Save"))
    }
}

fn main() {
    fluent_ui::application().run(|cx: &mut App| {
        fluent_ui::init(cx);
        FluentTheme::web_light().install(cx);
        cx.open_window(Default::default(), |_, cx| cx.new(|_| App))
            .expect("failed to open window");
    });
}
```

`FluentRoot` 负责应用主题的背景色、前景色、字体，并托管 Tooltip 浮层——应用的最外层应当用它包裹。

## 受控与非受控

选择类控件（`Checkbox` / `Radio` / `Switch`）支持两种模式，对应 Fluent React 的 `checked` / `defaultChecked`：

```rust
// 受控：状态由调用方持有
Switch::new("wifi")
    .checked(self.wifi_on)
    .on_change(cx.listener(|this, value, _, cx| {
        this.wifi_on = *value;
        cx.notify();
    }))

// 非受控：组件自持状态，无需外部变量
Switch::new("wifi").default_checked(true)
```

## 主题

5 套主题，来自 Fluent 官方 token：

```rust
FluentTheme::web_light().install(cx);
FluentTheme::web_dark().install(cx);
FluentTheme::teams_light().install(cx);
FluentTheme::teams_dark().install(cx);
FluentTheme::high_contrast().install(cx);
```

也可以跟随系统外观：`FluentTheme::follow_system(window)`。

## 架构

```
fluent-tokens     Fluent 设计 token（5 套主题 × 459 个），由脚本从官方数据生成
fluent-theme      FluentTheme：token + 品牌色 + 字体，投影到 gpui_base::Theme
fluent-icons      Fluent System Icons 的静态 SVG 数据（零 GPUI 依赖）
fluent-core       与 GPUI 的快照兼容层 + 纯函数样式解析（spec）
fluent-components 组件实现
fluent-ui         对外门面
```

**样式是纯函数**：`spec::<component>::resolve(&FluentTheme, &Props) -> Spec`，不依赖渲染即可单测。组件只负责把 `Spec` 画出来。这样"视觉是否正确"可以在没有窗口的情况下用单元测试锁定。

**渲染效果优先于 CSS 常量**：当 Fluent UI React v9 的 CSS 常量与 Fluent 2 的实际渲染效果冲突时，以**渲染效果**为准（见 `plan/01-decisions.md` ADR-0002 补充）。Switch 滑块、Radio 圆点都是按截图量像素校正的。

GPUI 精确锁定在快照版本（`gpui-pre = 0.3.10`）；用 caret 版本会漂移到不兼容的 API。

## 已知限制

- `Spinner`、`ProgressBar` 的不定态尚未实现（需要循环动画）
- `Avatar` 只渲染首字母，无图片与状态徽标
- Checkbox / Radio / Switch 的 hover 已接线，pressed 尚未
- 不支持 RTL 布局、`letter_spacing`、CSS `outline`
- 无 live region；状态变化不会主动播报给屏幕阅读器
- 高对比度需要手动切主题，未做系统级自动检测
- 输入法（IME）组合输入尚未测试

## 开发

```bash
cargo test --workspace          # 单元测试
cargo xtask gen-tokens --check  # token 生成物是否最新
cargo xtask gen-icons --check   # 图标生成物是否最新
cargo xtask tasks --check       # 任务索引是否最新
cargo deny --offline check      # 依赖许可与安全审计
cargo test -p fluent-gallery --features screenshot --test baselines   # 像素基线比对
FLUENT_GALLERY_SECTION=buttons cargo run -p fluent-gallery            # 单组件预览
```

像素基线需要 `screenshot` feature 和 GPU surface，目前只在本地运行。故意改变样式后，用 `FLUENT_BASELINE_UPDATE=1` 重新生成，并 review 图片差异。

## 许可

Apache-2.0