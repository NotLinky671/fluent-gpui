# Accordion 专题

折叠面板：一次展开一块（或用 `multiple` 各自展开），点标题或键盘切换，面板从标题下方出现，关闭时整块不渲染。

## 阅读顺序

1. 尺寸与颜色：[`crates/fluent-core/src/spec/accordion.rs`](../../crates/fluent-core/src/spec/accordion.rs) 的 `spec`、`header_colors`、`toggled`、`normalize_default`。
2. 渲染与交互：[`crates/fluent-components/src/accordion.rs`](../../crates/fluent-components/src/accordion.rs) 的 `Accordion`、`AccordionItem`、`AccordionToggle`。
3. 行为证据：[`crates/fluent-components/src/accordion/tests.rs`](../../crates/fluent-components/src/accordion/tests.rs)。
4. 现场演示：[`crates/fluent-gallery/src/sections/controls.rs`](../../crates/fluent-gallery/src/sections/controls.rs) 的 `accordion_section`，可用 `FLUENT_GALLERY_SECTION=accordion` 单独打开。

## 规格来源

数值取自 Fluent UI React `@fluentui/react-accordion` 9.12.5 的 `useAccordionHeaderStyles.styles.js` 与 `useAccordionPanelStyles.styles.js`（设计页是客户端渲染，抓不到正文，因此以源码为准）：

- 标题行 `min-height: 44px`、内边距 `0 var(--spacingHorizontalM) 0 var(--spacingHorizontalMNudge)`（即右侧 12、左侧 10）、`fontSizeBase300` / `lineHeightBase300`、`borderRadiusMedium`、透明底色。
- `small` 是 32px 高、`fontSizeBase200`；`large` / `extra-large` 只放大字号，行高仍是 44px。
- 展开图标 20px（`fontSizeBase500`），左侧再留 `spacingHorizontalS`（8px）的间距。
- 面板只有 `margin: 0 var(--spacingHorizontalM)`，没有内边距。
- 焦点环是 `colorStrokeFocus2` 的 2px 边框（`stroke_width.thick`）。
- 展开规则逐字复刻 `useAccordion.updateOpenItems`：单选时新项替换旧项，可折叠才允许清空；多选时增删，且只有 `collapsible` 或还剩不止一项时才能移除最后一项；`defaultOpenItems` 在单选下只取第一个。

## 状态归属

- 未受控时，打开集合存在窗口 keyed state `accordion-open-{element_id:?}`，由组件自己读写；受控时读取 `open_items`，只把点击结果通过 `on_toggle` 交回调用方，内部状态不参与渲染。这条与菜单的受控语义一致：显式 `open_items` 永远压过内部值。
- `default_open_items` 在渲染期经 `normalize_default` 归一化，因此单选传入多个只开第一个，多选会排序去重。
- 面板内容在打开时构建，关闭时**要等收起动画结束才移除**（上游 `unmountOnExit`）；调用方持有内容时不要依赖它被保留的局部状态。

## 键盘与焦点

- 每个可用标题是一个独立的 Tab stop（`accordion-focus-{element_id:?}-{index}`），禁用的标题既不可聚焦也不响应点击。
- `Enter` / `Space` 由 GPUI 自己的点击激活完成（标题带了 `role(Button)` 与 `on_click`）。组件曾经也处理这两个键，实测让它们走同一路径会重复，因此已删掉那一支：现在的键盘分支只处理 `↑` / `↓` / `Home` / `End`，移动焦点而不切换，不环绕，并跳过禁用项。
- 焦点环用与 `Button` 同构的写法：`focus_visible` 内换 `border_color`，再叠一层 `inset` 的零模糊 `BoxShadow` 把环加粗到 2px。`focus_visible` 只在“有焦点且最后一次输入来自键盘”时生效，所以鼠标点击不会亮环——这正是 Fluent 的 `:focus-visible` 语义。仓库里没有第二套焦点环绘制路径。
- 唯一有意偏离：Fluent 的环画在标题外侧 2px（`inset: -2px`），这里的 2px 环在标题内侧，否则会被相邻行的背景压住；视觉厚度与颜色一致。真实屏幕阅读器未验证。

## 面板动效

来源是 `AccordionPanel` 的 `collapseMotion`：`Collapse` 的默认值（`@fluentui/react-motion-components-preview@0.15.10`）是 `duration = motionTokens.durationNormal`（200 ms）、`easing = motionTokens.curveEasyEaseMax`、`staggerDelay = 0`、`animateOpacity: true`、`unmountOnExit: true`。因此高度与不透明度共用一条时钟、一条曲线，收起结束时才把内容从树里摘掉。

- 纯函数在 spec：`PANEL_MOTION`（200 ms）与 `panel_motion(progress, ease) -> (高度比例, 不透明度)`，两者取同一个缓动值，调用方无法只动高度不动透明度。曲线取 `tokens.curve.easy_ease_max`，即 `cubic-bezier(0.8, 0, 0.2, 1)`。
- 时间由组件推进：每帧读 `cx.background_executor().now()`，算 `(now - started) / 200ms`，未到 1 就 `window.request_animation_frame()`。不用 `with_animation`：当前 GPUI 快照里它的进度不前进（见[动效边界](README.md#状态与动效)）。
- 高度不是猜的：每个面板挂一个绝对定位的内容层，用 `on_children_prepainted` 量它的自然高度写进 keyed state `accordion-panel-{element_id:?}-{index}`。首帧只量不算，量到后目标追上，时钟不重开——所以首帧不会闪到某个假定高度。
- 面板容器因此需要两件事：`overflow: hidden`（`overflow_hidden` 没有公开方法，经 `Styled::style().overflow` 写 `Overflow::Hidden`）把内容裁到当前高度；`h(px(height))` 让下面的行跟着一起动。
- 关闭方向：`from` 取当前高度、目标 0，透明度走 `1 - 比例`；高度或透明度归零后不再渲染，等价 `unmountOnExit`。
- `cx.reduce_motion()` 为真时直接跳到终态：一步到位，不请求帧，也不留动画尾巴。
## 与上游的差异

- 上游图标用 `transform` 在 90° 之间过渡；本实现没有元素变换 API，直接切换 `ChevronRight` / `ChevronDown` 图标，因此这一处没有动画。
- 上游对“非 `collapsible` 且是唯一打开项”的标题加 `disabledFocusable`；本实现保持它可聚焦、可点击，只是点击结果是保持打开。
- 没有 `expandIconPosition` 选项：图标固定在标题文字之前。
- 嵌套 Accordion、面板内的滚动容器、原生屏幕阅读器读数都未验证。

## 验证入口

```text
cargo test -p fluent-core --lib spec::accordion --locked
cargo test -p fluent-components --lib accordion --locked
```

14 条行为测试覆盖点击开合、单选替换、不可折叠保持、多选各自展开、多选最后一项的守卫、禁用标题、面板 12px 外边距、键盘切换、方向键移动焦点、焦点环只对键盘出现，以及三条动效用例：展开时首帧不跳、半程在中间、200 ms 到位；收起时同一过程反向进行并在末尾卸载；reduced motion 一步到位。九条定向破坏（焦点环颜色、面板外边距、多选守卫、方向键、把动画改成瞬时、高度直接赋目标值、把周期改成 20 ms、忽略 reduced motion、把曲线降级为线性）都会让对应测试失败，记录见 [Accordion 验证](../../docs/validation/accordion-2026-11-17.md)。
