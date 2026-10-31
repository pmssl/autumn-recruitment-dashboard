# Windows 窗口重绘试验

本地分支 `codex/slint-upstream-repaint` 将 Slint 和 `slint-build` 固定到上游提交 `08a297b7151e52b98b500b4424a8dee770fc7b58`。该提交包含 [Windows 重绘修复 #13568](https://github.com/slint-ui/slint/pull/13568)，针对 [窗口部分区域未绘制 #13537](https://github.com/slint-ui/slint/issues/13537)。项目原先使用的正式版 1.18.1 早于该修复；本试验解析为 Slint 1.19.0 开发版，并非已发布的正式版本。

已完成 `cargo check`、11 项单元测试、Release 构建和隔离数据冒烟测试；EXE 仍为 Windows GUI 子系统。Debug 虚构数据截图覆盖看板、全部记录、日历、统计、抽屉、编辑弹窗和窄窗口，已抽查主要页面无明显布局变化。这些验证不能替代在出现问题的电脑上反复打开、托盘恢复和跨显示器移动窗口。

2026-10-31，用户在原先出现空白区域的电脑上试用该 EXE，确认首次打开和托盘恢复均不再出现空白；跨显示器移动尚未验证。

本地试用文件为 `native/release/autumn-native-slint-upstream.exe`。使用前先从托盘退出旧程序；它与同目录其他原生 EXE 共用 `data/`，不要同时运行。验证时保留旧 EXE 和数据目录不变。确认重绘问题消失且无回归后，再考虑合并；待 Slint 正式版包含该修复时，移除 Git 修订固定并升级到正式版。
