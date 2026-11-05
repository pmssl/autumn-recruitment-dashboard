---
name: project-usage-metrics
description: 查看和维护本项目每个 Skill 的逐轮使用次数及 RULE.md 改动次数；适用于用户询问 Skill 使用频率、规则文件变更统计或逐轮汇总口径。
---

# 项目使用统计

本 Skill 的脚本和本地状态都保存在同一目录。项目根目录是本文件向上三级的位置。使用方式和计数口径见 [统计工具说明](README.md)。

从项目根目录运行：

```powershell
python .agents/skills/project-usage-metrics/usage_tracker.py show
```

每轮答复前的例行登记由项目 `AGENTS.md` 指定：运行 `record`，并对本轮**实际应用**的项目 Skill 各传一次 `--skill <名称>`。只提到或读取 Skill 文件不计入；为了登记而运行本脚本也不计入 `project-usage-metrics`。当用户明确要求查询或维护这些统计且使用了本 Skill 时，才登记它的使用。

状态文件 `usage-state.json` 被 Git 忽略；它只保存轮次 ID、Skill 名称、时间和 `RULE.md` 哈希，不保存对话内容。迁移或备份统计时应将它与项目一起单独复制。不要把启用后的逐轮改动次数与 Git 历史提交次数相加。
