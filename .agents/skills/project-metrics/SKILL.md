---
name: project-metrics
description: 统计秋招看板项目的代码行数、源码体积、已有安装包和原生程序大小；在用户询问代码量、安装包体积、新旧版本体积对比或需要导出统计报告时使用。
---

# 项目代码量与体积统计

使用本 Skill 目录中的 `project-metrics.cjs`，不要重新编写统计逻辑或复制历史数字作为当前结果。
这是项目内 Skill，统计脚本随 Skill 一起保存在仓库中，不需要 npm install。

## 执行

项目根目录是本 Skill 目录向上三级的位置，确认该目录同时包含 `package.json` 和
`.agents/skills/project-metrics/project-metrics.cjs`。从根目录运行：

```sh
node .agents/skills/project-metrics/project-metrics.cjs --json
```

优先解析纯 JSON 输出。用户希望自行在终端查看时，提供 `npm run metrics`。
只有需要保存报告时才运行：

```sh
node .agents/skills/project-metrics/project-metrics.cjs --json --out metrics-report/current.json
```

历史快照使用不同文件名，例如 `metrics-report/before-refactor.json`，避免覆盖用于对比的基线。
如果命令失败，检查退出信息和路径；不要把失败解释为零行代码或零大小。

## 解释结果

- `source.total` 是源码汇总；`source.byGroup` 区分旧版 React/Electron、新版 Rust/Slint、工具和其他代码。
- 配置和文档在 `config`、`docs` 中单独统计，不混入源码总行数。
- `lines` 包含注释与空行，`nonEmptyLines` 仅排除空白行，不能称为“有效代码行数”。紧凑排版会使行数偏小，因此同时提供源码字节数或 KiB。
- `currentInstaller` 是与 package.json 版本号匹配的现有 Electron 安装包。它不证明安装包包含当前分支的代码，应结合 `modifiedAt` 和说明提示用户。
- `artifacts` 中安装包、压缩分发包、独立可执行程序分别呈现；`buildDirectories` 是解压后的构建目录，不是安装包体积。
- 没有 Rust 构建产物就明确说明“尚未构建或未找到”，不要套用旧版体积或预估值。
- MB = 1,000,000 字节；MiB = 1,048,576 字节。做版本比较时统一单位、平台和产物类型。
- 对比两个已有快照时使用相同分组/产物路径；减小百分比为 `(旧大小 - 新大小) / 旧大小 × 100%`，基线为零或缺失时不算百分比。
- 呈现 `warnings` 中影响结论的内容。统计默认只读，不因为一次体积查询自动安装依赖、构建程序或发布版本。

回答以用户所问的结果为主，不必粘贴全部 JSON。若导出文件，给出报告的本地链接。
需要确认文件排除规则或产物搜索路径时，读取 [统计工具说明](../../../docs/PROJECT_METRICS.md)。
