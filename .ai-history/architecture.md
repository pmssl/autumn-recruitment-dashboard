# 当前架构

**基线日期：2026-11-04（Asia/Shanghai）。状态：包含建立基线时工作区中的界面修改；不等同于已发布 EXE。**

## 系统边界

- **已确认**：当前主入口是 `native/` 的 Rust + Slint 便携程序；根目录的 React/Vite + Electron 代码与 v1.0.3 使用说明作为旧版保留。证据：`README.md`、`native/README.md`、`native/Cargo.toml`、`package.json`、Git 合并提交 `3d7379d`。
- **已确认**：原生程序的领域与数据层在 `native/src/{domain,store,views,api,local_api,note_links}.rs`，`main.rs` 连接 Slint 回调、状态、存储与本机 API，界面声明在 `native/ui/app.slint`。`native/src/lib.rs` 导出非 UI 模块；`native/build.rs` 编译 Slint。证据：上述源码与 `native/src/lib.rs`。
- **已确认**：运行数据默认位于实际 EXE 旁的 `data/applications.json`；`--data-dir` 可覆盖。旧 Electron 数据使用另一目录，原生版只显式导入旧 JSON 副本。证据：`native/src/main.rs`、`native/src/store.rs`、`native/README.md`。

## 依赖方向

```text
Slint UI (`native/ui/app.slint`)
        ↕ 回调 / 展示模型
原生协调层 (`native/src/main.rs`)
        ├── 领域规则 (`domain.rs`)
        ├── 查询投影 (`views.rs`)
        ├── 文件存储 (`store.rs`)
        ├── 请求操作 (`api.rs`) ↔ 本机传输 (`local_api.rs`)
        └── 备注链接展示 (`note_links.rs`)
```

**已确认**：`api.rs` 修改内存中的投递集合；运行中的程序接收本机请求、保存并刷新 UI。`local_api.rs` 只监听 `127.0.0.1`，使用数据目录中的临时地址与随机令牌文件。证据：`native/src/api.rs`、`native/src/local_api.rs`、`native/src/main.rs`、`native/README.md`。

## 关键约束

- **已确认**：投递与进展以 JSON 数组及 camelCase 字段保存；未知字段通过 `extra` 保留。进展状态含 `completed`、`pending`、`failed`、`postponed`。证据：`native/src/domain.rs` 及其 `legacy_round_trip_retains_unknown_fields`、`postponed_stage_keeps_application_active_and_round_trips` 测试。
- **已确认**：损坏数据、并发写入及外部修改应阻止覆盖；保存保留上次版本 `.bak`。证据：`native/src/store.rs` 及其三项测试。
- **已确认**：未完成筛选只依据是否存在 `pending` 进展；统计按含该阶段类型的投递去重；日历只放入有效日期的进展。证据：`native/src/domain.rs`、`native/src/views.rs` 及其测试。
- **已确认**：当前 README 仅声明 Windows x64 发布；macOS/Linux 实机验收、开机启动和重复启动恢复窗口仍未完成。证据：`README.md`、`native/README.md`。

模块细节见 [modules](modules/)。历史范围与未确认事项见 [决策 0001](decisions/0001-current-baseline.md)。
