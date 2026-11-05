# 浮层：实际路径、同帧定位与关闭语义

先从[子系统导航](README.md)确认问题类型；公共能力与限制见[中文说明](../../README.zh-CN.md#已知限制)和[英文说明](../../README.md#known-limitations)。
本页记录当前实现机制与验证边界，不是本次测试通过记录。旧的“Popover 是 gpui-base 薄包装”假设不再适用。

## 入口与职责

| 入口 | 实际路径 | 不应混同的能力 |
|---|---|---|
| `Button::tooltip` | Button 测量触发器并向根级 gpui-base `TooltipOverlay` 发送请求 | 不等于 GPUI 原生 `.tooltip()`，也不等于 Popover 定位器 |
| 通用 `tooltip(text)` 助手 | 构建 `TooltipView` 的闭包，供 GPUI 原生 `InteractiveElement::tooltip` 使用 | 内容采用 Fluent 外观，不代表具有 Button overlay 的锚定与间隙配置 |
| `Popover` | window keyed open state + 同帧测量 + 自研 deferred positioner + `on_mouse_down_out` 关闭；提供 `track_focus` 时做焦点交接与 Tab 陷阱 | 不依赖 TooltipService，也不是根级 PopoverHost；定位测试不证明关闭正确 |
| `Dialog` / `AlertDialog` | gpui-base `Dialog`/`AlertDialog` 宿主 + Fluent scrim/卡片 + window keyed 状态 + 焦点交接 | 自己 deferred 渲染，无 root host；不走 Popover 定位器 |
| `Sheet` | gpui-base `Sheet` 宿主（焦点陷阱 / Esc / overlay 关闭）+ window keyed 状态 + `Presence` 滑动；scrim 与 Dialog 共用 token | 不是 Popover 定位器；无拖拽关闭或滚动区；底层不区分 Esc 与 scrim |
| `MenuDropdown` | 把 `Menu` 组合进 `Popover`，继承定位与关闭；键盘与鼠标移动共用菜单的当前项，布局变化不触发改选 | 不等于完整菜单协议：无子菜单 |
| `Menu` / `MenuItem` | Fluent 菜单面板、菜单项、前导图标、勾选/单选行、方向键/Home/End/Enter 导航 | 键盘只覆盖已声明的 `.item()` 行；子菜单未实现 |

**阅读顺序**：[公开导出](../../crates/fluent-ui/src/lib.rs) → 对应组件 → 状态/坐标读写者 → [现有测试](../../crates/fluent-components/src/popover/tests.rs)。

## Button 锚定 Tooltip 路径

源码：[Button](../../crates/fluent-components/src/button.rs)的 tooltip builder / hover 接线、[FluentRoot](../../crates/fluent-components/src/root.rs)、[TooltipService](../../crates/fluent-components/src/tooltip.rs)、[TooltipView / TooltipSurface](../../crates/fluent-components/src/feedback.rs)。

1. `FluentRoot::render` 用 window keyed state 创建/复用 `TooltipOverlay`，发布到 App global 的 `TooltipService`，并将 overlay 放在根级 children 后。
2. 带 tooltip 的 Button 为触发边界建立共享 slot，在子元素 prepaint 后更新；hover 回调读取 slot。没有 service 或尚无测量值时不请求显示。
3. hover 进入时用 `TooltipRequest::new(bounds.dilate(px(4.)), ...)` 提交内容，配置 Fluent 的 4px 间隙；定位/翻转交给 gpui-base overlay。hover 离开时请求隐藏。
4. tooltip 分支同时维护按钮颜色所需的 hover 状态；修改这一回调时不能只保留 Tooltip 而遗漏状态更新。
5. 「已请求显示」的标志存在 window keyed state 里，不能放在 render 局部变量中：show 之后的这次重绘会把局部标志重新初始化成 `false`，离开边沿于是看不到未完成的请求，不会发隐藏，Tooltip 就留在屏幕上。

**约束与边界**：

- overlay 依赖宿主渲染 `FluentRoot`，不是 `fluent_ui::init` 单独创建的。`TooltipSurface` 的静态预览不检查显示时机或定位。
- 通用 [tooltip 助手](../../crates/fluent-components/src/feedback.rs)只生成内容闭包，使用 GPUI 原生定位，不经过上述 service。不能从内容相同推定定位相同。
- service 查询只取 App global，没有 window 参数；不同窗口的 root 会发布各自 overlay，因此当前不能宣称多窗口或嵌套 root 隔离可靠。
- 当前显示请求由 hover 驱动；键盘焦点触发、滚动时已显示 Tooltip 是否实时跟随、悬停/翻转位置都需要专门验证。不要把 Popover 的同帧保证套到这条路径。

## Popover：布局与绘制

源码：[Popover](../../crates/fluent-components/src/popover.rs)的 `TriggerBounds`、`PopoverPositioner`、`panel_origin` 与 `render`。

```text
render：为本次元素树创建共享 TriggerBounds，构建 trigger、backdrop 和 panel
  ↓
request_layout / 布局：计算面板尺寸；不在这里复制尚未测到的锚点
  ↓
普通树 prepaint：trigger 的 on_children_prepainted 写入首个包装子元素的窗口边界
  ↓
deferred positioner prepaint：读取同一共享值，结合面板尺寸和视口计算位置
  ↓
在绝对元素偏移下 prepaint 面板；完成定位后才 paint
```

### 坐标与时序约束

- `TriggerBounds` 为本次元素树共享的 `Rc<Cell<Option<Bounds<Pixels>>>>`，不是保存上一帧位置的长期 keyed state。测量与读取发生在同帧；缺少测量时不绘制面板。
- 不能在 render 或 request_layout 先复制 bounds 再期待下一帧纠正。连续重绘能让旧坐标追赶，但不能证明滚动事件后的第一帧正确。
- `panel_origin` 的目标是触发器左侧与底部 + 4px；结合 `8px + client inset` 连续限位。边缘处理不是上下翻转，接近窗口边缘时也不保证继续保留触发器间隙。
- 面板大于视口时对齐 inset，避免 clamp 上下界倒置；这不等于大内容总能完整容纳或自动滚动。
- 定位器基于 `layout_bounds`、当前元素偏移与 `pixel_snap_point` 抵消祖先偏移，再调用 `with_absolute_element_offset`。不要再次加滚动距离，或改为整逻辑像素取整而丢失物理像素对齐。
- deferred 路径让面板脱离普通滚动内容的裁剪；这不要求先改成根级 Host。若未来引入 Host，应另行论证生命周期、事件与状态，而不是把它当作同帧定位的必要前提。

### 外观与覆盖

面板背景/前景及阴影来自[tooltip spec](../../crates/fluent-core/src/spec/feedback.rs)，Popover 自己设置最小宽度、圆角、padding 和不换行。
当前 `Styled` 的 `refine_style` 应用于外层，而不是直接转发面板表面；不要把外层样式覆盖描述为完整的 panel 样式 API。

## Popover：开关与事件路径

| 状态来源 | 触发器包装层的内置点击 | 面板收到 `on_mouse_down_out` 后的处理 |
|---|---|---|
| 未提供 `open`（非受控） | 翻转 keyed store，通知重绘，并调用可选的 `on_open_change(next)` | 若按下点落在面板外且在触发器内则跳过（交给触发器自身的切换）；否则置 store 为 false，必要时通知重绘，调用可选的 `on_open_change(false)` |
| 提供 `open(value)`（受控） | 当前不安装内置 toggle 回调；调用方自行接线触发动作 | 仍写内部 store 并发出可选关闭回调，但显示状态继续取显式 `open`；需调用方更新值才能关闭 |

- `default_open` 仅初始化 window keyed store，控件身份要稳定。`on_open_change` 是可选通知，不应因名字推定所有状态变化和交互都已覆盖。
- 打开时构建全视口 `anchored` + `deferred` 背板，先加入背板，再加入 deferred 面板。背板只负责“面板外”的关闭请求。
- **绘制在上方不等于鼠标命中遮挡。** 面板容器调用 `InteractiveElement::occlude`（`HitboxBehavior::BlockMouse`）阻断其下命中，因此面板上的点击不会落到下方控件。行为测试用一块垫底元素验证了这一点：移除 `occlude` 后“面板内点击不穿透”的两条测试会失败。
- `Escape` 通过 `fluent_components::Dismiss` 关闭：`fluent_ui::init` 把它绑定到 `escape` 并限定在 `OVERLAY_KEY_CONTEXT`，Popover 在触发器包装层与面板上都设置该 context 并挂 `on_action`。**只有焦点位于浮层内时才会触发**，所以调用方必须通过 `track_focus` 提供焦点句柄；同一个句柄还用于打开时的焦点移入、关闭归还和 Tab 陷阱，见[焦点专题](focus.md#浮层焦点交接)。
- 关闭行为已有测试覆盖：外部按下上报 `false`；面板本体、面板内交互子元素上的按下不上报且不穿透；按下触发器只上报一次变化（触发器自身的切换与“点击外部”不会重复上报）；受控调用方忽略请求时保持打开。
- **表面归属**：`Popover` 默认给面板画卡片（内边距 12、圆角、背景、阴影）。内容自带表面时必须用 `draw_panel_surface(false)` 关掉，否则会叠出第二张卡片并把内容内缩一层——`Menu` 就是自带表面的例子，`MenuDropdown` 已经这样接线。打开动画由 `Menu` 自己做：下拉打开时把「代号」加一，菜单按 `now - opened_at` 在 350ms 内把**不透明度**从 0 抬到 1（四分 ease-out），同时整体向下归位约 4px（帧间互相关测得，30fps 录屏，精度 ±1–2px），所以观感是从按钮底部往下落定；独立摆在页面里的菜单不播这个动画。
- 焦点进入/归还与 Tab 约束只在提供 `track_focus` 时成立，且均有行为测试；未提供时这些能力不成立。嵌套或同时打开的多个浮层仍没有协调机制。Menu 的键盘模型见[菜单实现](../../crates/fluent-components/src/menu.rs)。

## Dialog / AlertDialog：模态卡片

源码：[dialog.rs](../../crates/fluent-components/src/dialog.rs)、[spec](../../crates/fluent-core/src/spec/dialog.rs)。

**渲染路径**。组件把 gpui-base `Dialog`/`AlertDialog` 宿主拼好后交给它自己渲染：宿主在 render 里产出 `deferred(anchored(...))`，priority `10 + layer`，因此不需要任何根级 overlay host——`FluentRoot` 只负责提供 trap 感知的 Tab 遍历 context。关闭状态下宿主渲染一个空 `div()`，所以对话框元素可以一直挂在树里。

**布局占位**。宿主打开时是绝对定位的 deferred 层，关闭时却是 in-flow 的空 `div()`；直接返回宿主会让父级 flex 在开关前后差一个 item 及其 gap，下方内容跳动。wrapper 用同一个零高度在流占位包住宿主（与 Sheet 相同的约定），两种状态的布局槽一致。

**scrim 与卡片**。scrim 由调用方（本组件）以 `DialogBackdrop` 提供，铺满宿主并填 `colorBackgroundOverlay`（web_light 40% 黑，web_dark 50%）。卡片是 `DialogPopup`：neutral background 1 底、`x_large` 圆角、`s64` 阴影、24px padding、16px 内容间距、8px footer 间距，标题/正文用 `base_500`/`base_300` 字号与行高。卡片 `occlude()`，落在卡片上的按下不会到达 scrim；scrim 上的按下对普通 Dialog 走 `Cancel` 路径。

**状态与关闭**。`.open(bool)` 为受控；`default_open` 仅初始化 window keyed store。Escape / scrim 按下 / `DialogHandle::close` 统一经 `DialogChangeReason::{Cancel,BackdropPress,Imperative}` 上报给 `on_open_change`，再（非受控时）写 keyed store 并 `cx.notify`。`on_ok`/`on_cancel` 返回 `false` 可否决；Enter 绑定到宿主的 `Confirm` 走 `on_ok`。`AlertDialog` 由 gpui-base 特化：`Role::AlertDialog`、无 scrim 关闭；本组件再叠加 `close_on_escape(false)`，因此必须点击 footer 动作。

**焦点**。宿主自带 `focus_trap`，配合 `FluentRoot` 的 `advance_focus` 让 Tab 在卡片内循环。gpui-base 不记忆旧焦点，本组件在 wrapper 里用 keyed state 复刻 Popover 的交接：打开首帧记录 `window.focused` 并聚焦宿主句柄，关闭首帧在焦点仍位于对话框内时归还。**如果调用方在关闭的同一帧直接把对话框元素卸载，归还永远不会发生**——组件必须保持挂载并用受控 `open` 驱动，notes 应用就是这么接的。

**命令入口**：`cargo test -p fluent-components --lib dialog --locked`。

## Sheet：侧边抽屉

源码：[sheet.rs](../../crates/fluent-components/src/sheet.rs)、[spec](../../crates/fluent-core/src/spec/sheet.rs)。

**渲染路径**。组件把 gpui-base `Sheet` 宿主拼好后交给它自己渲染。宿主在 render 里产出 `anchored` 的全视口层，自带 `key_context("Sheet")`、`track_focus` 与 `focus_trap`，并把 overlay 按下和 Escape 统一走 `request_close` → `on_close`；它本身没有开关状态和动画。wrapper 再把宿主包进 `deferred`（与 Dialog 相同），让模态层在帧末绘制、不被祖先的滚动/裁剪容器截断。

**布局占位**。`deferred` 宿主是绝对定位、不参与父级 flex 布局；如果打开时才把在流占位换成绝对宿主，父级会少一个 flex item（连同 gap），下方内容会跳动。组件在打开与关闭两种状态都保留同一个零高度在流占位：打开时由它包住 `deferred` 宿主，关闭后（含退出动画结束）返回同样的零高度占位，因此元素可以一直挂在树里且不改变周围布局。

**退出方向**。`side` 在打开时写入 keyed state，退出期间继续使用该值；调用方在请求关闭的同一帧把 `side` 清掉或回退到默认右侧，也不会让面板在退出动画中跳到另一侧。只有 `open` 为真时才会更新这个锁定值，因此打开期间改变 `side` 仍会立即生效。

**scrim 与面板**。scrim 复用 Dialog 的 `colorBackgroundOverlay`，随后台进度淡入淡出，并调用 `occlude()`：只画在上面不够，没有遮挡命中框时，下面的按钮仍会通过 scrim 收到 hover 边沿。面板是 `neutralBackground1`、`neutralForeground1`、`x_large` 圆角（只圆朝向页面的两角）、`s64` 阴影、24px padding、16px 内容间距、8px footer 间距；面板自身的 `occlude()` 让空白处不落到宿主的关闭层。尺寸是组件常量（默认 320px，`Sheet::size` 可覆盖），左/右为宽、上/下为高。

**动效**。`Presence` 以 `durationSlow`（300ms）与 `curveEasyEase` 驱动进/出；位移量等于 `size × (1 − progress)`，所以任一尺寸都从自身边缘外滑入。`reduce_motion` 时 `Presence` 直接返回稳定帧，不播放过渡。进场中途关闭时 `Presence` 会缩短反向时长而不是跳变。

**状态与关闭**。`.open(bool)` 为受控，`default_open` 只初始化 window keyed store；`on_open_change(false)` 在 Esc / scrim 请求时调用，底层不区分两者。受控调用方不更新 `open` 时抽屉保持打开。

**焦点**。宿主自带焦点陷阱；wrapper 在进入首帧记录 `window.focused` 并把焦点交给宿主句柄，退出动画结束且焦点仍在抽屉内时才归还。**调用方若在 `open(false)` 同帧卸载元素，归还不会发生**——保持挂载并用受控 `open` 驱动。

**尚未覆盖**：真实窗口中的滑入/滑出观感与系统 reduced-motion、固定尺寸下的长内容裁剪、拖拽关闭与滚动区都没有实现或验证；`Role::Dialog` 挂在面板上，AccessKit 的模态语义与焦点顺序留给 T-049 审计。

**命令入口**：`cargo test -p fluent-components --lib sheet --locked`。

## 验证入口与尚未覆盖的场景

在工作区根目录可运行下列定向检查；共享逻辑变化还需遵守[验证矩阵](../code-standards.md#验证矩阵)。这些命令不是本页记录的通过结果。

```text
cargo test -p fluent-components --lib popover --locked
cargo test -p fluent-components --lib sheet --locked
cargo test -p fluent-components --lib tooltip --locked
cargo test -p fluent-core --lib spec::sheet --locked
cargo test -p fluent-core --lib spec::feedback --locked
cargo test -p fluent-core --lib spec::menu --locked
cargo run -p fluent-gallery
```

[Popover 测试](../../crates/fluent-components/src/popover/tests.rs)与[关闭行为测试](../../crates/fluent-components/src/popover/dismissal_tests.rs)中的证据范围：

| 测试入口 | 检查的承诺 |
|---|---|
| `panel_keeps_the_trigger_gap_inside_the_viewport` / `panel_clamps_continuously_at_both_window_edges` / `oversized_panel_aligns_to_the_inset_without_panicking` | 默认间隙、边缘连续限位和小窗口/大面板退化 |
| `panel_tracks_each_scroll_event_in_the_same_frame` | 首次 draw 和滚动事件后的首个 draw 对齐；断言 trigger 确实移动，不靠补帧通过 |
| `panel_tracks_nested_fractional_scroll_and_escapes_clipping` | 嵌套/分数滚动、不同 scale factor、面板内容坐标与裁剪范围 |
| `reopened_panel_uses_the_current_trigger_position` | 测试直接切换受控 `open` 后的位置，不是点击/关闭回调测试 |
| `outside_click_requests_a_close` | 面板外的按下上报 `false` |
| `click_on_panel_surface_keeps_it_open` / `click_on_interactive_child_inside_panel_keeps_it_open` | 面板本体与面板内交互子元素上的按下既不上报关闭，也不穿透到下方元素；移除 `occlude` 后这两条会失败 |
| `clicking_the_trigger_again_toggles_instead_of_double_reporting` | 一次按下触发器只产生一次状态变化；移除“落在触发器内则跳过”的判断后该测试失败 |
| `controlled_caller_that_ignores_the_request_stays_open` | 受控调用方收到关闭请求但未更新值时，面板保持打开 |
| `escape_requests_a_close_while_focus_is_inside` | 焦点在浮层内时 Esc 上报关闭；测试自行绑定 `Dismiss` 并聚焦 |

[Tooltip 测试](../../crates/fluent-components/src/tooltip/tests.rs)只覆盖 service 归属：无 `FluentRoot` 时调用方拿到 `None`、根渲染后 service 可用、实体身份跨重绘稳定。定位、翻转、悬停时机与多窗口隔离不在其中。

**后续行为验证清单**（不是已实现保证；由下一次相关浮层改动认领并记录结果）：

- 受控调用方接受关闭请求后的实际关闭、非受控重绘后保持状态、不同 ID/窗口的隔离。
- Escape 在焦点位于触发器内、面板内和两者之外时的分发结果；焦点归还在窗口失焦、面板被卸载等边界下的表现；嵌套/同时打开的多个浮层。
- 面板在真实窗口中的命中归属（测试用 `canvas` 与垫底元素近似，不覆盖 GPU 字体与原生事件顺序）。
- Sheet 在真实窗口中的滑入/滑出观感、系统 reduced-motion、固定 320px 尺寸下的长内容裁剪，以及关闭动画结束后焦点归还的时序。
- Tooltip 的 hover/隐藏、翻转、滚动跟随、键盘触发及多窗口 service 归属。
- 真实窗口中的边缘、缩放、长内容和滚动；现有测试直接 draw，并不验证 GPU 字体或所有原生事件。

[现有像素夹具](../../crates/fluent-gallery/src/fixtures.rs)已包含 `dialog-open`、`menu-panel` 与 `sheet-right-open` 等静态帧；动画中间帧、真实指针/键盘路径与系统主题仍不在其中。
发现风险时保留复现、被测版本和归属，按[工程规范](../code-standards.md#结果与交付)追踪；本轮文档整理不宣称补齐上述行为。

## 修改后要同步什么

变更坐标来源、布局/绘制阶段、状态归属、事件/焦点语义或验证方法时，同步本页、[总导航](README.md)、公开限制和对应测试。
PR 要描述用户/调用方看到的“之前 → 之后”，并区分实现存在、自动测试证据和原生实测；不将旧记录或别的层次通过当作完成依据。
