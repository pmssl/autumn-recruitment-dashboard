# 原生本机 API

**已确认。** `native/src/api.rs::execute` 提供 `list_applications`、`get_application`、`add_application`、`update_application`、`add_stage`、`update_stage`。新增投递自动建立已完成的投递节点；更新允许只提供变化字段，部分可选字段可用 `null` 清空。操作返回 `(JSON, 是否变更)`，提交前执行领域校验。证据：`native/src/api.rs`、`round_trip_and_rejected_changes`、`native/README.md`。

**已确认。** `native/src/local_api.rs` 在回环地址上启动临时 TCP 服务，数据目录的 `local-api.json` 保存地址和随机令牌，退出时删除；单次请求上限 1 MiB、读写/处理超时 10 秒。请求送回运行中的应用协调层。命令行 `--api` 通过标准输入接收 JSON。证据：`native/src/local_api.rs`、`native/src/main.rs`、`native/README.md`；新增 API 的 Git 提交为 `7a42324`。

**未知**：未发现独立的本机 TCP 集成测试；`api.rs` 测试验证操作语义，不等同于跨进程连接或多实例并发验收。
