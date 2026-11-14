# 焦点：失焦时机与 Tab 顺序

先从[子系统导航](README.md)确认问题类型。本页记录焦点归属、失焦触发与 Tab 遍历的当前机制，以及哪些行为尚未验证。

## 入口与职责

| 入口 | 实际路径 | 不应混同的能力 |
|---|---|---|
| `FluentRoot` | 携带 `ROOT_KEY_CONTEXT`，并处理 `FocusNext` / `FocusPrev` | 不等于 gpui-base 的 `Root` 视图；两者 context 不同 |
| `TextInput` / `Textarea` | 点击自身聚焦；`on_mouse_down_out` 失焦 | 不包含 Tab 遍历与焦点陷阱 |
| `Popover` | `track_focus` 提供面板焦点句柄；打开时移入、关闭时归还；Escape 依赖它；面板注册焦点陷阱 | 触发器的焦点属于触发器自身，不与面板共用句柄；面板内控件仍需自己可聚焦 |
| `Dialog` / `AlertDialog` | 宿主自带 `focus_trap`；wrapper 记录旧焦点并在关闭帧归还 | 打开时焦点落在对话框宿主而非首个控件 |
| `Sheet` | 宿主自带 `focus_trap`；wrapper 记录旧焦点，等退出动画结束且焦点仍在抽屉内时归还 | 打开时焦点落在抽屉宿主；关闭期间焦点仍被约束，不会立刻归还 |

## 失焦时机

**GPUI 不会在点击别处时自动清除焦点**，`gpui-base` 的文本引擎同样不做。这一点容易误判：`window.blur(cx)` 在 gpui-base 中出现的位置都是测试代码，不是实现。

因此“点击别处失焦”是组件自身的责任：

```text
点击输入框内部  → disabled capture guard → 未禁用时 on_mouse_down → state.focus()
点击输入框外部  → on_mouse_down_out → is_focused() 为真时 window.blur(cx)
```

`on_mouse_down_out` 在 capture 阶段触发，判据是“指针不在本元素 hitbox 内”，所以点输入框内部不会误触发。先判 `is_focused` 再 blur，避免对未聚焦的字段做无谓调用。

`TextInput` 与 `Textarea` 都按这个形状实现，两者缺一不可——它们是各自独立的组件，没有共享基类。

## 输入禁用与 Tab 排除

TextInput/Textarea 的显式 disabled builder 同步 state；省略 builder 时使用 state 自己的设置。只在标志变化时写入，避免每帧通知。捕获阶段的 guard 阻止 GPUI 对 track_focus 元素的自动聚焦，不能只跳过自己的 focus 调用。底层 set_disabled 同步持有的 FocusHandle.tab_stop，避免下一帧又用旧句柄克隆注册 Tab 停靠。显式 disabled(false) 可重新启用，恢复 Tab 停靠。

AX role/name 仍在渲染期合并到文本引擎，这是发布单一原生节点的适配；相同投影不重复安装，保留调用方 role/placeholder/id/value_exposed。禁用态向 AccessKit 发布仍受当前平台 API 限制，焦点/编辑回归不等同于屏幕阅读器验证。回归入口为 input::disabled_tests 和 textarea::disabled_tests。

## RadioGroup

组为每项提供持久 FocusHandle，只有选中且可用的项是 Tab stop；没有可用选中项时用首个可用项。方向键随 axis 导航并循环、Home/End 到端点，禁用项跳过，导航同时移动焦点并请求组值改变；受控调用方可以忽略请求，选择不被内部接管。子项 checked builder 由组值覆盖，组外行为不变。

## 浮层焦点交接

`Popover` 在提供 `track_focus` 时按边沿处理焦点：

```text
打开（上一帧关闭）→ 记录 window.focused(cx)，聚焦面板句柄
关闭（上一帧打开）→ 若焦点仍在面板内，交还记录的句柄
```

面板在提供焦点句柄时还会注册焦点陷阱（`gpui-base::FocusTrapElement`），Tab 由 `FluentRoot` 的处理器约束在陷阱内。Esc 在触发器包装层与面板上**都**挂了处理：焦点移入面板后，触发器不再是祖先，只挂一处会收不到。

「仍在面板内」这个条件不能省：用户可能在浮层打开期间点击别处，此时焦点已经移动，关闭时强行归还会把焦点从用户刚点的地方抢走。

保存前一焦点与「上一帧是否打开」都用 window keyed state（按元素 ID 区分），因为 `Popover` 是 `RenderOnce`，自身没有跨帧状态。

**触发器包装层不再跟踪面板句柄。** 之前同一个 `FocusHandle` 既挂在始终存在的触发器包装层上，又用于面板焦点陷阱；关闭归还到的还是这个句柄，而 gpui-base 的 `active_focus_trap` 只检查「陷阱容器的句柄是否包含当前焦点」，注册项又不会随面板卸载删除。于是关闭后陷阱仍被判为活动，`FluentRoot::advance_focus` 把之后的 Tab 反复带回已消失的菜单。现在：

```text
trigger 元素 → 自己的焦点句柄（可聚焦控件）或无焦点
面板 / 陷阱 → track_focus 提供的句柄，只在打开时存在于树中
关闭归还 → 返回 trigger 等上一焦点；面板句柄不再被触发器撑住
```

触发器本身仍需键盘可到达时由调用方提供可聚焦元素（例如 `Button`）；`track_focus` 只描述面板与陷阱，不再把触发器包装层变成空焦点节点。

## Tab 顺序

`gpui_base::root::init` 确实绑定了 `tab` / `shift-tab`，但它限定在 gpui-base 自己 `Root` 视图的 context 上（该常量在 gpui-base 内是私有的）。应用挂载的是 `FluentRoot`，**不会**带上那个 context，所以那条绑定在 Fluent 应用里从不触发。

当前做法：

- `fluent_components` 定义 `FocusNext` / `FocusPrev` 与 `ROOT_KEY_CONTEXT`
- `fluent_ui::init` 把 `tab` / `shift-tab` 绑到这两个 action
- `FluentRoot` 设置 `ROOT_KEY_CONTEXT` 并处理两个 action，转发给 `window.focus_next` / `focus_prev`

不直接复用 gpui-base 的 context，是因为那个字符串是私有实现细节；自己绑定语义明确，也不受上游改动影响。

### 文本控件的 Tab

gpui-base 在 `Input` context 把 Tab/Shift+Tab 绑为 `IndentInline`/`OutdentInline`。Fluent 的 `Textarea` 通过 `TextareaTabBehavior` 选择：

- `Indent`（默认）：保留 gpui-base 的缩进行为。
- `Navigate`：组件包装层设置 `FluentTextInputNavigate` context；`fluent_ui::init` 绑定 `FluentTextInputNavigate > Input` 到 `FocusNext`/`FocusPrev`。`>` 表示标记 context 是 `Input` 的祖先，匹配深度落在 `Input` 上；同深度下后安装的绑定优先，因此仅 Navigate 实例覆盖缩进，Indent 实例和其他输入不受影响。

`TextInput` 单行输入同样应使用 Navigate；它的 builder 与 Textarea 共用同一 context/action 约定。

## 菜单键盘

菜单用「容器持有焦点 + 当前项」的模型，而不是让焦点在行之间移动。这样焦点陷阱仍然掌控 Tab，方向键只在菜单内部移动高亮。

- `Menu::new(id)` 要求显式 id：当前项要跨重绘保存，存在 window keyed state 里。
- 行必须用 `.item()` / `.separator()` / `.header()` 声明。走 `ParentElement::child` 的内容会渲染但**不参与**键盘导航——面板无法知道自定义内容里哪些可选。
- `Menu` 与宿主 `Popover` 必须共用同一个 `FocusHandle`（`Menu::track_focus` + `Popover::track_focus`）。焦点落在菜单容器上时，`MENU_KEY_CONTEXT` 才在派发路径上；只设在 Popover 面板上不行，因为面板是菜单的祖先，而不是它本身。
- 按键绑定限定在 `MENU_KEY_CONTEXT`，所以方向键不会从文本框被抢走。

### 指针与键盘只写一个「当前项」

菜单有子菜单时，每层各有自己的当前行；键盘的激活目标是最深层的当前行，更浅层的当前行只作为路径提示。行上的 `on_mouse_move` **写入该行所属层的当前项**，而不是只改该行样式；回调携带层路径，因此子面板的指针不会写到根层。不能用 `on_hover` 代替：GPUI 在布局变化让另一行移到静止指针下时也会调用它，这会无输入地覆盖当前项。拖动或按住鼠标时不接管。于是：

- 最深层的当前项使用当前项填充；打开更深层的那一行使用独立的 path hint 填充（`spec::menu::path_hint_background`），因此不会与键盘激活目标混淆。
- Enter / Space 激活的永远是最深层当前行。
- 谁最后动谁说了算：指针不动时方向键移动当前层；指针移动时指针接管它所在的层。
- 指针**离开整条菜单链**、或移到**不可选行**（标题、分隔线、禁用行）上时，若该层当前项由指针选中则清除；由键盘选中的不受影响。清空与保留都取决于当前项是谁写的，不能用「每帧清空」来实现。
- 标题与分隔线因此也需要上报指针（各自包一层带 id 的容器）；禁用行同样要上报，但上报的是「清除」而不是「选中我」。否则指针停在这些位置上时，上一行会继续保持「当前项」外观。
- 禁用行不写入当前项，也不能成为激活目标。

菜单之外单独使用 `MenuItem` 时没有上层菜单，此时保留局部 hover 样式（`pointer_activate` 为 `None`），不影响上述规则。

### 输入字符与 Tab

- **typeahead**：菜单在容器上挂 `on_key_down`，把单字符按键累积成前缀（600ms 后重置；重复同一字符时以前一字符重新开始，从而在多个同字母项之间循环）。前缀归属“完整层路径 + 菜单会话 + 本次层级访问代号（`level_epoch`）”，所以同深度换分支、离开再返回、关闭再打开，以及**左退/右进或 Esc 后重入同一分支**都不会继承旧前缀。层级访问代号只在 open 路径实际变化时递增，不在每帧重置，同一层内的连续输入仍正常累积；根层 Esc 与 Tab 不改变路径，会显式清空前缀。匹配从**当前项之后**开始并环绕。
- **Tab / Shift+Tab 关闭并移交方向**：`MENU_KEY_CONTEXT` 内 Tab 绑定 `MenuTabOut`、Shift+Tab 绑定 `MenuTabBackward`。两者都取消未触发的悬停请求、清空 typeahead，并派发对应的 `DismissAndFocusNext` / `DismissAndFocusPrev`。`Popover` 在关闭帧记录方向、归还触发器焦点，之后在该帧之后的首个帧回调（面板已离开 tab 顺序）再调用 `window.focus_next` / `focus_prev`；不能在渲染中立刻前进，因为此时 `rendered_frame` 仍含即将消失的面板。绑定放在菜单上下文里，是为了让更具体的上下文覆盖 ROOT 的 Tab 遍历——否则焦点会先移出菜单，面板留在原处。
- **受控关闭也消费方向**：方向请求会强制产生一帧（`window.refresh`），不依赖调用方是否重绘。受控调用方在同一事件周期把 `open` 改为 false 时，关闭帧消费方向并按 Tab/Shift+Tab 前进；拒绝关闭时，强制帧仍显示打开，方向在该帧丢弃。方向只属于「本请求对应的关闭帧」：之后才异步接受或程序化关闭只归还焦点，不再前进；需要方向性移交的调用方必须同步接受。没有基于时间的过期猜测。
- **没有可归还焦点时**：打开前没有焦点句柄时，关闭后焦点不能留在已卸载的面板上（它没有 dispatch 路径，后续 Tab 绑定也无法匹配）。此时关闭帧在下一帧把焦点移到首个真实停靠点；若 Tab 带来明确方向，则该方向优先。
- **子菜单**：焦点始终留在根容器；`→`/Enter 只在当前行确实带子菜单时打开下一层，`←`/Esc 退一层，Tab 关整条链。每层的当前行按层保存，指针回调带层路径，深层面板没有自己的键上下文或焦点陷阱。相邻层允许复用同一个触发行 ID：打开判据是层路径长度，不是裸 ID。
- **悬停请求的取消**：面板上的真实 `on_mouse_move` 用本层待打开行的测量边界核对指针；移到面板 padding、行间 gap 或自定义非选择内容时取消 `pending`。该检查只响应实际指针移动，布局变化不会触发；它只清 `pending`，不清键盘写入的当前项。

### 当前项是身份，不是序号

状态存的是行的 `ElementId`（每层各一份），不是它在可选行里的位置：

- 前面的行插入、删除或禁用时，只要当前行仍然存在且可用，它保持当前。
- 当前行被删除或禁用时，**没有激活目标**：Enter/Space 不执行任何动作，而不是顺位激活邻居；下一次导航键会重新选一个。

这是刻意的选择。「顺位激活邻居」在没有明确契约时最危险——用户按下的键会执行一个他从未选中的动作。

## 写键盘测试的两个前提

这两点曾让焦点陷阱的测试全部变成空测试，务必记住：

**1. `FocusHandle` 默认不是 tab stop。**

`FocusHandle::new` 里 `tab_stop: false`，而导航会跳过非 stop 的条目。测试里用 `cx.focus_handle()` 建出的句柄**不可达**，Tab 永远不会停在上面——测试看起来通过，其实什么都没验证。必须写：

```rust
let handle = cx.focus_handle().tab_stop(true);
```

**2. tab stop 在绘制时登记，下一帧才对 `focus_next` 可见。**

`track_focus` 的 div 在 paint 阶段写入 `next_frame.tab_stops`，而 `focus_next` 读的是 `rendered_frame.tab_stops`。所以测试必须先聚焦、再绘制两次（或分多次 `update_window` + `run_until_parked`），然后才能派发 Tab。

deferred 面板（Popover 的内容）同样会进入 tab 顺序，不需要特殊处理。

## 验证入口

```text
cargo test -p fluent-components --lib focus_tests --locked
cargo test -p fluent-components --lib tab_order_tests --locked
```

| 测试入口 | 检查的承诺 |
|---|---|
| `pressing_inside_the_field_focuses_it` | 点击字段内部聚焦 |
| `pressing_outside_the_field_blurs_it` | 点击字段外部失焦；移除接线后该测试失败 |
| `pressing_inside_again_keeps_focus` | 再次点击内部不误失焦 |
| `tab_moves_focus_to_the_next_field` | Tab 在两个字段间前进；移除 `FluentRoot` 的 context 后失败 |
| `shift_tab_moves_focus_backwards` | Shift+Tab 反向移动 |
| `tab_advances_between_boxes_in_a_plain_tree` / `..._inside_a_popover_panel` | 普通树与 deferred 面板内容都能参与 Tab 顺序（探针） |
| `tab_cycles_between_the_panel_items` | 面板内 Tab 循环：第一项 → 第二项 → 回到第一项；移除陷阱后失败 |
| `tab_never_reaches_the_element_behind_the_panel` | 连续四次 Tab 都不会逃到面板外；移除陷阱后失败 |
| `shift_tab_wraps_backwards_inside_the_panel` | 第一项 Shift+Tab 回绕到最后一项；移除陷阱后失败 |
| `down_then_enter_activates_the_first_row` 等 6 条 | 菜单方向键/Home/End/回绕/跳过禁用行；断言「哪个行的回调被触发」，因为当前项是视觉状态。移除 `Menu` 的焦点接线后全部失败 |
| `hovering_moves_the_row_that_enter_activates` / `space_also_follows_the_hovered_row` / `keyboard_continues_from_the_hovered_row` | 指针与键盘共用同一个当前项。在基线 `0fa65a4` 上这三条失败 |
| `hovering_a_disabled_row_changes_nothing` / `a_disabled_row_is_never_activated_by_enter` | 禁用行既不能被指针选中，也不能被键盘激活 |
| `redrawing_keeps_the_current_row` | 无输入重绘后当前项仍在（不是每帧清空来规避问题） |
| `disabling_an_earlier_row_keeps_the_current_row` | 前面的行禁用后当前项身份不变。基线 `0fa65a4` 上失败（会顺位激活邻居） |
| `disabling_the_current_row_leaves_no_action` / `removing_the_current_row_leaves_no_action` | 分别禁用和真正删除当前行，验证没有激活目标；删除后没有当前行背景 |
| `removing_an_earlier_row_under_a_stationary_pointer_keeps_identity` / `inserting_before_the_current_row_under_a_stationary_pointer_keeps_identity` | 真实增删行，指针不动时当前身份保持；检查更新后的首次绘制及后续绘制位置和 Enter/Space 激活目标 |
| `keyboard_and_pointer_paint_one_current_row` | 检查真实行背景 quad 的数量、颜色及缩放后的坐标，并验证键盘→指针→键盘→同一行内移动时只有一个高亮且激活一致 |
| `leaving_the_menu_clears_a_pointer_selected_row` | 指针离开菜单后当前项清除，并断言**真实绘制**已无行背景。移除离开清除逻辑后，失败发生在填充断言上 |
| `a_pointer_leave_does_not_wipe_out_a_keyboard_selection` | 指针仅从菜单外经过时不清除键盘选择，绘制断言仍保留该行背景 |
| `keyboard_navigation_after_leaving_starts_from_the_top` | 指针选中被清除后，下一次 Down 从首行重新开始 |
| `hovering_a_header_clears_a_pointer_selected_row` | 指针移到标题行时当前项清除（绘制断言：行填充数 0）。移除标题的指针上报后该测试失败 |
| `a_header_does_not_clear_a_keyboard_selection` | 标题不会清除键盘选择（绘制断言：仍为 1 行填充） |
| `hovering_a_disabled_row_clears_a_pointer_selected_row` | 指针移到禁用行时清除指针选中的当前项，且禁用行本身不被选中；绘制断言由 1 变 0，Enter 无动作。禁用行改回不上报指针后该测试失败 |
| `a_disabled_row_does_not_clear_a_keyboard_selection` | 禁用行不会清除键盘选择（绘制断言：仍为 1 行填充） |
| `opening_moves_focus_into_the_panel` | 打开时焦点移入面板；移除交接逻辑后失败 |
| `closing_returns_focus_to_where_it_was` | 关闭时归还焦点；先断言打开确实移入了焦点，否则该测试在无交接时也会通过 |

## 尚未覆盖

- 焦点陷阱的 Tab 循环已可自动化验证，见下方「写键盘测试的两个前提」。
- 模态语义：`Dialog` 打开时焦点进入宿主、Tab 在卡片内循环、关闭归还，均有行为测试；AX 树测试确认宿主发布 `Role::Dialog`/`Role::AlertDialog` + `aria_modal`，打开期间只保留最上层模态子树、背景节点消失并在关闭后恢复，模态内 deferred Popover/Menu 按 owner reparent。真实 VoiceOver/NVDA 的朗读与导航仍未验证（本机 AX 权限未授予）
- 菜单键盘：子菜单的每层独立 focus scope 未实现（只有根容器一个焦点范围）；根容器已发布单一 `aria_active_descendant`。Esc/→/←/Tab 的逐层行为已有行为测试（`menu::submenu_regression_tests`），但真实窗口中的键盘焦点环与屏幕阅读器播报未验证
- 子菜单的点击归属、逐层绘制、首帧定位、同 ID 相邻触发行与 typeahead 访问代号由 `menu::submenu_tests`、`menu::submenu_regression_tests` 覆盖；这些测试直接派发平台事件，不替代原生窗口验证
- 当前项通过根容器的 `aria_active_descendant` 发布（焦点仍在容器本身），并配合行上的 `aria_selected`；`menu::ax_tests` 检查发布树中的 active descendant 与选中翻转，原生屏幕阅读器的实际朗读效果未验证
- AccessKit role/state 的发布结构已由 `menu::ax_tests`、`dialog::tests`、`sheet::tests` 覆盖；真实屏幕阅读器的朗读与激活仍未验证
- 真实窗口中的 Tab 顺序（测试直接派发按键，不经过平台输入法或原生焦点环）
- 禁用控件是否被跳过（依赖 gpui-base 的 `tab_index`，本项目未设置）

## 修改后要同步什么

改变焦点归属、失焦触发条件、Tab 遍历方式或相关公开 API 时，同步本页、[总导航](README.md)、中英文 README 的对应行与测试。
