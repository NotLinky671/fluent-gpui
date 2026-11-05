# 菜单树：数据、状态与事件协议

本文规定嵌套菜单（子菜单）的数据模型、导航状态与事件语义。实现见
[crates/fluent-components/src/menu.rs](../../crates/fluent-components/src/menu.rs)，
视觉与点击见同文件；浮层定位与外部点击见 [overlays.md](overlays.md)。

> 状态：**切片 1、2、3 已实现**。真实窗口的命中、外部点击与缩放验证未做。

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

- `MenuItem.submenu: Option<Box<Menu>>`，因此子菜单可以是任意 `Menu`：item、separator、header、custom 都保留。
- 子菜单**不自带导航状态**：打开哪一层、每层的当前行，全部由根 `Menu` 持有。
- 子菜单不自带 `FocusHandle`、不重复建键上下文；键盘动作只由根菜单解释。

### 为什么不用闭包

闭包不是障碍：`FnOnce() -> Menu` 可以在构建本帧菜单树时调用一次、读出 `entries`。
选择数据树是为了**统一管理整棵树**（导航、逐层状态、路径失效降级都在一处），
而不是为了绕过闭包的限制。

### 无障碍边界（明确声明）

整条菜单链只有一个焦点范围：焦点始终在根菜单容器上，`role=menu` 只在根节点，
子面板不构成独立 focus scope。屏幕阅读器不会把子面板播报为子菜单。
这是本次范围选择；真正分层的焦点范围（每层 `FocusHandle` + 嵌套陷阱）不在本次范围，
也不能用"T-049 已登记缺口"当作排除它的理由——T-049 未覆盖子菜单语义。

## 2. 状态

```rust
struct MenuState {
    /// 根层当前行，沿用现有 CurrentRow（含 from_pointer）。
    root: Option<CurrentRow>,
    /// 打开的子菜单路径，从根的子菜单开始，逐层向内。
    open: Vec<OpenLevel>,
}

struct OpenLevel {
    /// 打开这一层的父行 ID（在它所在层里的身份）。
    parent: ElementId,
    /// 这一层自己的当前行。
    active: Option<ElementId>,
}
```

### 身份与降级

- 路径按**行 ID**保存，不按索引。插入、删除、禁用都不改变已保存路径的含义。
- 打开前与每次渲染前校验路径；任一段失效就**截断到最后一个有效前缀**：
  - 父行已不存在 → 截断该层及其后全部；
  - 父行已禁用 → 同上（禁用的行不能再打开子菜单）；
  - 父行已没有子菜单 → 同上；
  - 某一层的 `active` 指向已失效的行 → 该层 `active = None`，但**保留该层本身**（层还开着，只是没有当前行）。
- 路径有效但末层 `active` 为 `None` 时，方向键从该层第一行开始，而不是回到根层。

### typeahead

- 只作用于**最深层**的行列表。
- 每次换层（打开、关闭、截断）都清空前缀；同一层内的 600ms 重置规则不变。
- 实现上把层深度纳入 typeahead 状态，深度变化即视为过期。

## 3. 动作表

| 动作 | 绑定 | 结层（`open` 为空） | 有子菜单打开 |
|---|---|---|---|
| `MenuNext` / `MenuPrev` | ↓ / ↑ | 根层移动 | 最深层移动 |
| `MenuFirst` / `MenuLast` | Home / End | 根层首尾 | 最深层首尾 |
| `MenuActivate` | Enter / Space | 当前行有子菜单则**打开**，否则执行处理函数 | 同左（作用于最深层） |
| `MenuSubmenuOpen` | → | 当前行有子菜单则打开 | 已打开则无操作 |
| `MenuSubmenuClose` | ← | **无操作**（根层不响应） | 退一层，当前行留在父行 |
| `MenuEscape` | Esc | 派发 `Dismiss`（关整个菜单） | 退一层，**不**派发 `Dismiss` |
| `MenuTabOut` | Tab / Shift+Tab | 派发 `Dismiss` | 派发 `Dismiss`（关整条链） |

要点：

- **Esc 与 Tab 语义不同**，不能共用"弹栈"。Tab 无论几层都关整条链；Esc 只在根层关整个菜单。
- 因此新增 `MenuEscape`，Esc 不再直接绑 `Dismiss`；`MenuTabOut` 保持派发 `Dismiss`。
- 打开子菜单时，把该层 `active` 设为第一行可选行（与 Fluent 一致：右开即进入子菜单）。

## 4. 指针协议

- 指针移动只写**最深层**的当前行（与键盘同一条状态），不写根层。
- 指针从父行移向子面板时经过两面板之间的空隙：**不关闭**。只有指针移动到**同一层的另一行**时，才关闭更深层。
- 子面板是菜单容器的后代元素，因此外部按下判定（`Popover` 的 `on_mouse_down_out` + `occlude`）把它们视为"内部"：点击子面板不关根菜单。此点依赖既有行为测试（面板及其交互子元素上的按下不关闭），实现时必须实测验证，不能只靠结构推断。
- 延迟打开（悬停约 300ms）不靠定时器：待打开项 `PendingOpen { row, deadline }` 存在状态里，渲染时比对截止时间；指针移到别的行会**替换或清除**它，所以过期的截止时间不可能打开已经离开的分支。（文档早先写作"代次"，实现用替换/清除达到同一保证。）
- 指针在延迟窗口内离开该行：取消该行的待打开任务（同样靠代次）。

## 5. 定位

- 子面板相对于**打开它的那一行**定位：左边缘贴该行右边缘，顶边与该行顶边对齐。
- 行边界不在渲染期可得，沿用本项目既有做法：`on_children_prepainted` 记录绘制边界，下一帧用于定位（晚一帧，过渡不可见）。
- 右侧空间不足时翻转到左侧；垂直方向钳制在窗口内。
- 首帧：边界未知时不绘制子面板（避免在 (0,0) 闪一帧），第二帧起按测量值定位。

## 6. 测试清单（每项都要在旧实现或定向破坏后失败）

**行为（GPUI）**

1. `activating_a_submenu_child_runs_that_child`：打开子菜单 → 导航 → Enter 执行的是子项处理函数，父项处理函数未被调用。
2. `escape_pops_one_level`：两层打开后 Esc → 子面板消失、根菜单仍在；再 Esc 才关根菜单。
3. `tab_closes_the_whole_chain`：两层打开后 Tab → 整条链关闭。
4. `left_returns_to_the_parent_row`：← 退层后当前行仍是原来的父行。
5. `a_press_inside_the_submenu_keeps_the_root_open`：在子面板内按下 → 根菜单不关闭。
6. `the_submenu_opens_at_the_trigger_row_edge`：首帧之后子面板左边缘不小于父行右边缘。
7. `a_removed_parent_row_collapses_the_open_path`：打开子菜单后把父行从菜单里去掉 → 路径截断回根层，不崩。
8. `a_disabled_parent_row_cannot_open_a_submenu`：禁用行的 → 与 Enter 都无效。
9. `typeahead_starts_fresh_after_changing_level`：根层输入 `a` 后进入子菜单再输入 `b`，匹配的是以 `b` 开头的子项。

**纯逻辑**

10. 路径截断函数（输入路径 + 树，输出截断结果）单独可测，不依赖窗口。

## 7. 分片

- **切片 1**：数据模型 + 状态 + `→`/`←`/Esc/Tab + 渲染与首帧定位；测试 1、2、3、4、6。
- **切片 2**：悬停延时打开与取消（代次）；测试 5、7、8。
- **切片 3**：指针协议与外部点击、typeahead 换层、边缘翻转；测试 9 与真实窗口验证。

## 8. 实现现状（切片 1）

已实现：

- `MenuItem::submenu(Menu)`，`submenu: Option<Box<Menu>>`；带子菜单的行画右向箭头，子菜单被取走后箭头仍保留。
- 状态 `MenuState { root, open: Vec<OpenLevel> }` 存在 window keyed state，键 `menu-tree-<菜单 id>`；每次渲染前 `normalize_state` 截断失效路径。
- 键盘：`MenuSubmenuOpen`（→）、`MenuSubmenuClose`（←）、`MenuEscape`（Esc，分层）、`MenuTabOut`（Tab，整链）。Enter/Space 遇到带子菜单的行等同于打开。
- 渲染：子面板是菜单容器的后代，按上一帧记录的触发行边界定位；首帧没有边界就不画，避免在面板原点闪一下。
- 子面板复用同一套行渲染与样式，但没有自己的键上下文、动作和 reveal。

切片 3 补上的部分：

- 子面板内的行也写树状态（指针与键盘共用同一份当前行）。
- 换层清空 typeahead 前缀：`Typeahead.depth` 记录前缀所属层，深度变化即清空。
- 子面板放置：`submenu_placement` 在右边缘放不下时翻到行的左侧，垂直方向钳制在视口内（尺寸取上一帧测量值）。

仍未做：

- 真实窗口下的命中、外部点击（点击子面板是否被 Popover 判为内部）、缩放与长内容验证。

### 几何测试的调整

原计划用绘制四边形断言"子面板在触发行右侧"，但菜单面板背景与窗口背景同色，
按颜色筛选无法区分，该断言实际只能拿到窗口背景。改为把偏移量抽成纯函数
`submenu_offset(row, panel)` 并单测（两条：贴右边缘、随面板原点相对计算），
真实几何留到切片 3 的原生窗口验证。

### 切片 2 的测试

- `hovering_a_submenu_row_opens_it_after_the_delay`：鼠标移到带子菜单的行 → 推进 400ms → Enter 执行**子项**（证明子菜单自己打开并接管键盘）。
- `hovering_another_row_cancels_the_pending_open`：悬停 150ms 后移到另一行 → 再等 400ms → Enter 执行**另一行**（证明取消生效）。

定向破坏：去掉渲染期触发 → 第一条失败；悬停别的行不清除待打开 → 第二条失败。

### 切片 3 的测试

- `typing_restarts_when_the_keyboard_enters_a_submenu`：根层输入 `t` 选中 Two，进入子菜单后输入 `b` 必须匹配 Beta Two（不清空前缀会变成 `tb`，落到 Beta One）。
- `the_submenu_stays_right_when_it_fits` / `the_submenu_flips_left_when_it_would_leave_the_window` / `the_submenu_clamps_vertically`：放置的纯函数。

定向破坏：去掉换层清空前缀 → 第一条失败。
