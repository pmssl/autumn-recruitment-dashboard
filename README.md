# 秋招看板桌面版

## 当前版本：v2.0.0 原生便携版

当前主版本采用 Rust + Slint。到 GitHub Releases 下载 Windows x64 ZIP，解压到可写目录，双击 `autumn-native.exe` 使用，无需安装 Node.js 或浏览器运行时。

支持投递和进展管理、搜索、状态与未完成筛选、全部记录、日历、统计、托盘。数据默认保存在 EXE 旁的 `data/applications.json`；移动程序时一起移动 `data` 文件夹。升级前退出托盘中的程序，备份 `data`，再替换 EXE。发布包不包含个人数据。

旧版数据不会自动导入：退出旧版并备份其数据后，可运行 `autumn-native.exe --import-legacy "旧版 applications.json 的完整路径"`，仅允许导入到尚无数据的新目录。也可用 `--data-dir` 指定数据目录。具体开发与数据说明见 [native/README.md](native/README.md)。

本次仅提供 Windows x64 产物；macOS/Linux 尚未完成实机验证。当前不提供 Excel 导入、开机启动和重复启动恢复窗口；数据目录被其他实例占用时会提示退出已有实例。

旧版 v1.0.3 Release 和下方 Electron 源码使用说明继续保留。

## 旧版 v1.0.3（Electron）

Windows 桌面程序，保留招聘进度、全部记录、统计、Excel 导入等功能。

## 安装和使用

运行 `release/Autumn-Dashboard-Setup-1.0.3.exe` 完成安装，从桌面快捷方式启动。

- 点击窗口关闭按钮：隐藏到系统托盘，程序继续运行。
- 点击托盘图标：恢复窗口。图标可能位于任务栏的折叠区域。
- 右键托盘图标 → 退出：完全退出程序。
- 右键托盘图标 → 开机启动：自行开启或关闭，默认不开启；开启后登录 Windows 时静默驻留托盘。
- 重复启动只会恢复已有窗口。
- 使用程序不需要 Node.js、浏览器或开发服务器。

## 数据

桌面版数据保存到 `%APPDATA%/秋招看板/applications.json`，托盘菜单可打开实际数据文件夹。
每次保存保留上一版本 `applications.json.bak`，它不是完整历史备份。可自行复制整个数据文件夹备份。
如需恢复，先退出程序，再用备份替换 applications.json。
读取损坏的数据会阻止编辑，避免覆盖原文件；保存失败会在窗口内提示并提供重试按钮。
首次运行桌面版为空白记录，浏览器 localStorage 数据不会自动迁移。
Excel 导入只处理第一个工作表的公司、岗位、日期，每行生成一条投递；日期建议使用 YYYY-MM-DD 文本，重复导入不会去重。

## 开发与打包

```powershell
npm install
npm run desktop
```

生成 Windows x64 安装包：

```powershell
npm run dist
```

输出在 release，`release/win-unpacked/秋招看板.exe` 可直接运行，但必须保留同目录全部文件。

```powershell
npm test         # 本地存储验证
npm run build    # TypeScript 检查与前端构建
```

采用 Electron 承载本地 React 界面，主进程管理托盘和文件存储。渲染进程不启用 Node.js，通过隔离的 preload API 保存数据。安装包暂未配置代码签名。

## 未完成进展筛选

招聘进度和全部记录页提供“有未完成进展”筛选，显示至少有一个 pending（待进行）节点的投递，可与公司/岗位搜索同时使用。计数按投递条数计算。即使投递整体为 Offer 或已结束，只要仍有待进行节点也会显示，方便清理遗漏进展。

## 日历

点击顶部“日历”，有进展的日期会高亮，按状态区分颜色。可切换月份、返回今天、搜索公司/岗位及仅看未完成进展。选中日期后查看当天所有节点，点击节点打开投递详情进行修改。日期与状态直接读取现有流程数据，编辑后自动更新。未填写有效日期的节点不会被放入日历；截止日期与具体时间以备注原文为准。
