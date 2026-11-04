# 原生版

Rust + Slint 原生版 v2.0.8 是当前主版本。旧 Electron 源码与 v1.0.3 Release 保留；下文预览记录用于追溯开发过程。当前发布 Windows x64 便携包，尚未完成的功能和平台验证见“当前范围”。

v2.0.6 新增运行时本机 API 和可点击的备注网址，并修复长备注下进展流程线断开的问题。

v2.0.7 让日历详情和招聘流程的进展行按内容调整高度；岗位详情的正文右侧预留 24px，避免滚动条遮挡日期和删除按钮。长备注仍按可用宽度换行；含长备注的 Debug 截图已检查布局。

v2.0.8 将招聘进度的整张投递卡片缩小 24px，以免边框与滚动条相交；全部记录的表头和行内容也预留 24px，保持对齐。两页含滚动条的虚构数据截图已检查。

v2.0.5 新增“已推迟”进展状态。已有数据若包含该状态，需要用 v2.0.5 或更新版本打开；升级时保留程序旁的 `data/`。

v2.0.4 为数据统计的计数列留出滚动条空间，9 项统计、900px 窄窗口的修复前后截图已验证。

v2.0.3 固定招聘进度筛选栏高度，搜索无匹配结果时不再拉高搜索框和按钮。已通过正常及 900px 窄窗口的空结果截图验证。

v2.0.2 将 Slint 固定到包含 Windows 重绘修复的上游提交；用户实测首次打开和托盘恢复不再出现空白。该提交属于 1.19.0 开发版，跨显示器移动尚未实测，详见 `docs/SLINT_REPAINT_TRIAL.md`。

当前 Windows 便携版可从 [v2.0.8 发布页](https://github.com/pmssl/autumn-recruitment-dashboard/releases/tag/v2.0.8)单独下载 EXE，或下载包含同一 EXE 和说明文件的 ZIP。本机默认入口为 `native/release/autumn-native.exe`；Windows Release 使用图形子系统启动，不附带控制台窗口；开发用 Debug 构建保留控制台输出。

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

默认使用可执行文件旁边的 `data/` 文件夹，与启动时所在工作目录无关。首次启动会创建该文件夹，首次保存后生成 `applications.json`。点击“数据目录”可打开它；`--data-dir DIR` 仍可显式指定其他位置。程序目录需要有写入权限。

移动程序时，请一起移动同目录的 `data/`，其中包含当前数据、自动备份及迁移备份。开发构建的数据位于实际 Debug EXE 旁，`--smoke-test` 仍使用临时目录。

2026-10-28 曾将此前 C 盘原生预览版的数据与备份复制到 `native/release/data/`。这是历史迁移记录；当前数据位置以实际启动的 EXE 所在目录或 `--data-dir` 为准。

显式导入旧 JSON 副本（目标目录必须尚无 applications.json）：

```sh
cargo run --manifest-path native/Cargo.toml -- --data-dir native/.dev-data --import-legacy native/tests/legacy.json
```

示例使用虚构测试数据。导入保留源文件，并保存 `legacy-import.original.json`；后续保存保留上一次内容到 `.bak`。遇到损坏数据、外部修改或同目录并发写入时拒绝覆盖。

`--smoke-test` 始终使用临时目录，执行界面回调与保存检查后自动退出，不接受旧数据导入。它不能替代真实鼠标操作、中文输入法或托盘测试。

## 运行时本机 API

程序运行期间会在数据目录中写入临时的 `local-api.json`，其中有仅供本机连接的端口和随机令牌。接口只监听 `127.0.0.1`；正常退出后地址文件会移除，异常退出可能留下失效文件。请像保护 `applications.json` 一样保护此目录，不要分享令牌。API 请求由运行中的程序保存，列表、日历和统计会立即刷新；仍保留原有目录锁与备份机制。

用同一个 EXE 的 `--api` 模式从标准输入传入一条 JSON 请求，标准输出返回 `{ "ok": true, "data": ... }` 或 `{ "ok": false, "error": ... }`。默认连接 EXE 旁的 `data/`；使用自定义数据目录时，两端都加 `--data-dir DIR`。PowerShell 例子：

```powershell
$OutputEncoding = [System.Text.UTF8Encoding]::new()
'{"op":"list_applications","query":"小红书"}' | .\autumn-native.exe --api
'{"op":"add_application","company":"示例公司","position":"Java 开发","date":"2026-11-03"}' | .\autumn-native.exe --api
'{"op":"add_stage","applicationId":"app_...","type":"written_test","title":"笔试","date":"2026-11-04","status":"pending","notes":"19:00-21:00"}' | .\autumn-native.exe --api
'{"op":"update_stage","applicationId":"app_...","id":"stage_...","status":"postponed"}' | .\autumn-native.exe --api
```

操作包括 `list_applications`（可选 `query`，匹配公司或岗位）、`get_application`（`id`）、`add_application`、`update_application`（`id`）、`add_stage`（`applicationId`）和 `update_stage`（`applicationId`、`id`）。更新时只需提供要改变的字段；投递的 `url`、`notes`，以及进展的 `date`、`notes` 可设为 `null` 以清空。进展类型为 `applied`、`assessment`、`written_test`、`interview`、`hr`、`oc`、`offer`、`rejected`、`other`；状态为 `completed`、`pending`、`failed`、`postponed`。日期使用 `YYYY-MM-DD`。含账号密码的备注请通过标准输入传递，不要作为命令行参数。

## 当前范围

已实现投递和进展的新增、编辑、删除，搜索、状态/未完成筛选、全部记录、月历、阶段统计、汇总及托盘入口。早期重构验证记录见 `docs/REWRITE_PLAN.md`；当前版本的验证内容见发布页。

进展可标记为已完成、待进行、已失败或已推迟。“已推迟”保留旧场次记录，不计入“有未完成进展”，也不会仅因改期让投递显示为“已结束”；新场次应单独添加为“待进行”。

投递和进展备注中的 HTTP/HTTPS 地址会在详情中显示为可点击链接；备注原文仍按纯文本保存和编辑。招聘流程中的进展行随备注长度调整高度，长备注完整展开，流程线连接相邻进展。

日历按进展日期高亮，支持月份跳转（1900–2100）、今天、搜索、仅看未完成及当天详情。当天每项进展的行高随内容变化，长备注完整展开，整个详情列表可滚动，备注链接可点击；点击“查看投递”可打开完整投递详情。缺少有效日期的节点单独提示数量。阶段统计按投递去重：同一岗位的多轮面试只算一次，统计包含所有状态。

尚需完成开机启动、再次启动恢复窗口，以及三平台实机验收。Linux 预览版暂不启用关闭隐藏，避免缺少托盘宿主时无法恢复窗口。

## 旧版外观还原（历史记录）

界面已按旧版样式重绘，范围和开发截图入口见 `docs/NATIVE_UI.md`。2026-10-28 的 UI 预览曾单独保存为 `native/release/autumn-native-ui.exe`；当前版本应使用 `native/release/autumn-native.exe` 或 GitHub 发布页提供的 EXE。

2026-10-28 的 UI 还原预览版实测 9.95 MB（9.49 MiB），Release 冒烟测试通过；该数值仅是历史记录。

## 启动没有打开窗口时

同一数据目录只允许一个实例写入。已有程序可能仍驻留在托盘，需通过窗口中的“退出”或托盘菜单退出；点击窗口关闭按钮通常只是隐藏。程序会用提示窗口显示目录占用、数据损坏等启动错误。
