# 无障碍与键盘清单（T-049 产出，2026-11-13 更新）

日期：2026-11-13。基线：`aa1cf6c` + 本次 AX/键盘整合改动（未提交）。

> 任务文件里写的是 `plan/a11y-checklist.md`，但 `plan/` 是被 git 忽略的本地笔记。
> 按项目规范，长期有效的规则必须进版本控制，因此清单放在这里。
> 本页记录当前契约与验证入口，不是本轮测试通过记录；实际运行结果见 PR/验证记录。

## 怎么核对

- **role / aria**：来源分两类——gpui-base 提供的组件（Button、CheckBox、Switch、Dialog、Tooltip）与自研组件（Menu、MenuItem、TabList、Tab、ProgressIndicator、Link、TextInput/Textarea 默认角色、Sheet 面板）。核对命令：

  ```bash
  grep -n 'Role::\|aria_' crates/fluent-components/src/*.rs
  grep -rn 'Role::' vendor/gpui-base/src/
  ```

- **AX 树**：行为测试通过 `window.debug_a11y_tree_json()` 读取发布树，不查 getter 回显。入口：
  `input::accessibility_tests`、`textarea::ax_tests`、`menu::ax_tests`、`menu::cjk_consumer_tests`、`dialog::tests`、`sheet::tests`。
- **键盘**：`cargo test -p fluent-components --lib menu --locked`、`--lib tabs`、`--lib sheet`、`--lib popover`、`--lib dialog`。
- **焦点归还**：`popover/focus_probe_tests.rs`、`popover/focus_trap_tests.rs`、`dialog/tests.rs`。

## 逐组件

| 组件 | role | aria | 键盘 | 焦点可见 | 状态 |
|---|---|---|---|---|---|
| `Button` | 基类 `Button` | `accessibility_label` 必须显式提供；可见文字不会被自动推导为名称 | 基类 click/Enter | `focus_visible` 内描边 | 已实现；真实屏幕阅读器未验证 |
| `Checkbox` / `Radio` | 基类 `CheckBox` / `RadioButton` | Radio label、selected/toggled；组内有 position/size | 基类 | 基类 | 继承 |
| `RadioGroup` | `RadioGroup` | 名称与 orientation | 方向键/Home/End 跳过禁用项并移动焦点，roving Tab stop | 基类 Radio | 互斥、受控与状态保持有行为/AX 测试；真实屏幕阅读器未验证 |
| `Switch` | 基类 `Switch` | 基类 | 基类 | 基类 | 继承 |
| `TextInput` | 无 builder 也默认 `Role::TextInput`；已有 `InputAccessibility` 的 role/label/placeholder/value_exposed 保留，builder 只改名称 | `accessibility_label` 仅在显式提供时合并名称 | 原生 IME/光标 | 聚焦下划线动画 | 已实现；真机 IME 修复后复测待做 |
| `Textarea` | 无 builder 也默认 `Role::MultilineTextInput`；已有 accessibility 状态保留 | `accessibility_label` 只改名称 | 原生 IME/光标；Tab 默认 Indent，可显式 `Navigate` | 聚焦下划线动画 | 已实现；真机 IME 修复后复测待做 |
| `Field` / `Label` | 无 | 不自动关联；调用方给输入控件 `accessibility_label` | — | — | 调用方负责命名 |
| `Menu` / `MenuItem` | 每个面板 `Role::Menu`，行 `Role::MenuItem` | 面板名默认 `Menu`，可用 `accessibility_label` 覆盖；行有 `aria_label`、`aria_selected`；当前行作为 focused 容器的 active descendant | 方向键/Home/End/Enter/Space/typeahead/→ 子菜单/Esc 逐层/Tab 关闭 | 最深当前项填充 + 父行 path hint | 已实现（单焦点范围，见 [menus.md](subsystems/menus.md)） |
| `MenuDropdown` | 继承 `Menu` | 继承，真实 Popover 面板 | 同上 + Esc/Tab | — | 已实现；模态内打开时按 owner 挂到模态子树 |
| `TabList` / `Tab` | `TabList` / `Tab` + `aria_selected` | 无 label | 方向键/Home/End | 选中指示条；聚焦不加外框 | 已实现（模型差异见缺口 2） |
| `Popover` | 无 role（容器） | — | Esc/外部点击 | — | 非模态；不隐藏背景，也不得当作模态处理 |
| `Dialog` | 宿主 `Role::Dialog`（`AlertDialog` 为 `Role::AlertDialog`） | 宿主 `aria_modal(true)`；名称默认取 `title` 文本，`accessibility_label` 覆盖，`title_element` 不提供名称；不同 Dialog 使用不同 host id | 主机焦点陷阱 + Enter/Esc | 卡片内 | 已实现；打开时 AX 树只保留最上层模态子树，背景节点消失，关闭恢复；模态内 deferred popup 按 owner reparent |
| `AlertDialog` | 宿主 `Role::AlertDialog` | 同上 | 无 Esc/无 backdrop 关闭 | — | 已实现 |
| `Sheet` | 面板 `Role::Dialog` + `aria_modal(true)` | 名称默认取 `title`，`accessibility_label` 覆盖 | Esc / scrim 关闭，Tab 陷阱 | 宿主 | 已实现；背景 AX 隔离与关闭恢复有树测试 |
| `Tooltip` | 基类 `Tooltip` | 文本内容 | — | — | 继承 |
| `TitleBar` 自绘按钮 | `Button` | `TitleBarLabels`（可本地化） | 不加入内容 Tab 顺序，原生系统快捷键另测 | — | 部分实现；真实屏幕阅读器/辅助技术激活未验证，见[标题栏专题](subsystems/titlebar.md) |
| `ProgressBar` / `Spinner` | `ProgressIndicator` + `aria_label` | — | — | — | 已实现 |
| `Link` | `Link` | — | — | 下划线 | 已实现 |

### 模态与 deferred 浮层的 AX 契约

- `aria_modal(true)` 是显式门槛；只有它触发背景裁剪，普通 Popover 不带 modal。
- 同时存在多个模态时，按真实绘制序选择最上层：deferred priority + push order；不是按节点深度或 render 顺序。
- 模态打开期间，AX 树只发布最上层模态子树及其真正 owned 的 deferred Popover/Menu；背景菜单、背景 popup、被覆盖模态一律删除。
- 属于模态的 deferred popup 在发布树中 reparent 到 owner 节点下，不再是 ROOT 的兄弟；owner 被裁掉时随 owner 删除。
- 关闭/切换模态后，背景节点与关闭模态的节点按树恢复；`aria_hidden` 子树在隐藏期间不发布。

## 缺口（如实记录，未修）

1. **真实屏幕阅读器未验证**。AX 树测试证明发布结构，不等于 VoiceOver/NVDA 的朗读、导航顺序或激活行为；本机 `AXIsProcessTrusted()` 为 false 时不得写通过。
2. **Tabs 不是 roving tabindex**。整条 strip 持焦点、方向键改选中；严格来说 `role=tab` 要求 tab 自身可聚焦。行为可用，但屏幕阅读器语义不完整。
3. **禁用态没有暴露给 accesskit**。本版 GPUI 没有对应 API（`MenuItem` 源码里已注明）；禁用项在无障碍树里与普通项相同。
4. **没有通用 live region**。保存/删除等状态变化不会主动播报；菜单当前项通过 active descendant 发布 AX 焦点更新，但实际朗读未测。GPUI 当前无通用 live-region API。
5. **焦点环覆盖不全**。Button 有焦点环，Menu 当前项有填充，Input 有下划线；Tabs 按设计仅保留选中指示条，无独立焦点外框。Checkbox/Radio/Switch 依赖基类绘制，未逐项核对。
6. **`aria_hidden` 当前 API 限制**。抑制子树依赖带 role/id 的容器产生发布节点；没有 role/id 的包裹层不会抑制后代。使用方应把 `aria_hidden` 放在带 `role` 与 `id` 的容器上；本轮不扩大 API 去修这个限制。
7. **`title_element` 不提供名称**。自定义标题必须另给 `accessibility_label`，否则宿主无名。
8. **`Dialog::title` 已改为 `SharedString`**。需要自定义元素时用 `title_element`；旧 `IntoElement` 调用方必须 clean cutover。

## 键盘与焦点归还（已回归入口）

| 项 | 证据 |
|---|---|
| 弹层打开移入焦点、关闭归还 | `popover::focus_probe_tests`、`dialog::tests`、`sheet::tests` |
| Tab 在面板内循环、不逃逸 | `popover::focus_trap_tests`、`sheet::tests::tab_stays_inside_the_sheet`（移除陷阱后失败） |
| 菜单方向键/Home/End/Enter/Space/跳过禁用 | `menu::keyboard_tests`、`menu::input_mode_tests` |
| 菜单 typeahead、Tab 关闭 | `menu::typeahead_tests` |
| 子菜单逐层导航、点击归属、首帧定位、typeahead 路径 | `menu::submenu_tests`、`menu::submenu_regression_tests`、`popover::dismissal_tests` |
| Tabs 方向键/Home/End/跳过禁用 | `tabs::tests` |
| 中文输入 committed text 只进 typeahead、不激活行 | `menu::cjk_consumer_tests`（真实平台输入句柄） |

## 复跑

```bash
cargo test -p fluent-components --lib input::accessibility_tests --locked
cargo test -p fluent-components --lib textarea::ax_tests --locked
cargo test -p fluent-components --lib menu::ax_tests --locked
cargo test -p fluent-components --lib menu::cjk_consumer_tests --locked
cargo test -p fluent-components --lib menu --locked
cargo test -p fluent-components --lib tabs --locked
cargo test -p fluent-components --lib sheet --locked
cargo test -p fluent-components --lib popover --locked
cargo test -p fluent-components --lib dialog --locked
```

## 未验证

- 任何真实屏幕阅读器（NVDA / VoiceOver）的实际朗读与导航；本机 AX 权限未授予
- 键盘完整路径下的 pressed/长按/取消（按住移出后释放）
- dark / high-contrast 下的对比度
- 修复后的中文 IME 真机复测（`aa1cf6c` 的用户通过记录属于修复前/源输入基线）
