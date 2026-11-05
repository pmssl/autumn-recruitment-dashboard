# 原生本机 API 参考

依据：`native/src/api.rs`、`native/src/local_api.rs`、`native/src/main.rs`、`native/README.md`。以下为当前源码支持的操作；修改接口后应同步核对本页。

## 调用方式

程序运行时在其数据目录写入临时 `local-api.json`。EXE 的 `--api` 模式从标准输入读取一条 JSON 请求，代为连接本机回环地址，不需要调用者直接读取令牌。默认数据目录为 EXE 旁的 `data/`；自定义目录使用 `--data-dir DIR`。返回 `{ "ok": true, "data": ... }` 或 `{ "ok": false, "error": "..." }`。程序未运行时不可调用。单次请求上限 1 MiB。

从项目根目录查询的 PowerShell 示例：

```powershell
$OutputEncoding = [System.Text.UTF8Encoding]::new()
$exe = Join-Path (Get-Location) 'native/release/autumn-native.exe'
$request = @{ op = 'list_applications'; query = '示例公司' } | ConvertTo-Json -Compress
$reply = $request | & $exe --api | ConvertFrom-Json
if (-not $reply.ok) { throw $reply.error }
$reply.data | Select-Object id, company, position
```

## 操作与字段

| `op` | 必需字段 | 可选字段 | 返回 `data` |
| --- | --- | --- | --- |
| `list_applications` | 无 | `query`：公司或岗位文本，缺省为全部 | 投递数组，含进展 |
| `get_application` | `id`：投递 ID | 无 | 单条投递，含进展 |
| `add_application` | `company`、`position` | `date`、`url`、`notes` | 新投递，自动含已完成的“投递”节点 |
| `update_application` | `id`：投递 ID | `company`、`position`、`url`、`notes` | 更新后的投递 |
| `add_stage` | `applicationId`、`type`、`title`、`status` | `date`、`notes` | 新进展 |
| `update_stage` | `applicationId`、`id`：进展 ID | `type`、`title`、`date`、`status`、`notes` | 更新后的进展 |

`add_application` 未传 `date` 时，以运行程序所在机器的本地日期创建“投递”节点；显式传 `null` 可使其无日期。`add_stage` 未传 `date` 时无日期。`update_*` 只修改出现的可选字段；`url`、`notes` 和进展 `date` 可传 `null` 清空，不能用空字符串当有效日期。公司、岗位和进展标题必须是非空字符串。

进展 `type`：`applied`、`assessment`、`written_test`、`interview`、`hr`、`oc`、`offer`、`rejected`、`other`。

进展 `status`：`completed`、`pending`、`failed`、`postponed`。`date` 为真实日历日期，格式 `YYYY-MM-DD`；时间及时间区间写入 `notes`。

## 添加与更新示例

在已核对目标投递 ID 后，将请求对象传给上述 EXE；示例值为虚构数据。

```powershell
@{ op = 'add_stage'; applicationId = 'app_example'; type = 'written_test'; title = '笔试'; date = '2026-11-06'; status = 'pending'; notes = '20:00-21:30' } | ConvertTo-Json -Compress | & $exe --api
@{ op = 'update_stage'; applicationId = 'app_example'; id = 'stage_example'; status = 'postponed' } | ConvertTo-Json -Compress | & $exe --api
```

实际操作请解析 `ok` 并用 `get_application` 回读，不能仅以命令退出状态判断保存成功。备注中含敏感信息时，避免输出完整请求或完整响应。
