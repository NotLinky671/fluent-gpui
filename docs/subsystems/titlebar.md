# 标题栏：窗口行为、命中与浮层层级

阅读入口：[组件与公开 API](../../crates/fluent-components/src/titlebar.rs)、[纯样式 spec](../../crates/fluent-core/src/spec/titlebar.rs)、[Gallery 窗口初始化](../../crates/fluent-gallery/src/main.rs)、[行为测试](../../crates/fluent-components/src/titlebar/tests.rs)。

## 接入与职责

每个窗口顶部放一个 `TitleBar::new(id, title)`，创建窗口时以 `TitleBar::window_options(title)` 为基础，通过结构更新语法覆盖应用自己的 bounds、minimum size、可缩放性等设置。辅助配置启用透明系统标题栏、应用负责标题栏拖拽、Linux 客户端装饰，并为默认 32px 标题栏设置 macOS 红绿灯位置。组件不修改应用的退出模式，不安装或覆盖应用已有的原生关闭回调。

`TitleBarLabels` 提供最小化、最大化、还原、关闭的可访问名称；默认英文，应用可用 `.labels(...)` 本地化。每个自绘按钮有 `Role::Button` 和 `aria_label`；它们不加入内容区域的 Tab 顺序。屏幕阅读器实际朗读与原生窗口按钮的辅助技术激活仍需平台检查。

TitleBar 是窗口顶部 chrome，不是页面内可重复的按钮预览。按钮固定锚定视口右上角；挂载多个 TitleBar 或将其放进页面中间不受支持。保留默认栏高/几何；一般样式细化在默认值之后应用，但改变外层高度或 padding 不会重配原生红绿灯与按钮层的几何。

## 前置图标槽

标题前有一个固定方槽，边长 `spec::titlebar::TitleBarMetrics::icon_size`（16 DIP），后接 `gap`（8 DIP）。三种填充方式共用同一槽位：

| 入口 | 内容 | 说明 |
|---|---|---|
| `TitleBar::icon(FluentIcon)` | 主题着色的 Fluent 字形 | 跟随前景色，和按钮图标同一套 SVG 数据 |
| `TitleBar::logo(Arc<gpui::Image>)` / `logo_bytes(ImageFormat, Vec<u8>)` | 位图或 SVG 品牌图 | 保留自身颜色；`ImageSource::Image` 由 GPUI 在 UI 线程外解码 |
| `TitleBar::icon_slot()` | 空槽 | 只有它会在没有图时也占位 |

槽位是否占用只看两个字段：有 artwork 就画，只有显式 `icon_slot()` 才会在无图时留空。因此**默认不预留**，标题紧接 12 DIP 边距；需要“图标稍后到达也不推动标题”时用 `icon_slot()`。三种状态的标题起点完全一致，由行为测试 `the_icon_slot_holds_the_title_in_place_for_every_kind_of_artwork` 断言。

位图用 GPUI 的默认 `ObjectFit::Contain` 缩放进 16 DIP 方槽：保持宽高比、居中、不裁切，所以非正方形素材会在槽内留边。准备正方形的高分辨率源（例如 32 或 64 的整数倍）即可，绘制时再缩放。

系统窗口图标与标题栏图标是两条通道：`TitleBar::window_options_with_icon(title, Arc<image::RgbaImage>)` 额外设置 `WindowOptions::icon`，把同一份作品交给窗口管理器；调用方仍需自己把相同字节传给 `logo_bytes`，库不会替应用缓存或推导资源。**在本版 GPUI 快照中 `WindowOptions::icon` 只有 X11 后端读取**（写入 `_NET_WM_ICON`）；Windows 的窗口类图标来自可执行文件资源（`load_icon()`），macOS 来自应用包，字段在这两个平台无效。因此 Windows 上任务栏图标需由打包资源提供，标题栏 logo 仍正常绘制。

## 平台事件与关闭

- **Windows**：完整标题栏标记 `WindowControlArea::Drag`，包括文字上下和左侧 padding。按钮独立标记 `Min` / `Max` / `Close`，同时 `occlude()` 遮挡背后的 Drag 命中。Windows 后端将其映射为 `HTCAPTION` / `HTMINBUTTON` / `HTMAXBUTTON` / `HTCLOSE`。不额外挂 `on_click` 调用 minimize、zoom、remove 或 quit；后端会先转发非客户区输入给 GPUI，再执行原生动作，两条路径不能并用。原生 `Max` 负责最大化/还原，字形取实际 `window.is_maximized()`。
- **macOS**：保留原生红绿灯，为其预留 leading 空间；不重复绘制 Windows 三按钮。应用持有拖拽意图，按下并移动时交给 `start_window_move()`；双击调用 `titlebar_double_click()`，遵循用户系统设置。`app_owns_titlebar_drag` 避免 AppKit 再处理一遍。
- **Linux / FreeBSD**：仅在实际获得 `Decorations::Client` 时绘制按钮；服务器端装饰下不重复绘制。按钮集合考虑平台支持的能力以及窗口的 minimizable/resizable 标志；拖拽、双击和右键窗口菜单走 GPUI 平台方法。全屏不绘制 caption 按钮。

Windows/macOS 应用在打开窗口后安装 `window.on_window_should_close(cx, ...)`，返回 `false` 可以保留窗口以便保存或确认。Linux 自绘关闭按钮用 `.on_close(|window, cx| -> bool { ... })` 作同样的否决；Linux 窗口管理器发起的原生关闭仍由 `on_window_should_close` 检查。需要两条入口时，调用方应复用自己的同一个关闭决策。`on_close` 不覆盖原生 hook。

允许的自绘关闭只移除当前窗口。是否退出遵循 GPUI `QuitMode`：当前快照默认在非 macOS 最后一个窗口关闭后退出，macOS 默认保持应用；组件不调用 `cx.quit()`。

## 绘制与实际命中

| 表面 | deferred 优先级 | 意图 |
|---|---|---|
| 普通内容、标题栏拖拽区、Sheet | 默认 | 内容区域与模态背板 |
| Dialog | `10 + layer` | 普通模态层 |
| 标题栏按钮 | `50` | 保持窗口按钮能在普通模态之上操作 |
| Popover / MenuDropdown | `gpui_base::POPUP_PRIORITY`（100） | 菜单覆盖窗口按钮时先选择菜单项 |
| 根级 Tooltip | gpui-base tooltip 优先级（200） | 提示在 popup 之上 |

按钮行从流中移出，标题栏提前预留固定宽度，长标题通过 `min_w_0` 与 truncate 收缩，按钮不参与 flex 收缩。遮挡命中由按钮 `occlude` 与 popup 自己的 `occlude` 实现；单纯改变绘制顺序不等于事件隔离。标题栏按钮优先级不是无限大，调用方自定义的更高优先级层仍可覆盖它。

图形统一用一个 10px ink box。最大化/还原轮廓是一条**矢量描边路径**（`PathBuilder::stroke` + `arc_to` 圆角），不是 `paint_quad`：后者对边框宽度按设备像素取整，150% 下会把 1 DIP 描边和 1.5 DIP 半径都推到 2 设备像素，内侧圆角退化为直角、整块变重。圆角半径取 1.75 DIP，是对照 WinUI 参考图逐像素量出来的：150% 下参考图的角部覆盖为角格 1/9、次格 7/9，1.5 DIP 时为 2/9 与 9/9（仍偏方），1.75 DIP 时为 0–1/9 与 7/9，基本重合。还原图形后层只裁出顶部和右侧，避免用实心背景遮盖。关闭 X 也用描边路径。绘制时读取当前文字前景色，避免 render 时固定图标颜色覆盖 hover 白色。外接边界和定位按窗口 scale factor 对齐物理像素，但描边宽度与半径保留分数设备像素交给抗锯齿；最小化横线继续沿用原来的物理像素线宽。颜色与度量在 spec 中解析，包含 hover/pressed 和独立于应用 palette 的 Windows 关闭红色。

## 验证边界

```text
cargo test -p fluent-core --lib spec::titlebar --locked
cargo test -p fluent-components --lib titlebar --locked
cargo test -p fluent-components --lib popover --locked
cargo run -p fluent-gallery
```

spec 测试检查几何与跨主题状态颜色。行为测试检查 native 路径没有手动关闭、fallback 可否决且保留其他窗口、caption 与 modal/popup 的实际输入归属、hover/pressed 背景的绘制输出，以及最大化轮廓的几何：内圆角必须存在（半径大于描边），且描边不允许到达方形外角。最后一条在半径为 0 或描边被取整吃掉内圆角时失败。测试平台不执行 Windows 非客户区动作，不能用这些绿灯证明拖动、Snap、最大化/还原、真实退出或 macOS/Linux 的原生行为。

原生检查应覆盖：标题左 padding 和文字上下的拖拽/双击；三个按钮的命中、最大化与还原；关闭检查否决；多窗口关闭；打开 Dialog/Sheet 后的窗口按钮；菜单覆盖时的点击；light/dark/high-contrast、窄窗口、长标题、100%/150%/200% DPI。Mica 与便签应用接入另行处理。运行结果见[标题栏修复记录](../validation/titlebar-2026-11-10.md)。
