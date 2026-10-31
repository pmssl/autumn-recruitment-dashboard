# 秋招看板 v2.0.1

修复“有未完成进展”筛选下“全部记录”的进展摘要：该列现在显示待进行节点；多个待进行节点按流程顺序列出，列名同步显示为“未完成进展”。其他筛选仍沿用原有的“当前进展”口径。

下载 `Autumn-Dashboard-2.0.1-windows-x64.zip`，解压到有写入权限的目录，运行 `autumn-native.exe`。升级时先从托盘退出旧程序，备份程序旁的 `data/`，再替换 EXE。发布包不包含个人数据，现有数据仍位于 EXE 旁的 `data/applications.json`。

本次提供 Windows x64 便携包；macOS/Linux 尚未完成实机验收。Excel 导入、开机启动和重复启动恢复已有窗口仍未提供；程序尚未签名。

验证：11 项原生单元测试和隔离数据冒烟测试通过，Windows Release EXE 的 GUI 子系统已检查。附件提供 ZIP 和 EXE 的 SHA-256 校验值。
