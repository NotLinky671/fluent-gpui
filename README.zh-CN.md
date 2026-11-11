# fluent-ui

以 **Microsoft Fluent 2** 为视觉规范、用 **GPUI** 渲染的 Rust UI 组件库。目标是在 Windows、macOS、Linux 上提供一致的组件外观、交互行为和开发体验。

[English](README.md) | **简体中文**

> 状态：**M1 组件基础已建立，M2 动效与浮层进行中**。API 尚未稳定；当前能力与未完成项见下文。

## 组件

下表描述实现范围，不代表所有平台、交互与视觉状态均已验证；验证层次见[工程规范](docs/code-standards.md)。

| 组件 | 实现范围 | 说明 |
|---|---|---|
| `Button` | 基础实现 | Primary / Secondary / Outline / Subtle / Transparent；支持尺寸、形状和图标配置；Toggle / Compound / Menu / Split 派生未实现 |
| `TextInput` | 基础实现 | 单行，含前置/后置内容、错误态、禁用态；点击别处会清除焦点，Tab 可在字段间切换；IME 尚未实测 |
| `Textarea` | 基础实现 | 多行，三种外观；点击别处会清除焦点 |
| `Field` / `Label` | 基础实现 | 垂直/水平布局、hint、Error/Warning/Success 验证消息 |
| `Checkbox` / `Radio` / `Switch` | 基础实现 | 受控与非受控模式；鼠标 hover / pressed 接线及颜色过渡已实现，完整交互验证待补 |
| `Link` / `Divider` | 基础实现 | 三种 Link 配色、四种 Divider 外观 |
| `Badge` / `Avatar` | 基础实现 | 7 种 Badge 外观、4 档 Avatar；Avatar 仅首字母 |
| `ProgressBar` | 部分实现 | 确定进度与不定态滑动动画，两者均遵循 reduced motion；不定态像素基线待补 |
| `Tooltip` | 基础实现 | `Button::tooltip` 接入根级锚定 overlay，配置 4px 间隙；悬停与翻转位置尚待人工验证 |
| `Popover` | 实验性 | 已有锚定浮层、受控/非受控开关、同帧滚动定位、外部点击关闭、Esc 关闭，以及提供 `track_focus` 时的焦点交接与 Tab 焦点陷阱；陷阱在面板项之间循环且不会逃到背后页面 |
| `Menu` / `MenuItem` | 部分实现 | 面板、前导图标、勾选与单选行、分隔线、标题、经 `Popover` 的 `MenuDropdown` 触发器接线，方向键/Home/End 导航并跳过禁用行、typeahead，以及 Tab 离开即关闭；任意层子菜单（右开/左翻、逐层 Esc 与 ←、悬停延时、首帧定位、子面板点击不误关根菜单）|
| `Spinner` | 基础实现 | 四档尺寸按上游梯度（16/20/24/28），`Spinner::label` 提供无障碍名称，遵循 reduced motion |
| `Dialog` / `AlertDialog` | 基础实现 | 自渲染 deferred 模态：Fluent scrim 与卡片（background overlay、x-large 圆角、shadow64），受控/非受控开关，打开移入焦点、关闭归还，Tab 约束在卡片内；`AlertDialog` 必须显式选择——无 Esc 与 scrim 关闭 |
| `Sheet` / 侧边抽屉 | 基础实现 | 四向抽屉（默认右侧），滑入/滑出过渡遵循 reduced motion；复用 Dialog 的 `colorBackgroundOverlay` scrim 与焦点陷阱，打开移入焦点、退出动画结束后归还；受控/非受控开关，Esc 与 scrim 按下都请求关闭；固定尺寸、内容裁剪，无滚动与拖拽关闭 |
| `Tabs` / `TabList` / `Tab` | 基础实现 | 横向/纵向标签条；选中/hover/禁用三态，指示条按上一帧记录的标签绘制边界滑动；方向键与 Home/End 移动并跳过禁用项，`role=tablist`/`tab`，选中项画焦点环 |
| `TitleBar` | 基础实现 | Windows 原生命中、最大化/还原图形与 hover/pressed；macOS 原生红绿灯、Linux 按实际装饰绘制按钮；配套窗口配置与可本地化无障碍名称；标题前有 16px 图标槽（Fluent 字形、位图 logo 或空槽预留），系统窗口图标仅在 X11 生效；见[标题栏专题](docs/subsystems/titlebar.md) |

## 快速开始

```toml
[dependencies]
fluent-ui = "0.1"
```

```rust
use fluent_ui::{Button, FluentRoot, FluentTheme};
use fluent_ui::gpui::{App, AppContext, Context, IntoElement, Render, Window};
use fluent_ui::prelude::*;

struct HelloView;

impl Render for HelloView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        FluentRoot::new().child(Button::new("save").child("Save"))
    }
}

fn main() {
    fluent_ui::application().run(|cx: &mut App| {
        fluent_ui::init(cx);
        FluentTheme::web_light().install(cx);
        cx.open_window(Default::default(), |_, cx| cx.new(|_| HelloView))
            .expect("failed to open window");
    });
}
```

`FluentRoot` 负责应用主题的背景色、前景色、字体，并托管 Tooltip 浮层——应用的最外层应当用它包裹。

## 集成验证应用

[`fluent-notes`](crates/fluent-notes) 是一个小型原生便签应用，用真实操作流程组合现有组件（草稿、主题、菜单、浮层、焦点与模态确认），补充 Gallery 的单组件矩阵。

```text
cargo run -p fluent-notes
```

数据只保存在内存中，关闭窗口即丢失；不联网、不做账号或持久化。删除与未保存切换已使用真正的 `AlertDialog` / `Dialog` 模态对话框。使用说明、可重复的手工验收清单、已知限制与 M2 接入点见 [docs/apps/notes.md](docs/apps/notes.md)；本次实际验证记录见 [docs/validation/notes-app-2026-11-03.md](docs/validation/notes-app-2026-11-03.md)。

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

token 数据包含 5 套官方主题；当前 `FluentTheme` 提供以下 3 个内置主题构造器，不提供 `teams_light()` / `teams_dark()`：

```rust
FluentTheme::web_light().install(cx);
FluentTheme::web_dark().install(cx);
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

**样式决策是纯函数**：`spec::<component>::resolve(&FluentTheme, &Props) -> Spec`，不依赖渲染即可单测。它能锁定 token 映射、尺寸、颜色和状态优先级，不能证明事件接线或最终绘制正确；还需要 GPUI 行为测试与真实窗口/像素检查。

**渲染效果优先于 CSS 常量**：当 Fluent UI React v9 的 CSS 常量与 Fluent 2 的实际渲染效果冲突时，以**渲染效果**为准（ADR-0002）。Switch 滑块、Radio 圆点都是按截图量像素校正的。

GPUI 精确锁定在快照版本（`gpui-pre = 0.3.10`）；用 caret 版本会漂移到不兼容的 API。

## 已知限制

- 图库以 `TitleBar::window_options` 接入自绘标题栏。窗口按钮独立于内容层，位于普通模态之上、Popover/菜单之下；组件不退出应用。Windows/macOS 关闭检查由应用的 `on_window_should_close` 处理，Linux 自绘按钮另用 `TitleBar::on_close` 否决。原生平台与 DPI 验证范围见[修复记录](docs/validation/titlebar-2026-11-10.md)；macOS/Linux 真机、Snap、跨显示器缩放与屏幕阅读器仍需验证。Mica 未启用；便签应用仍用系统标题栏
- `Tabs` 让整条标签条持焦点、方向键改选中（Fluent 的自动激活），不是严格 roving tabindex；hover/pressed 是瞬时样式而非过渡，选中态优先于两者；指示条位置取自上一帧记录的标签绘制边界，因此比布局变化晚一帧
- `Sheet` 的抽屉尺寸是组件常量（默认 320px，可用 `Sheet::size` 覆盖），内容超出时被裁剪，没有滚动区或拖拽关闭；Esc 与 scrim 按下都只上报 `on_open_change(false)`，底层不区分两者；退出动画结束前焦点仍被约束在抽屉内，之后只在焦点仍在抽屉内时归还——调用方若在 `open(false)` 的同一帧卸载元素，就不会发生归还
- `Avatar` 只渲染首字母，无图片与状态徽标
- Checkbox / Radio / Switch 已接入鼠标 hover / pressed 状态及颜色过渡；完整键盘与指针取消行为尚未验证
- `Popover` 的关闭路径已有行为测试：外部按下上报 `false`，面板本体与面板内交互子元素上的按下不上报，按下触发器只上报一次变化，受控调用方忽略请求时保持打开。提供 `track_focus` 时，打开会把焦点移入面板，关闭且焦点仍在面板内时归还；Esc 仅在焦点位于浮层内时生效。焦点进入的是面板容器，不自动进入第一个控件；窗口边缘采用限位而非上下自动翻转。面板容器不必标记为 Tab 停靠点；若调用方标记，它会加入控件循环
- `Badge` 前景按对比度选取：每种外观取主题的一对静态深/浅标签 token 中与徽标背景对比更高的一个，因此在 light、dark 与 high-contrast 下文字都可辨（此前 Warning 在 dark 是白字黄底、高对比度是白底白字）。映射仍取自 token 且有 spec 断言覆盖；受影响外观的原生窗口视觉检查尚待进行。见[修复记录](docs/validation/badge-contrast-2026-11-03.md)
- `Textarea` 在本版 GPUI 快照中不处理鼠标滚轮；长正文可用光标/键盘滚动，滚动条会随光标出现
- `MenuDropdown` 把 `Menu` 组合进 `Popover`，因此菜单继承锚定定位与关闭语义。方向键、Home/End、Enter/Space 与跳过禁用行均已可用；鼠标移动时写入的当前行与键盘一致，静止指针下的布局变化不会改选。子菜单支持任意层：每层各有当前行，最深层的当前行才是键盘激活目标，打开更深层的那一行用独立的 path hint 样式；→ 或 Enter 只在行确实带子菜单时打开，←/Esc 退一层，Tab 关闭整条链；悬停 300ms 自动打开，移到别的行、标题/分隔线/禁用行、离开整条链或键盘接管都会取消；右侧放不下时首帧即翻到行左侧。点击子面板属于菜单链内部（由实际发布的面板边界判断并遮挡命中），不会误关根菜单或穿透到背后控件；点击整条链外仍请求关闭。typeahead 前缀按完整层路径与会话归属，换分支、离开再返回或重新打开都不会继承。item 激活后菜单不会自动关闭，调用方必须在动作中显式关闭（便签应用每个 item 都这样做）。真实原生窗口中的子面板命中、DPI 与长内容仍未人工验证，且本版 GPUI 未提供 accesskit 的禁用状态
- `ProgressBar` 不定态与 `Spinner` 的像素基线待补；`Spinner` 弧度跨度与旋转周期为近似值，尚未与官方 Fluent 截图核对
- `Button::tooltip` 的翻转由 gpui-base overlay 提供，悬停与翻转位置尚待人工验证；通用 `tooltip(text)` 助手使用 GPUI 原生定位，不提供相同锚定保证。测试只覆盖：根是否发布 service、实体身份是否跨重绘稳定、无根时调用方拿到 `None`
- 不支持 RTL 布局、`letter_spacing`、CSS `outline`
- 无 live region；状态变化不会主动播报给屏幕阅读器
- 高对比度需要手动切主题，未做系统级自动检测
- 输入法（IME）组合输入尚未测试

## 开发

修改前阅读 [agent 工作入口](AGENTS.md)与[工程规范、组件交付清单和验证矩阵](docs/code-standards.md)，交付时填写 [PR 验证记录](.github/pull_request_template.md)。纯文档改动和代码改动的验证范围不同。

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
cargo deny check
cargo xtask gen-tokens --check
cargo xtask gen-icons --check
```

编译器版本固定在 `rust-toolchain.toml`（Rust 1.98.0），CI 安装同一版本。macOS 专属依赖 `gpui-pre-macos` 调用 `Option::map_or_default`，该 API 到 1.98.0 才稳定，因此更旧的 stable 工具链会在编译本工作区之前就报 E0658。rustup 会自动安装固定版本及 `rustfmt`/`clippy` 组件。

`cargo deny check` 可能需要联网更新公告；离线审计须注明数据新鲜度限制。本地任务索引若存在，可额外运行 `cargo xtask tasks --check`；它依赖不随仓库分发的本地笔记，不属于当前 CI 门槛。

```text
cargo run -p fluent-gallery
cargo run -p fluent-notes
cargo test -p fluent-gallery --features screenshot --test baselines --locked
```

像素比对需要 `screenshot` feature 和可用 GPU surface，目前仅在本地运行，未接入 CI。已有 15 张 Windows / light 基线，尚未覆盖 dark/high-contrast 或 hover/focus/pressed/disabled。当前目录只按 OS 区分，DPI、字体及渲染环境也需一致。只有有意接受视觉变化时才设置 `FLUENT_BASELINE_UPDATE=1`，审查图片差异后取消更新模式重跑；不要以重录基线代替排查回归。

## 许可

Apache-2.0