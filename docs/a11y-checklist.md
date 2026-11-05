# 无障碍与键盘清单（T-049 产出）

日期：2026-11-04。基线：`bdcf1b9` + 未提交的 Tabs 改动。

> 任务文件里写的是 `plan/a11y-checklist.md`，但 `plan/` 是被 git 忽略的本地笔记。
> 按项目规范，长期有效的规则必须进版本控制，因此清单放在这里。

## 怎么核对

- **role / aria**：来源分两类——gpui-base 提供的组件（Button、CheckBox、Switch、Dialog、Tooltip）与自研组件（Menu、MenuItem、TabList、Tab、ProgressIndicator、Link）。核对命令：

  ```bash
  # 自研组件
  grep -n 'Role::\|aria_' crates/fluent-components/src/*.rs
  # 基类
  grep -rn 'Role::' ~/.cargo/registry/src/*/gpui-base-0.8.1/src/
  ```

- **键盘**：`cargo test -p fluent-components --lib menu --locked`、`--lib tabs`、`--lib sheet`、`--lib popover`、`--lib dialog`。
- **焦点归还**：`popover/focus_probe_tests.rs`、`popover/focus_trap_tests.rs`、`dialog/tests.rs`。

## 逐组件

| 组件 | role | aria | 键盘 | 焦点可见 | 状态 |
|---|---|---|---|---|---|
| `Button` | 基类 `Button` | `accessibility_label` builder（2 处调用） | 基类 click/Enter | `focus_visible` 内描边 | ✅ |
| `Checkbox` / `Radio` | 基类 `CheckBox` | 基类 `aria_checked` | 基类 | 基类 | ✅ 继承 |
| `Switch` | 基类 `Switch` | 基类 | 基类 | 基类 | ✅ 继承 |
| `TextInput` / `Textarea` | gpui 原生输入元素 | — | 原生 IME/光标 | 聚焦下划线动画 | ⚠️ 见缺口 1 |
| `Menu` / `MenuItem` | `Menu` / `MenuItem` + `aria_selected` | 无 label（行文本即可访问名） | 方向键/Home/End/Enter/Space/typeahead/Tab 关闭 | 当前项填充 | ✅ |
| `MenuDropdown` | 继承 | — | 同上 + Esc/Tab | — | ✅ |
| `TabList` / `Tab` | `TabList` / `Tab` + `aria_selected` | 无 label | 方向键/Home/End | 选中项 2px 内描边 | ✅（模型差异见缺口 2） |
| `Popover` | 无（容器） | — | Esc/外部点击 | — | ✅ |
| `Dialog` | 基类 `Dialog` | — | 主机焦点陷阱 + Enter/Esc | 卡片内 | ✅ |
| `AlertDialog` | 基类 `AlertDialog` | — | 无 Esc/无 backdrop 关闭 | — | ✅ |
| `Sheet` | 面板 `Role::Dialog`，宿主无 role | — | Esc / scrim 关闭，Tab 陷阱 | 宿主 | ⚠️ 见缺口 6 |
| `Tooltip` | 基类 `Tooltip` | 文本内容 | — | — | ✅ |
| `ProgressBar` / `Spinner` | `ProgressIndicator` + `aria_label` | ✅ | — | — | ✅ |
| `Link` | `Link` | — | — | 下划线 | ✅ |
| `Field` / `Label` | 无 | — | — | — | ⚠️ 见缺口 1 |

## 缺口（如实记录，未修）

1. **文本控件与标签没有关联**。`Field` 只做视觉排布，不把 `Label` 的文本写进输入元素的 accessible name；`TextInput`/`Textarea` 也没有 `aria_label` builder。屏幕阅读器读不到字段名。
   修法需要给输入组件加 `aria_label`（或让 `Field` 注入），但 gpui 原生输入元素的 aria 接口未验证，属独立改动。
2. **Tabs 不是 roving tabindex**。整条 strip 持焦点、方向键改选中；严格来说 `role=tab` 要求 tab 自身可聚焦。行为可用，但屏幕阅读器语义不完整。
3. **禁用态没有暴露给 accesskit**。本版 GPUI 没有对应 API（`MenuItem` 源码里已注明）；禁用项在无障碍树里与普通项相同。
4. **没有 live region**。对话框打开、菜单选中变化都不会主动播报；GPUI 无此 API。
5. **焦点环覆盖不全**。只有 Button、Tabs、Menu 当前项、Input 下划线；Checkbox/Radio/Switch 依赖基类绘制，未逐项核对。
6. **Sheet 的模态语义未验证**。`Role::Dialog` 设在面板上，焦点实际落在 gpui-base 宿主 div；背景内容是否在 accesskit 树里被标记为不可交互、Esc/关闭请求是否会重复播报，均未验证。

## 键盘与焦点归还（已回归）

| 项 | 证据 |
|---|---|
| 弹层打开移入焦点、关闭归还 | `popover::focus_probe_tests`、`dialog::tests`、`sheet::tests` |
| Tab 在面板内循环、不逃逸 | `popover::focus_trap_tests`、`sheet::tests::tab_stays_inside_the_sheet`（移除陷阱后失败） |
| 菜单方向键/Home/End/Enter/Space/跳过禁用 | `menu::keyboard_tests`、`menu::input_mode_tests` |
| 菜单 typeahead、Tab 关闭 | `menu::typeahead_tests` |
| Tabs 方向键/Home/End/跳过禁用 | `tabs::tests` |

## 复跑

```bash
cargo test -p fluent-components --lib menu --locked
cargo test -p fluent-components --lib tabs --locked
cargo test -p fluent-components --lib sheet --locked
cargo test -p fluent-components --lib popover --locked
cargo test -p fluent-components --lib dialog --locked
```

## 未验证

- 任何真实屏幕阅读器（NVDA / VoiceOver）的实际朗读
- 键盘完整路径下的 pressed/长按/取消（按住移出后释放）
- dark / high-contrast 下的对比度
