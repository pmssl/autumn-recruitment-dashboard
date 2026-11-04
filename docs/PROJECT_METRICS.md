# 代码量和安装包统计

项目 Skill 位于 `.agents/skills/project-metrics/SKILL.md`，可在支持项目 Skill 的 Codex 会话中通过
`$project-metrics` 调用，例如：“使用 $project-metrics 查看当前代码量和安装包大小”。
Skill 复用下方统计脚本，不维护另一份统计实现。若当前会话尚未刷新 Skill 列表，可直接指定该 SKILL.md 路径。

在项目根目录执行：

```powershell
npm run metrics
```

也可直接运行 `node .agents/skills/project-metrics/project-metrics.cjs`，不需要先安装 npm 依赖。

输出包括：

- React/Electron 旧版、Rust/Slint 新版、项目工具各自的文件数、物理行数、非空行数和源码字节数。
- 配置和文档的独立统计；JSON 报告还包含按扩展名分组及逐文件明细。
- 已有的各版本安装包大小，以及与当前 package.json 版本匹配的安装包。
- 原生构建产物（如存在）、前端 dist 和 Electron win-unpacked 目录的大小。

## 导出报告

```powershell
npm run metrics -- --out metrics-report/current.json
node .agents/skills/project-metrics/project-metrics.cjs --json
```

`--json` 直接运行 Node 时标准输出是纯 JSON，适合其他工具读取。
`--out` 只允许写入被 Git 忽略的 `metrics-report/` 下的 JSON 文件。需要历史对比时使用不同文件名保存快照。

## 统计口径

Git 环境下读取已跟踪和未跟踪但未忽略的文件，并读取当前磁盘内容；因此尚未提交的新代码也会统计。
没有 Git 时使用目录扫描和内置排除规则。跳过符号链接，避免重复统计或越界读取。
不统计 node_modules、target、dist、release、个人备份、同步数据、Excel、环境配置及依赖锁文件。
历史的一次性 Excel 同步脚本也被排除。工具自身和测试代码属于源码，会计入。

行数是包含注释的物理行数，空白行单独排除后得到“非空行数”；不声称测量有效逻辑代码行数。
当前部分 React/CSS 文件采用紧凑排版，行数不适合直接与格式化的 Rust 代码比较，需同时参考字节数。

安装包与解压目录分开统计。MB 按十进制，MiB 按二进制。
工具不会触发构建：现有安装包不一定包含当前分支代码。要测量新版，请先完成对应构建再运行。
对 Rust 默认识别 native/target/release/autumn-native[.exe]，以及 native/dist、native/release、native/target/release/bundle 中的分发包。
