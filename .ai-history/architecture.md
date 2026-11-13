# 当前架构

**基线日期：2026-11-04（Asia/Shanghai）。状态：包含建立基线时工作区中的界面修改；不等同于已发布 EXE。**

## 系统边界

- **已确认**：当前主入口是 `native/` 的 Rust + Slint 便携程序；根目录的 React/Vite + Electron 代码与 v1.0.3 使用说明作为旧版保留。证据：`README.md`、`native/README.md`、`native/Cargo.toml`、`package.json`、Git 合并提交 `3d7379d`。
- **已确认**：原生程序的领域与数据层在 `native/src/{domain,store,views,api,local_api,note_links}.rs`，`main.rs` 连接 Slint 回调、状态、存储与本机 API，界面声明在 `native/ui/app.slint`。`native/src/lib.rs` 导出非 UI 模块；`native/build.rs` 编译 Slint。证据：上述源码与 `native/src/lib.rs`。
- **已确认**：运行数据默认位于实际 EXE 旁的 `data/applications.json`；`--data-dir` 可覆盖。旧 Electron 数据使用另一目录，原生版只显式导入旧 JSON 副本。证据：`native/src/main.rs`、`native/src/store.rs`、`native/README.md`。
- **已确认（当前源码）**：原生程序可按 `click-stats` Cargo feature 构建点击统计版；无此 feature 的默认版不读取或写入点击统计。统计版把生效按钮操作的累计次数原子保存到数据目录的 `click-stats.json`，与 `applications.json` 分开。证据：`native/Cargo.toml`、`native/src/click_stats.rs`、`native/src/main.rs`、`native/ui/app.slint`。
- **已确认（已从本机部署）**：`web/` 是单个 Cloudflare Pages 项目的网页适配。Rust + Slint 前端编译为浏览器 WebAssembly；同项目 Pages Functions 通过 D1 绑定保存云端数据。部署脚本凭单账号范围的 Cloudflare API Token 创建或复用 Pages 与 D1，运行迁移并部署。2026-11-13 已验证正式域名和受保护的读取接口；GitHub Actions 自动部署待验证。网页与 EXE 分别使用 D1 和本地 JSON，不自动同步。证据：`web/frontend/Cargo.toml`、`web/frontend/src/lib.rs`、`web/functions/api/data.js`、`web/scripts/{cloudflare.mjs,deploy.mjs}`、`.github/workflows/pages.yml`。

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
        └── 可选点击计数 (`click_stats.rs`)
```

**已确认**：`api.rs` 修改内存中的投递集合；运行中的程序接收本机请求、保存并刷新 UI。`local_api.rs` 只监听 `127.0.0.1`，使用数据目录中的临时地址与随机令牌文件。证据：`native/src/api.rs`、`native/src/local_api.rs`、`native/src/main.rs`、`native/README.md`。

网页适配的依赖方向为 `native/ui/app.slint` 与 `native/src/{domain,views,note_links}.rs` → `web/frontend` WebAssembly → 同源 `/api/data` Pages Function → D1。D1 中的单个 JSON 文档沿用原生模型；条件更新版本号防止旧窗口覆盖新写入。网页按钮协调逻辑目前在 `web/frontend/src/lib.rs` 中单独实现；后续修改原生回调时应检查两个入口是否需要同步。详见 [web 模块](modules/web-pages.md) 和 [决策 0003](decisions/0003-pages-d1-web-adapter.md)。

## 关键约束

- **已确认**：投递与进展以 JSON 数组及 camelCase 字段保存；未知字段通过 `extra` 保留。进展状态含 `completed`、`pending`、`failed`、`postponed`。证据：`native/src/domain.rs` 及其 `legacy_round_trip_retains_unknown_fields`、`postponed_stage_keeps_application_active_and_round_trips` 测试。
- **已确认**：损坏数据、并发写入及外部修改应阻止覆盖；保存保留上次版本 `.bak`。证据：`native/src/store.rs` 及其三项测试。
- **已确认**：未完成筛选只依据是否存在 `pending` 进展；统计按含该阶段类型的投递去重；日历只放入有效日期的进展。证据：`native/src/domain.rs`、`native/src/views.rs` 及其测试。
- **已确认**：当前 README 仅声明 Windows x64 发布；macOS/Linux 实机验收、开机启动和重复启动恢复窗口仍未完成。证据：`README.md`、`native/README.md`。

模块细节见 [modules](modules/)。历史范围与未确认事项见 [决策 0001](decisions/0001-current-baseline.md)。
