# 对话使用统计

该工具按 Codex 轮次统计本项目 `.agents/skills/` 中各 Skill 的使用次数，以及 `RULE.md` 在启用后的逐轮改动次数。它只在本机写入忽略 Git 的 `usage-state.json`，不保存提示词或用户数据。`AGENTS.md` 要求每轮结束前调用并在答复末尾附上汇总。

```powershell
python .agents/skills/project-usage-metrics/usage_tracker.py record
python .agents/skills/project-usage-metrics/usage_tracker.py record --skill autumn-dashboard-api
python .agents/skills/project-usage-metrics/usage_tracker.py show
python .agents/skills/project-usage-metrics/usage_tracker.py show --json
```

每轮实际使用多个项目 Skill 时重复传 `--skill`。同一 Skill 在一轮内使用多次仍记 **1 次**；重复运行 `record` 不增加轮次，技能名会合并。例行执行统计脚本本身不算使用 `project-usage-metrics`；只有用户要求查询或维护统计功能并实际使用本 Skill 时才计入。全局 Skill 不计入“该项目下每个 Skill”。脚本依赖 Codex 的 `CODEX_THREAD_ID` 与本机会话日志中的轮次 ID；在其他环境中可明确传 `--thread-id`、`--turn-id`。只提到 Skill 名称不算使用，实际应用时须在 `record` 命令中声明。

`RULE.md` 的“启用后改动轮次”来自每轮结束时的文件哈希变化，首次记录建立基线；同一轮内改动多次只记 1 轮，改后又恢复原文则记 0。另列出的“Git 历史涉及该文件的提交数”用 `git rev-list --count HEAD -- RULE.md` 计算，两种口径**不相加**。只有实际运行了 `record` 的轮次才进入启用后统计；这不是对旧会话的自动补算。
