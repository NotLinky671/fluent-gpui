# 子系统导航

这是一份按职责组织的代码地图，不是目录清单或完成率看板。先按问题选入口，再沿源码核对实际路径；文档与源码不符时先查明原因，不沿用历史假设。
通用要求见[工程规范](../code-standards.md)，公开能力与限制见[项目说明](../../README.zh-CN.md)。

## 按问题选择入口

| 问题 | 阅读入口 |
|---|---|
| 应用初始化、类型不兼容、依赖升级 | [应用初始化与依赖边界](#应用初始化与依赖边界) |
| 颜色、字体、主题切换、图标或生成物不对 | [设计数据与主题](#设计数据与主题) |
| 尺寸、外观、状态样式不对 | [样式解析与组件渲染](#样式解析与组件渲染) |
| 点击不更新、重绘后状态丢失、动画异常、输入焦点 | [状态与动效](#状态与动效) |
| 浮层漂移、被裁剪、遮挡或关闭异常 | [浮层专题](overlays.md) |
| 截图、像素差异、基线失败或 CI 覆盖疑问 | [Gallery 与视觉验证](#gallery-与视觉验证) |

下面的“验证入口”是可执行的检查建议，不是本次运行结果。测试通过仍只证明其覆盖的场景；最终交付范围按[验证矩阵](../code-standards.md#验证矩阵)决定。

## 应用初始化与依赖边界

- **阅读顺序**：[依赖定义](../../Cargo.toml) → [对外门面](../../crates/fluent-ui/src/lib.rs)的 `application` / `init` → [兼容层](../../crates/fluent-core/src/gpui_compat.rs)的 `init_base` → [主题安装](../../crates/fluent-theme/src/lib.rs) → [根容器](../../crates/fluent-components/src/root.rs)。
- **实际路径**：`application()` 创建平台应用；`init(cx)` 先初始化 gpui-base，再安装默认 Fluent light 主题。需要自定义主题时在 `init` 后安装，随后创建窗口和视图。`FluentRoot` 应用根级视觉并托管 Tooltip overlay，不负责创建平台应用。
- **边界与约束**：应用使用 `fluent_ui::gpui` 导出的同一套类型，不能另加不兼容的官方 GPUI 版本。兼容层集中常用导出，但现有实现也直接使用同一 workspace 的 `gpui`；升级不能只改兼容层而忽略实际调用者。
- **验证入口**：[依赖图测试](../../crates/fluent-core/src/lib.rs)的 `dependency_graph_stays_independent` 检查禁用依赖和预期包名；再检查公开示例与原生 Gallery 启动。该测试不逐项验证精确版本或平台运行能力，版本仍需核对 manifest / lockfile。

```text
cargo test -p fluent-core --lib dependency_graph_stays_independent --locked
cargo run -p fluent-gallery
```

## 设计数据与主题

- **阅读顺序**：[token 快照脚本](../../scripts/snapshot-tokens.mjs)、[icon 快照脚本](../../scripts/snapshot-icons.mjs) → [token 输入](../../vendor/tokens.json)、[icon 输入](../../vendor/icons.json) → [xtask 生成器](../../xtask/src/main.rs) → [token API](../../crates/fluent-tokens/src/lib.rs)、[icon API](../../crates/fluent-icons/src/lib.rs) → [主题引擎](../../crates/fluent-theme/src/lib.rs)和[图标渲染](../../crates/fluent-components/src/icon.rs)。
- **实际路径**：生成器从仓库内的 JSON 产生 Rust 数据；主题保存 token、品牌与字体模式。`FluentTheme::install` 解析可用字体、安装 Fluent global，再单向投影部分颜色到 gpui-base。图标以内嵌 SVG 字节交给 `svg().data(..)`，不注册宿主的 `AssetSource`。
- **边界与约束**：Fluent 数据是主题来源，不能从 gpui-base 的投影反推完整主题。直接修改 global 的字段不等于重新执行 `install`。`follow_system` 的返回订阅须由调用方保存，目前只映射系统 light/dark，不自动检测高对比度。
- **来源边界**：token 快照脚本读取所提供包的版本；icon 抓取脚本当前使用上游 `main`，不是固定 commit。已提交输入可供本地再生成，但重新抓取可能变化，升级时必须审查来源及差异，不能称其为不可变的上游版本。不要手改[生成 token](../../crates/fluent-tokens/src/generated.rs)或[生成 icon](../../crates/fluent-icons/src/generated.rs)。
- **验证入口**：在工作区根目录执行两个生成物检查及 token/theme/icon 单测；检查字体和最终颜色还需真实窗口。token 数据包含五套主题，不等于 `FluentTheme` 暴露五个内置构造器，应核对实际 API。

```text
cargo xtask gen-tokens --check
cargo xtask gen-icons --check
cargo test -p fluent-tokens -p fluent-theme -p fluent-icons --locked
```

生成器使用本地输入，不代表 Cargo 首次构建也不需要下载依赖；上游快照抓取与普通回归测试是不同活动。

## 样式解析与组件渲染

- **阅读顺序**：[spec 模块](../../crates/fluent-core/src/spec/mod.rs) → 对应样式函数，例如[Button spec](../../crates/fluent-core/src/spec/button.rs) → [Button 渲染](../../crates/fluent-components/src/button.rs) → [公开导出](../../crates/fluent-ui/src/lib.rs)。其他组件按同一路径检查。
- **实际路径**：主题与 props 输入纯函数，产生尺寸、颜色及各状态的样式决策；组件将其映射成 GPUI 元素，并按实时交互/焦点状态选取目标样式。更改 spec 时也要检查渲染是否真的使用了该值。
- **边界与约束**：spec 不负责发送事件、保存用户状态或证明最终像素正确。显式样式覆盖、状态优先级和组件内部绘制仍可能影响结果；不要用一个数值断言替代行为或截图。上游 CSS 数值与核实的 Fluent 2 渲染冲突时，记录视觉证据与偏离理由，而不是静默更改真值来源。
- **验证入口**：对应 spec 单测、受影响组件的 GPUI 行为测试及原生/像素检查。当前行为测试入口之一是 [Popover 测试](../../crates/fluent-components/src/popover/tests.rs)，不能据此声称所有组件已有完整行为测试。

```text
cargo test -p fluent-core --lib --locked
```

## 状态与动效

- **阅读顺序**：[Checkbox / Radio](../../crates/fluent-components/src/selection.rs)、[Switch](../../crates/fluent-components/src/switch.rs)的 builder 和 `render` → [motion spec](../../crates/fluent-core/src/spec/motion.rs) → [动效适配](../../crates/fluent-components/src/motion.rs)。输入问题另看 [TextInput](../../crates/fluent-components/src/input.rs)和[Textarea](../../crates/fluent-components/src/textarea.rs)。
- **实际路径**：选择控件通过 `checked` 选择受控模式、`default_checked` 选择非受控模式；非受控值保存在 window keyed state 中。事件更新允许内部持有的状态、通知重绘，并向调用方发回调。hover/pressed 状态选出 spec 的目标颜色后，由动效适配调用 gpui-base `transition`。
- **边界与约束**：保持控件身份和动效 channel ID 稳定；不同 channel 不共用同一个 transition key。受控值由调用方更新，非受控默认值只用于 keyed state 初始化；不要把重绘当作重新初始化，也不要混用两种 builder 模式来猜测优先级。
- **输入边界**：文本、选择、光标等状态来自调用方持有的 `InputState` / `TextareaState`；本库绘制外壳并转发部分交互。需要跨重绘保留 state entity，不能每次 render 创建新文本状态；IME 正确性需真实输入验证。
- **动效边界**：transition 路径依赖 gpui-base 的 reduced-motion 策略，不等于所有动画都走该路径。[ProgressBar 不定态](../../crates/fluent-components/src/feedback.rs)直接使用循环 `Animation`，其 reduced-motion 处理仍待补。鼠标 pressed 接线存在，不证明键盘 pressed 或按住移出后释放等取消路径完整。
- **验证入口**：motion 纯函数与插值测试只检查数值；状态保持、回调次数、禁用/取消、焦点及 reduced motion 需要相应行为测试和原生操作。

```text
cargo test -p fluent-core --lib spec::motion --locked
cargo test -p fluent-components --lib motion --locked
```

## 浮层

先读[浮层专题](overlays.md)，区分 Button 的根级 Tooltip overlay、通用原生 tooltip 助手及自研 Popover。不要因为名称相似就假设它们共用定位、事件或焦点机制。
菜单面板的视觉与点击实现见 [Menu](../../crates/fluent-components/src/menu.rs)；它不等于完整菜单触发、键盘导航或嵌套浮层协议。

## Gallery 与视觉验证

- **阅读顺序**：[Gallery 启动与环境开关](../../crates/fluent-gallery/src/main.rs) → [组件夹具](../../crates/fluent-gallery/src/fixtures.rs) → [screenshot feature](../../crates/fluent-gallery/Cargo.toml) → [像素测试](../../crates/fluent-gallery/tests/baselines.rs) → [CI 配置](../../.github/workflows/ci.yml)。
- **实际路径**：像素测试先用 `FLUENT_GALLERY_LIST_FIXTURES` 获取名称，再为每个夹具启动 Gallery，设置 fixture / appearance / screenshot 输出，读取实际图与基线比较。基线位于[按 OS 分组的目录](../../crates/fluent-gallery/tests/baselines/)，actual/diff 输出位于 gallery crate 自己的 `target/baselines/`，不是工作区根目录的 target。
- **边界与约束**：普通 workspace 测试不会自动开启 `screenshot`。现有像素测试固定使用 light，不因手工设置 appearance 就变为 dark 全套比对；DPI、字体和渲染器也须匹配，当前目录未按这些维度分组。
- **覆盖边界**：当前有 Windows / light 静态基线，没有浮层、其他主题和完整交互态覆盖；CI 运行三平台普通检查及 Windows 启动 smoke，不运行像素比较。截图前的固定等待不等于布局已稳定，启动后定时退出不等于交互正确。
- **验证入口**：具备 GPU surface 和对应环境时运行下列命令。只有有意接受视觉变化才开启 `FLUENT_BASELINE_UPDATE=1`，审查差异后取消更新模式重跑；容差通过也可能漏掉小范围变化。

```text
cargo test -p fluent-gallery --features screenshot --test baselines --locked
```

## 修改与审查时如何维护

- 变更状态归属、实际调用路径、布局/绘制阶段、默认行为、回调、公开 API 或关键验证方法时，同步本页及涉及的专题；纯重命名只需修复受影响链接。
- PR 描述“之前 → 之后”的可观察行为、调用方影响和兼容性，不只列文件名；设计说明无需改动时给出理由，见 [PR 模板](../../.github/pull_request_template.md)。
- 文档指向机制和检查入口；本次通过/失败、环境与被测版本写入验证记录。已知限制与公开说明保持一致，不复制任务完成率或最新测试数字。
- 共享文档不依赖被忽略的本地计划或任务目录；阅读相关源码与测试后再更新，不照搬历史勾选。
