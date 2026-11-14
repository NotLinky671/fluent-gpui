# Radio / Switch 圆心与缩放回归

日期：2026-11-14。基线：`a774880`。被测实际 HEAD 为 `a774880af88ed937ccc069c4990a578d4169ce16`，加本次未提交的圆形绘制、几何测试、Gallery 夹具、四张基线及文档改动；无依赖或 vendor 修改。

## 问题与结果

用户在 Gallery 的选择控件和 Switch 中再次发现内圆偏移。此前只调整名义直径，未验证最终物理像素圆心，因此无法排除非整数 DPI 的边缘独立取整。

- Radio：旧实现让外环与内圆各自参与布局和 quad 边缘取整；150% 下，外环 24px、内圆 15px，会产生半像素偏移。原代码几何测试实际得到外环 (12,12) / 24×24、内圆 (16,16) / 15×15，圆心相差 -0.5px。
- Switch：名义 offset 是相对于完整轨道的坐标，却被用于 border 内的 padding。旧代码 100% small/off 的轨道 (8,8) / 32×16、滑块 (12,11) / 10×10，左间隙 4px、上间隙 3px。
- 新实现：canvas 保留外环/轨道的布局与样式，paint 时从其最终边界计算圆心；共享[圆形路径](../../crates/fluent-components/src/circle.rs)保留非整数设备像素边缘。Switch 将原 motion channel 的 offset 转成两端帽圆心之间的进度，动画中间帧沿同一水平轴移动。
- 尺寸、颜色、状态归属、公开 builder 和回调不变。Switch 实际端点位置修正；`SwitchSpec::thumb_rest_offset` 的说明纠正为 border-box offset。每个内圆仅构建固定四段三次曲线，无 IO、输入扫描或依赖变更；未进行性能基准。
- 同类：Checkbox 的方形勾选标记、Spinner 的弧线没有这一内圆 flex 定位路径，本次不改。

## 回归与原生证据

[几何测试](../../crates/fluent-components/src/selection/geometry_tests.rs)读取实际 painted quads / paths，兼容旧 quad 渲染与新 path 渲染；验证圆形直径、宽高、纵向圆心以及 Radio 横向圆心 / Switch 端点间隙。三种主题、100%/125%/150%/200%、small/medium/large、8/9px 起点及启用/禁用共覆盖 144 个 Radio 与 288 个 Switch 帧。Switch 覆盖 on/off；Radio 检查选中内圆。关闭动画以直接测量终态，不能据此声称动画或交互手工矩阵全部通过。

原生环境：Windows 10.0.26200 x86_64，rustc 1.98.0 (88d9e12ae 2026-08-18)，当前 Windows 原生渲染后端；显示器 150%（240×50 逻辑像素夹具输出 360×75）。字体来自当前 Fluent 默认配置，未替换；未跨字体或 GPU 做比较。

原生 light 截图：`target/review/radio-checked-{before,after}.png`、`switch-on-{before,after}.png`、`switch-off-{before,after}.png`。新增 `selection-geometry` 夹具同时展示默认/large Radio、默认/small Switch、on/off 和部分禁用选中态；其 light/dark/high-contrast 原生截图均已查看，位于 `target/review/selection-geometry-{after,dark,hc}.png`。截图均等待连续三帧稳定。

原生 Radio 内圆的红通道覆盖量质心，在外环内部同一裁剪区测得：修复前约 (132.472,37.472)，修复后约 (131.948,36.947)，外环圆心 (132,37)。约 0.47px 偏移降至约 0.05px；剩余误差属于该图像的覆盖量测量与栅格采样，非所有渲染环境的误差保证。单一二值阈值会受抗锯齿影响，不能代替覆盖量测量或实际绘制坐标。

## 检查记录

本地日志均在被忽略的 `target/review/`，日志前缀 `circles-`。

| 检查 | 结果 | 证据与边界 |
|---|---|---|
| 旧代码定向测试 | 预期失败 | `circles-before-tests.log`；Radio 150% 偏心，Switch 100% 端点留白不对称 |
| 修复后的几何测试 | 通过 | `circles-after-tests.log`；2 项测试，432 帧组合 |
| cargo fmt --all --check | 通过 | `circles-fmt.log` |
| cargo clippy --workspace --all-targets --locked -- -D warnings | 通过 | `circles-clippy.log` |
| cargo test --workspace --locked | 通过 | `circles-test.log`；369 单测 + 4 doctest，1 项原有 ignored |
| cargo doc --workspace --no-deps --locked | 通过 | `circles-doc.log` |
| cargo run -p xtask --locked -- gen-tokens --check | 通过 | `circles-tokens.log` |
| cargo run -p xtask --locked -- gen-icons --check | 通过 | `circles-icons.log` |
| cargo test -p fluent-gallery --features screenshot --test baselines --locked | 通过 | `circles-baselines-final.log`；21 张 Windows/light 基线及子进程生命周期，共 3 项测试 |
| cargo deny check | 未完成 | `circles-deny.log`；沙箱拒绝公告库的外部锁文件，提权请求等待超时，未获得本次审计通过结果 |

首次基线比较的差异仅为 Radio 60px、Switch on 80px、off 79px，以及新增夹具缺少基线。已查看 actual/diff；只替换这三张因圆心/端点修复而有意变化的图像，并增加一张多尺寸图像，其余 17 张未修改。容差不变，未启用全量重录；最终以正常比较模式重跑通过。

验证过程出现的编译红灯已修复：初始测试误用 Pixels 私有字段；去除生产代码不用的 px 导入后，旧 group_tests 依赖父模块的隐式导入，现改为显式导入 px。保留旧代码几何失败与首次基线红灯日志。最终代码检查已重新执行。

中英文 README、子系统绘制路径、工程规范中的基线范围和本地任务笔记已同步。未验证 macOS/Linux 原生渲染、真实显示器 100%/125%/200%、原生 hover/pressed 与动态交互矩阵；其他缩放的证据来自 GPUI 测试窗口。后续跨平台/审计复核由项目维护者安排，不阻止本次 Windows 几何修复的本地检查，但不能把它写成三平台或依赖审计通过。

提交状态：未提交、未推送。
