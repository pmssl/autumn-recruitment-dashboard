# 原生领域模型与文件存储

**已确认。** `native/src/domain.rs` 定义 `Application` 与 `Stage`，后者通过 `application_id` 归属投递，按 `sort_order` 构成流程。JSON 使用 camelCase；未知属性扁平保留。`validate` 检查 ID 唯一、公司与岗位非空、节点归属及有限顺序值；日期合法性由 `valid_date` 和调用方使用，并非 `validate` 的全局约束。证据：`native/src/domain.rs`、`legacy_round_trip_retains_unknown_fields`、`reject_wrong_owner_duplicate_ids_and_unknown_status`、`dates_require_real_calendar_days`。

**已确认。** 已完成 Offer 节点优先导出 Offer 结果；失败节点或 rejected 类型导出已结束；否则进行中。`has_pending` 仅检查 `pending`，已推迟不算未完成。证据：`Application::state`、`has_pending`、`pending_is_independent_of_result`、`postponed_stage_keeps_application_active_and_round_trips`。

**已确认。** `native/src/store.rs` 为目录取得独占锁；拒绝直接使用旧 Electron 配置目录。读取失败时不覆盖原文件。保存前比对磁盘快照，写入临时文件并持久化，上一版本写入 `applications.json.bak`；导入仅允许空目标并保留源文件及原始副本。证据：`Store::{open,load,save,import_legacy}`、`import_is_copy_only_and_preserves_source`、`corrupt_data_and_external_edits_are_never_overwritten`、`two_writers_cannot_open_same_directory`。

**未知**：旧版未来新增字段是否始终能由原生程序正确解释；现有测试只覆盖仓库内的 `native/tests/legacy.json` 样本。
