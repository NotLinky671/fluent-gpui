# M2 像素基线（T-050）

日期：2026-11-04。基线：`bdcf1b9` + 未提交的 Tabs / 审计改动。

## 做了什么

在 `crates/fluent-gallery/src/fixtures.rs` 增加 4 个 M2 fixture，并用 `FLUENT_BASELINE_UPDATE=1` 生成基线：

| fixture | 画布 | 内容 |
|---|---|---|
| `tabs-horizontal` | 340×60 | 三个标签，第三个禁用 |
| `tabs-vertical` | 220×140 | 竖排三个标签，选中中间那个 |
| `menu-panel` | 240×220 | 静态 `Menu`：标题、两行、分隔线、禁用行、危险行 |
| `dialog-open` | 420×260 | `Dialog` 打开态：scrim + 卡片 + Cancel（Secondary） |

生成命令：

```
FLUENT_BASELINE_UPDATE=1 cargo test -p fluent-gallery --features screenshot --test baselines
```

产出 `crates/fluent-gallery/tests/baselines/windows/light-{tabs-horizontal,tabs-vertical,menu-panel,dialog-open}.png`，Windows 基线总数从 15 增至 19。

## 动画组件怎么处理

`MenuDropdown` / `Popover` 的面板在打开时有淡入（首帧不透明度为 0），把首帧当基线等于存一张空白图，没有回归价值。因此：

- 菜单基线用**静态 `Menu`**（放在页面里的菜单不播放 reveal，见 `Menu::reveal_generation` 的接线），画面即最终态
- 打开动效本身由 `menu/reveal_tests.rs` 的行为断言覆盖（首帧透明、350ms、位移与淡入同曲线），不靠像素图
- `Popover` / `MenuDropdown` 的锚定面板仍未纳入像素基线：它们在 deferred 里，单帧截图能否稳定呈现需要在有编译环境时确认

## 截图核对

- `dialog-open`：scrim 变暗、卡片圆角与阴影、`Cancel` 有 1px 边框 —— 与官方一致
- `menu-panel`：单层表面、标题、分隔线、禁用行灰化、危险行红色

## 发现的一处偏差（未改，仅记录）

`menu-panel` 基线显示：**没有图标的行，文字相对标题向内缩了一段**（保留了 16px 导引槽）。官方 Fluent 在整组都没有图标时不会留这个槽。这不属于 T-050 的范围，改动会影响已有菜单外观，因此记录待确认。

## 验证状态

| 项 | 结果 |
|---|---|
| 基线生成（更新模式） | ✅ 通过，4 张新基线 |
| 基线比对（`cargo test --test baselines`） | ✅ 通过（34.7s，1 passed）——阻塞解除后复跑 |
| `cargo fmt --check` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅（先修掉我自己引入的未使用 `Popover` 导入） |
| `cargo test --workspace` | ✅ 241 passed、0 failed |
| `cargo doc --workspace --no-deps` | ✅ |
| 图库启动冒烟 | ✅ exit 0 |
| README 组件表 + 已知限制 | ✅ 中英文均已加入 `Tabs` 行与 Tabs 限制说明 |
| gallery M2 分区 | ✅ 已有 `tabs` 分区；`menu` / `popover` / `dialog` 在 `misc` 与 `dialog` 分区 |

## 下一步

1. 等并行会话修好 `sheet.rs`（或确认归属），重跑：基线比对、fmt、clippy、test、doc
2. 决定是否修正菜单无图标时的导引槽
3. 决定是否把 `Popover`/`MenuDropdown` 静态面板纳入基线

## 阻塞期间的补充核对（不依赖编译）

在 `sheet.rs` 修好之前，用图像工具直接核对四张新基线：

| fixture | PNG | 逻辑尺寸 | 缩放 | 非空白像素 |
|---|---|---|---|---|
| `tabs-horizontal` | 510×90 | 340×60 | 1.5 | 4.6% |
| `tabs-vertical` | 330×210 | 220×140 | 1.5 | 2.7% |
| `menu-panel` | 360×330 | 240×220 | 1.5 | 7.4% |
| `dialog-open` | 630×390 | 420×260 | 1.5 | 80.8%（含全屏 scrim） |

尺寸与 fixture 声明一致、纵横比与 DPI 缩放一致、无空白图。这只证明产物正确，**不代替**基线比对测试。
