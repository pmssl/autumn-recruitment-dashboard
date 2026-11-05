---
name: autumn-dashboard-api
description: 通过秋招看板正在运行的原生程序本机 API 查询、新增或修改投递及招聘进展；适用于用户要求查看、补充或更正看板数据，不用于修改 API 代码。
---

# 秋招看板数据操作

使用仓库内 `native/release/autumn-native.exe --api`，通过标准输入传入单条 JSON 请求。该入口连接正在运行的原生程序，修改后由程序保存并刷新界面。项目根目录是本 Skill 目录向上三级的位置；若用户指定其他 EXE 或数据目录，使用其对应的 `--data-dir DIR`。API 的字段与示例见 [接口参考](references/api.md)。

## 操作要点

1. 开始前读取项目 `RULE.md` 的相关规则。确认所连接的 EXE 与数据目录；需要写入时，先用 `list_applications` 查询，再用 `get_application` 核对目标公司的岗位、投递 ID 和进展 ID。查询可能返回同公司多岗位，不能只按公司名盲目修改。
2. 用户要求的数据变更可直接执行。新增投递用 `add_application`；它会自动创建已完成的“投递”进展。补充节点用 `add_stage`；更正已有节点用 `update_stage`，只提交要改的字段。保留现有备注时，先读取原文再构造完整的新备注，因为 `notes` 更新会替换整个字段。
3. 有日期时使用有效的 `YYYY-MM-DD`，具体时间写入备注；清空已有日期用 `null`。通知送达时间、考试时间和推算截止时间应分别标明；没有明确日期时不要编造。状态只使用接口支持的值。保存后回读目标投递，核对 ID、日期、状态和必要的备注内容。
4. 请求中的账号、密码、专属链接和令牌经标准输入传递，不放在命令行参数、日志、Skill 文件或 Git 跟踪文件中；输出和答复只显示用户需要的信息。不要直接编辑正在运行程序的 `applications.json`；API 会沿用现有数据校验、目录锁和备份机制。
5. 如果程序未运行、地址文件已过期或接口返回错误，报告实际错误并检查运行状态与数据目录。不要把失败的调用描述为已保存，也不要在未核对目标的情况下重试写入。

在 PowerShell 中先设置 `$OutputEncoding = [System.Text.UTF8Encoding]::new()`，再用 `ConvertTo-Json -Compress -Depth 10` 生成请求并管道传入 EXE。解析返回的 `{ok,data}` 或 `{ok,error}`；`ok` 为 `false` 时停止并处理错误。脚本和文档读写中文时遵守 `RULE.md` 的 UTF-8 规则。
