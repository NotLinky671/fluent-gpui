# 项目审查整改验证（2026-11-14）

## 代码与范围

基准 d5742c1，整改内容以承载本记录的 Git 提交为准。审查第 4 条（释放到控件外的 pressed 取消）和第 6 条（发布策略）按用户要求跳过；发布 manifest、依赖版本与下游 patch 策略未改。其余条目在本记录追踪。本地检查在对应工作区完成；GitHub CI 按实际运行链接与 head SHA 单独报告，不以本地通过替代三平台结果。

Windows / Rust 1.98.0。组件行为通过 TestAppContext；像素通过本机原生 GPU surface。macOS/Linux CI 配置已补，不能据此声称两平台已经运行通过；真机 IME、辅助技术、跨显示器及其他 DPI 仍未验证。

## 行为与兼容性

- TextInput/Textarea：disabled builder 原先只改外壳，现在同步文本引擎、阻止默认指针聚焦、跳过 Tab 并拒绝编辑。未给 builder 保留 state 设置，显式 false 可重新启用。角色/名称仍作幂等投影，不额外创建 AX 节点。
- Button/Icon：禁用/动态前景色和调用方文本颜色传给实际 SVG；默认图标读取周围文字样式。
- Theme：custom HighContrast 使用固定可访问 token，品牌保留供切回；系统 light/dark 更新不覆盖手动 HighContrast。
- RadioGroup：新增组级互斥值、受控/非受控模式、方向键/Home/End、roving Tab stop、分组 AX 角色/名称/方向和子项位置。组内覆盖子项 checked/default_checked，组外 Radio 不变；新增 Radio size 及 RadioProps.size，直接构造 spec props 的调用方需给 size 或使用 Default。
- 共享 interaction 只抽出现有指针状态更新与内部释放接线，未实现本次跳过的外部释放取消。Checkbox/Radio/Switch 的 on_change 类型复用。
- Menu 拆出会话归一化、键盘/IME 接线与显现阶段；Gallery 按页面职责拆模块。保留原来的状态归属和绘制顺序。
- Switch/Checkbox/Radio 几何、Popover 视口边距/间隙/表面、根字号由 spec 输出。motion 测试模块移至文件尾，移除过期任务号并修正 Textarea 名称优先级文档。
- 截图在三帧图像相等后保存，最多 120 帧或 20 秒；每个子进程 30 秒截止并回收；新运行前删除旧 actual，stdout/stderr 分别保存。图像相等仅证明当前静态图像稳定，不证明交互或未来资源状态。
- CI 普通检查使用 locked，macOS 增加原生启动，Linux 用 Xvfb/Mesa 启动两应用；仍不运行像素比较。
- vendor 补丁从 SHA-256 校验的原版 archive 导出，并在临时目录重建全部保留文件；不升级依赖。工程规则统一到 code-standards，AGENTS/PR 模板仅导航和记录。

## Radio 基线溯源

首次像素检查 19/20 通过，radio-checked 相对旧基线差 72px（0.267%），差异集中在圆点。隔离 worktree 检出原始 d5742c1，用旧的 1200ms 截图路径重拍，得到：原始代码与整改代码图像完全一致（0px 不同）；原始代码与旧基线同样差 72px。历史 f1fb8e8 已将圆点改为 10px，基线仍来自更旧实现。

因此仅更新 light-radio-checked.png 到原始已提交实现的输出。未扩大容差，未批量重录。相关本地证据在 target/review/original-radio.png、original-radio.log 及 Gallery target/baselines 的 actual/diff；这些临时文件不随仓库分发。

## 执行记录

完整日志保存在本地 target/review/final-*.log；日志不随 clone 分发，命令与结果在此保留。

| 命令/检查 | 结果 | 证据与范围 |
|---|---|---|
| cargo fmt --all --check | 通过 | final-fmt.log |
| cargo clippy --workspace --all-targets --locked -- -D warnings | 通过 | final-clippy.log |
| cargo test --workspace --locked | 通过 | 367 个单元测试及 4 个 doctest 通过，1 个原有 doctest ignored；final-test.log |
| cargo doc --workspace --no-deps --locked | 通过 | final-doc.log |
| cargo deny check | 通过，带重复版本警告 | advisories/bans/licenses/sources ok；final-deny.log |
| cargo run -p xtask --locked -- gen-tokens --check | 通过 | final-tokens.log |
| cargo run -p xtask --locked -- gen-icons --check | 通过 | final-icons.log |
| cargo test -p fluent-gallery --features screenshot --test baselines --locked | 通过 | 20 张 Windows/light 基线 + 子进程超时/陈旧截图检查；3 个测试通过；final-baselines.log |
| FLUENT_GALLERY_SMOKE=1 cargo run -p fluent-gallery --locked | 通过 | 原生窗口启动并正常退出，exit 0；final-gallery.log |
| FLUENT_NOTES_SMOKE=1 cargo run -p fluent-notes --locked | 通过 | 原生窗口启动并正常退出，exit 0；final-notes.log |
| python scripts/vendor-patches.py | 通过 | SHA-256 校验与重建 gpui-base 212 / gpui-pre 104 项；vendor-patches.log |
| 修改文档中的本地文件链接及 git diff --check | 通过 | 无失效链接或空白错误 |

定向破坏：将两个输入组件的 disabled builder 写成恒 false，禁用同步/指针聚焦/Tab 排除的 6 条测试失败；将图标改回 rest.foreground，图标绘制断言失败；将 HC custom 改用 dark token 且移除系统跟随的 HC 保留策略，两条主题测试失败。随后恢复正确源码。编辑拒绝测试进一步使用平台 commit_text，并用启用字段接受同样文本作为阳性对照；最终这组 14 条测试通过。三个 mutation 日志保存在 target/review/mutation-*.log。

## 已调查的红灯

- 新增禁用测试的按键辅助函数需要 static 字符串；用 WindowHandle.update 派发事件会重复借用根 entity，改用 update_window。
- 禁用指针聚焦测试证明 GPUI track_focus 默认聚焦需要 capture guard；底层 tab_stop 需改持有句柄。Textarea 的 Tab 导航测试显式选 Navigate，避免把默认缩进误当作跳过失败。
- 模块提取遇到读取截断，改为分段读完整原文件重新提取；编译发现的可见性/trait 导入/测试参数错误均已逐项修正。
- 原始提交对照构建复用了工作区 target，曾导致当前测试链接旧 spec/GPUI 产物，出现字段/API 缺失。已保留该失败日志，只清理相关本地包的编译产物后从当前源码重建；这轮检查不计通过，最终结果取重建后完整运行。隔离比较 worktree 已清理。
- 首次新截图调用 request_animation_frame 时不在渲染阶段，触发 GPUI 断言；已移除此调用，使用 on_next_frame 自带的调度。后续只剩上面已溯源的 Radio 旧基线差异。
- cargo deny 首次因受限环境中用户公告数据库路径只读而无法加锁；同一审计命令获准在受限环境外重跑，advisories/bans/licenses/sources 全部通过，保留依赖重复版本警告，不修改策略或依赖来静默压制。
- vendor 重建中的 Python 临时目录在受限 Windows token 下被拒绝。诊断工具检查未作权限修改或回滚；随后该验证命令获准在受限环境外运行，逐文件重建 gpui-base 212 项和 gpui-pre 104 项通过。无需修改源码目录权限。

## 未验证与负责人

维护者：本仓库 maintainer。macOS/Linux smoke 的真实 CI 结果、两平台原生窗口和屏幕阅读器、不同 DPI/字体/reduced-motion 全矩阵待维护者验证；这些验证影响跨平台发布验收，不影响本地已经明确运行的命令结论。第 4/6 条保留为本次明确跳过项，后续由维护者另行安排。
