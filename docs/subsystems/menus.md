# 菜单树：数据、状态与事件协议

本文规定嵌套菜单（子菜单）的数据模型、导航状态、点击归属与事件语义。实现见
[crates/fluent-components/src/menu.rs](../../crates/fluent-components/src/menu.rs)；
浮层定位与外部点击的共享部分见 [overlays.md](overlays.md)，焦点模型见 [focus.md](focus.md)。

> 状态：**子菜单的逐层导航、点击归属与定位已实现**。下列行为有 GPUI 行为测试与绘制断言；
> 真实窗口中的命中、缩放与长内容仍未人工验证（见文末“边界”）。

## 1. 数据模型

子菜单是**内容**，不是回调：

```rust
MenuItem::new("export", "Export").submenu(
    Menu::new("export-menu")
        .item(MenuItem::new("export-pdf", "PDF").on_click(..))
        .separator()
        .item(MenuItem::new("export-png", "PNG").on_click(..)),
)
```

- `MenuItem.submenu: Option<Box<Menu>>`，子菜单可以是任意 `Menu`：item、separator、header、custom 都保留。
- 嵌套层数不受组件限制：每个打开层级都参与渲染，子菜单里的行也可以再带子菜单。
- 子菜单**不自带导航状态**：打开哪一层、每层的当前行，全部由根 `Menu` 持有。
- 子菜单不自带 `FocusHandle`、不重复建键上下文；键盘动作只由根菜单解释。
- 行 ID 只要求在**所属层内**唯一。不同层可以复用同一个 ID，因为状态与测量都按“层路径 + 行 ID”区分。
- 子菜单的 `Menu::style` 会应用到该层面板；`Menu::track_focus` 在子菜单上没有意义（焦点始终属于根容器）。

### 为什么不用闭包

闭包不是障碍：`FnOnce() -> Menu` 可以在构建本帧菜单树时调用一次、读出 `entries`。
选择数据树是为了**统一管理整棵树**（导航、逐层状态、路径失效降级都在一处），
而不是为了绕过闭包的限制。

### 无障碍边界（明确声明）

整条菜单链只有一个焦点范围：焦点始终在根菜单容器上，子面板不构成独立 focus scope，
屏幕阅读器不会把子面板播报为独立的子菜单焦点区域。**每个面板本身仍带 `role=menu`**
（根面板与子面板都是），因为它们是独立的菜单弹出层；差异在焦点模型，而不是 role。
根容器有可访问名（`Menu::accessibility_label`，默认 `Menu`），每个行发布 `Role::MenuItem`、
`aria_label` 与 `aria_selected`，当前行通过根容器的 `aria_active_descendant` 暴露。
子面板不各自声明 active descendant。真正分层的焦点范围（每层 `FocusHandle` + 嵌套陷阱）不在当前范围。

## 2. 状态

```rust
struct MenuState {
    /// 根层当前行。
    root: Option<CurrentRow>,
    /// 打开的子菜单路径，从根的子菜单开始，逐层向内。
    open: Vec<OpenLevel>,
    /// 指针悬停后等待延时打开的请求。
    pending: Option<PendingOpen>,
    /// 当前下拉会话（reveal generation）。
    session: Option<u64>,
    /// 层级访问代号：open 路径每次增删/截断都递增。
    level_epoch: u64,
}

struct CurrentRow { id: ElementId, from_pointer: bool }
struct OpenLevel { parent: ElementId, active: Option<CurrentRow> }
struct PendingOpen { path: Vec<ElementId>, row: ElementId, deadline: Instant }
```

- 每层当前行都带来源：`from_pointer`。指针离开时只清指针写入的行；键盘选中的行保留。
- `PendingOpen.path` 是**包含该行的层的完整路径**，不是裸行 ID 或深度。同一行 ID 出现在别的层时不会误命中。
- **身份是“层路径 + 行 ID”，不是全树唯一的裸 ID。** 打开下一层只检查 `path.len() == open.len()`；
  不能拿 `open.last().parent` 与当前层行 ID 比较——那是两个不同层的行，允许同名。
  相邻两层可以使用同一个触发行 ID，仍能继续展开第三层。
- `session` 记录宿主下拉的打开代号。**关闭再打开**会开始新会话：旧路径、旧悬停请求与旧 typeahead 前缀都不复活。
- `level_epoch` 记录“这次访问该层”的身份。只在 open 路径实际变化（打开、关闭、截断）时递增，
  不随每帧渲染变化，因此不会打断同一层内的连续输入。

### 归一化是真实状态

`normalize_state(state, levels)` 在**每次渲染和每个事件读取前**执行，并在确实变化时写回 window keyed state：

- 父行已不存在、已禁用或已没有子菜单 → 从该层起截断，连同更深路径。
- 某层 `active` 指向已失效的行 → 该层 `active = None`，但保留该层本身。
- `pending` 的路径必须与当前 open 路径完全一致，且行仍可用并带子菜单；否则清除。
- 仅在状态实际变化时写回并通知；渲染期写回不重复 `notify`，因此不会形成每帧重绘循环。

因此绘画、方向键、Enter、Esc、typeahead 和指针读取的是**同一份有效状态**，
不是“渲染副本修好了、事件还看旧路径”。

### typeahead

- 只作用于**最深层**的行列表。
- 前缀归属于“完整层路径 + 会话 + 本次层级访问代号”，不只是深度或最终路径：
  同深度换分支、离开再返回、关闭再打开、以及**左退/Esc 后重新进入同一分支**都会重新开始。
  仅比较路径不够：退层再进同一分支时最终路径和 session 都没变，旧前缀会被拼成新前缀。
- 层级切换结束时不会每帧清空前缀；它记录 `level_epoch`，下一次输入发现访问代号变化才丢弃旧前缀。
  根层 Esc 与 Tab 不改变 open 路径，因此这两条关闭动作会显式清空前缀，避免受控宿主忽略关闭后继续旧前缀。
- 同一层内仍保持 600ms 重置与“重复同一字符循环匹配”的规则。

## 3. 动作表

| 动作 | 绑定 | 根层 | 有子菜单打开 |
|---|---|---|---|
| `MenuNext` / `MenuPrev` | ↓ / ↑ | 根层移动 | 最深层移动 |
| `MenuFirst` / `MenuLast` | Home / End | 根层首尾 | 最深层首尾 |
| `MenuActivate` | Enter / Space | 当前行有子菜单则**打开**，否则执行处理函数 | 同左（作用于最深层） |
| `MenuSubmenuOpen` | → | **仅当**当前行存在、可用且带子菜单时打开 | 已打开则无操作；叶子行无操作 |
| `MenuSubmenuClose` | ← | 无操作 | 退一层，当前行回到父行 |
| `MenuEscape` | Esc | 派发 `Dismiss`（关整个菜单） | 退一层，**不**派发 `Dismiss` |
| `MenuTabOut` | Tab | 派发 `DismissAndFocusNext` | 派发 `DismissAndFocusNext`（关整条链后向前移交） |
| `MenuTabBackward` | Shift+Tab | 派发 `DismissAndFocusPrev` | 派发 `DismissAndFocusPrev`（关整条链后向后移交） |

要点：

- **Esc 与 Tab 语义不同**：Tab 无论几层都请求关闭整条链并移交焦点方向；Esc 只在根层关整个菜单，不移动焦点。
- 打开子菜单时把该层 `active` 设为第一行可选行。
- 叶子行、禁用行、没有当前行时，→ 不改变状态，不会生成不可见的“伪层级”。
- 键盘动作会清除尚未触发的悬停请求：按键接管后旧 hover 不得再打开分支。
- Tab 只清除悬停请求并派发 `Dismiss`；受控宿主若忽略关闭请求，显示仍由调用方的 `open` 决定。

## 4. 指针协议

- 行的指针回调携带**所属层路径**（根层为空路径），不是只带行 ID。
- 指针写入它所在层的当前行，然后按协议截断更深分支：
  - 移到同层的另一行（含禁用行、标题、分隔线）→ 关闭更深层；
  - 移到“当前正打开更深层的那一行”（锚点行）→ 保留现有分支；
  - 从锚点行移入已打开子面板 → 层路径更深，不关闭任何分支。
- 锚点行用独立的 **path hint** 填充（`spec::menu::path_hint_background`），
  不使用当前项填充。这样最深层的当前项始终唯一，Enter 的目标不会与父行混淆。
- 悬停延时打开用状态里的 `PendingOpen { path, row, deadline }`，渲染时比对截止时间；
  任何其他行、其他层、关闭一层/整链、键盘接管、路径失效或新会话都会替换或清除它。
- **实际 MouseMove 离开待打开行也取消请求**，即使指针仍在同一面板：面板收到的鼠标移动会用本层当前帧测得的
  待打开行边界核对位置；落在 padding、行间 gap、自定义非选择内容上时清除 `pending`。
  静止指针下的布局变化不会触发该检查，因此不会误清键盘当前项。
- 指针离开**某个面板**时，只取消属于该面板层路径的 `pending`（即使仍在菜单链内、例如移到父面板）；
  指针离开**整条链**时，再清除指针写入的各层当前行、剩余 `pending` 和 typeahead 前缀；
  键盘写入的当前行不受影响。判断“是否仍在链内”使用已绘制面板的窗口边界，
  而不是“子面板是后代元素”的结构推断。从父行进入已打开子面板是合法移动，不关闭现有分支。

### 内部点击归属

子面板通常超出 Popover 面板自身的矩形，Popover 仅按自己的 hitbox 判断“外部按下”会误关根菜单。
当前机制：

1. `Menu` 每帧重建整条链的面板边界，写入共享的 `InteriorRegions`（按层索引的窗口坐标）。
2. `MenuDropdown` 把同一个集合同步给 `Popover`。
3. `Popover` 的外部按下处理先检查该集合；落在任一已绘制子面板内就视为内部，不上报关闭。
4. 每个子面板调用 `occlude()`，因此点击/悬停不会穿透到背后控件。
5. 点击整条链外仍按普通 Popover 语义请求关闭；触发器点击仍只上报一次变化。

这不是“后代关系就能保证”：面板的可视边界、命中遮挡和 Popover 的关闭判据是三条不同路径，必须分别接线。

## 5. 渲染与定位

- **所有有效 open 层级都渲染**；每层有自己的面板 ID、行集合、当前行、行测量与子菜单位置。
  不存在“键盘能到、屏幕上看不见”的层。
- 每个子面板由 `SubmenuPositioner` 放置：
  - 面板先参与布局，同一帧 `prepaint` 取得自身尺寸；
  - 锚点行的边界在**同一帧**由其测量包装器写入，位置器在该行之后 prepaint；
  - 因此首次打开、切换分支和布局变化后的**首帧**就能按真实尺寸决定右开/左翻与纵向钳制，
    不需要“晚一帧再补”。
  - 面板贴锚点行右边缘、顶边对齐；右侧放不下翻到行左侧；两轴都钳制在视口内。
  - 位置写入共享区域集合，供内部点击与离开判断使用。
  - 若锚点行在本帧确实未被测量（正常路径不应发生），面板按当前布局位置绘制并请求下一帧，
    而不是留下一个可激活但不可见的层。
- 子面板复用同一套行渲染与样式，但没有自己的键上下文、动作和 reveal 动画。

## 6. 测试入口

行为测试都在 `crates/fluent-components/src/menu/` 下，通过真实窗口事件与绘制结果断言：

- **基础子菜单**（`submenu_tests.rs`）：子项激活、Esc 逐层、Tab 整链、← 回父行、悬停延时与取消、
  typeahead 换层、放置纯函数。
- **缺陷回归**（`submenu_regression_tests.rs`）：
  1. 子菜单内点击只执行一次、无关闭请求、不穿透到背后控件；
  2. 外部点击请求关闭、触发器不重复上报、受控拒绝保持显示；
  3. 打开子菜单后悬停根层兄弟，Enter 执行兄弟；
  4. 父层兄弟子菜单间切换时路径、path hint、面板与激活目标一致；
  5. 三层菜单全部绘制，最深层当前项与回调一致；
  6. 跨层复用同一行 ID 不混淆；
  7. 删除/禁用父行或移除 submenu 后真实路径截断，Esc 按当前有效层处理；
  8. 离开整链、移到标题/分隔线/禁用行、键盘接管都取消旧 pending；
  9. 叶子行 → 不创建层级，随后根层 Esc 关闭；
  10. Esc/← 逐层退回，Tab/Shift+Tab 一次关闭整链；
  11. typeahead 同深度换分支、离开再返回不继承前缀；
  12. 关闭再打开不复活过期悬停请求；
  13. 首次打开/切分支/窗口边缘/布局变化后的首帧定位与填充断言；
  14. 相邻两层触发行使用同一 ID 时，→、Enter 与悬停都能展开第三层；
  15. ←/→ 或 Esc 后重新进入同一分支，typeahead 从新输入重新匹配；
  16. 同一层内连续输入仍形成多字符前缀；
  17. 移到面板 padding、行间 gap、自定义非选择内容都取消 pending 且不绘制子菜单；
  18. 静止停在触发行仍按延时打开；padding 上的指针移动不清除键盘当前项。
- **Popover 共享逻辑**（`popover/dismissal_tests.rs`）：内容发布的区域算内部、
  未重新发布的旧区域失效、区域外点击仍关闭。
- **spec**（`crates/fluent-core/src/spec/menu.rs`）：当前项填充与 path hint 填充不同。

命令：

```text
cargo test -p fluent-components --lib menu:: --locked
cargo test -p fluent-components --lib popover:: --locked
cargo test -p fluent-core --lib spec::menu --locked
```

第一轮六项缺陷与第二轮三个边界问题的复现、定向破坏、命令结果与未验证范围见
[第一轮验证记录](../validation/menu-submenu-six-fixes-2026-11-05.md)与
[第二轮验证记录](../validation/menu-submenu-second-pass-2026-11-05.md)。

## 7. 边界与未验证项

- **真实原生窗口**中的子面板命中、跨面板移动、窗口边缘翻转与逐层关闭尚未人工截图验证；
  自动测试直接派发平台事件并检查场景 quad，不覆盖 GPU 字体、DPI 与原生事件顺序。
- 一个焦点范围：子面板不是独立 focus scope；只有根容器发布一个 `aria_active_descendant`，不是每层各有一个。
- 本版 GPUI 未提供 accesskit 的禁用状态；禁用行在无障碍树里与普通行相同。
- 子菜单数量很大或每层行很多时，每帧重建测量与边界集合的成本尚未做性能测量；
  当前按小规模菜单设计。
- `Popover` 的 `InteriorRegions` 是 crate 内部接线，不是公开 API；外部自定义内容若也要在面板外
  绘制可点击区域，需要另行设计公开能力。
- 长内容、滚动容器内的子菜单与多窗口隔离不在本轮验证范围。
