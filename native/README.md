# 原生预览版

Rust + Slint 原生版 v2.0.5 是当前主版本。旧 Electron 源码与 v1.0.3 Release 保留；下文预览记录用于追溯开发过程。当前发布 Windows x64 便携包，尚未完成的功能和平台验证见“当前范围”。

v2.0.5 新增“已推迟”进展状态。已有数据若包含该状态，需要用 v2.0.5 或更新版本打开；升级时保留程序旁的 `data/`。

v2.0.4 为数据统计的计数列留出滚动条空间，9 项统计、900px 窄窗口的修复前后截图已验证。

v2.0.3 固定招聘进度筛选栏高度，搜索无匹配结果时不再拉高搜索框和按钮。已通过正常及 900px 窄窗口的空结果截图验证。

v2.0.2 将 Slint 固定到包含 Windows 重绘修复的上游提交；用户实测首次打开和托盘恢复不再出现空白。该提交属于 1.19.0 开发版，跨显示器移动尚未实测，详见 `docs/SLINT_REPAINT_TRIAL.md`。

本机已构建 Windows 预览程序 `native/release/autumn-native.exe`，可直接启动。Windows Release 使用图形子系统启动，不附带控制台窗口；开发用 Debug 构建保留控制台输出。2026-10-27 测得 9.69 MB（9.24 MiB），不含系统库；这不是安装包，也不是最终功能体积。

## 开发

安装 Rust stable 和平台编译工具后，在仓库根目录执行：

```sh
cargo test --manifest-path native/Cargo.toml
cargo run --manifest-path native/Cargo.toml
cargo run --manifest-path native/Cargo.toml -- --smoke-test
cargo build --release --manifest-path native/Cargo.toml
```

Windows GNU 工具链需要 MinGW 的 `bin` 在 PATH 中。本机旧版 MinGW 对中文构建输出路径存在兼容问题，可在 PowerShell 中设置纯英文缓存目录后执行以上命令：

```powershell
$env:CARGO_TARGET_DIR = "$env:USERPROFILE/.cache/autumn-native-target"
```

设置后产物位于该目录的 `debug/` 或 `release/` 下。其他机器可使用正常的 `native/target/`。

## 数据隔离

默认使用可执行文件旁边的 `data/` 文件夹，与启动时所在工作目录无关。点击“数据目录”可打开它；`--data-dir DIR` 仍可显式指定其他位置。程序目录需要有写入权限。

移动程序时，请一起移动同目录的 `data/`，其中包含当前数据、自动备份及迁移备份。开发构建的数据位于实际 Debug EXE 旁，`--smoke-test` 仍使用临时目录。

2026-10-28 已将本机此前 C 盘原生预览版的数据与备份完整复制到 `native/release/data/`；原目录保留为迁移备份，之后的编辑以程序目录数据为准。

显式导入旧 JSON 副本（目标目录必须尚无 applications.json）：

```sh
cargo run --manifest-path native/Cargo.toml -- --data-dir native/.dev-data --import-legacy native/tests/legacy.json
```

示例使用虚构测试数据。导入保留源文件，并保存 `legacy-import.original.json`；后续保存保留上一次内容到 `.bak`。遇到损坏数据、外部修改或同目录并发写入时拒绝覆盖。

`--smoke-test` 始终使用临时目录，执行界面回调与保存检查后自动退出，不接受旧数据导入。它不能替代真实鼠标操作、中文输入法或托盘测试。

## 当前范围

已实现投递和进展的新增、编辑、删除，搜索、状态/未完成筛选、全部记录、月历、阶段统计、汇总及托盘入口。具体通过的验证以 `docs/REWRITE_PLAN.md` 为准。

进展可标记为已完成、待进行、已失败或已推迟。“已推迟”保留旧场次记录，不计入“有未完成进展”，也不会仅因改期让投递显示为“已结束”；新场次应单独添加为“待进行”。

日历按进展日期高亮，支持月份跳转（1900–2100）、今天、搜索、仅看未完成及当天详情。缺少有效日期的节点单独提示数量。阶段统计按投递去重：同一岗位的多轮面试只算一次，统计包含所有状态。

尚需完成开机启动、再次启动恢复窗口，以及三平台实机验收。Linux 预览版暂不启用关闭隐藏，避免缺少托盘宿主时无法恢复窗口。

## 旧版外观还原

界面已按旧版样式重绘，范围和开发截图入口见 `docs/NATIVE_UI.md`。本轮新版单独保存为 `native/release/autumn-native-ui.exe`；请先从托盘退出正在运行的预览版，再打开新文件。当前更新版默认使用该 EXE 旁的 `data/`。

2026-10-28 的 UI 还原版实测 9.95 MB（9.49 MiB），Release 冒烟测试通过，仍为无控制台窗口的 Windows GUI 程序。

## 启动没有打开窗口时

同一新版数据目录只允许一个实例写入。旧预览版可能仍驻留在托盘，必须通过窗口中的“退出”或托盘菜单退出；点击窗口关闭按钮通常只是隐藏。新版会用提示窗口显示目录占用、数据损坏等启动错误。
