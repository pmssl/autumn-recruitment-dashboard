# 0001：建立当前设计基线

- 日期：2026-11-04，Asia/Shanghai。
- 状态：基线快照；不是对所有历史架构选择的追认。
- 范围：仓库中当前原生版、保留的 Electron 旧版、数据与 API 约束。

## 背景和原因

用户要求将设计知识与后续 AI 修改的意图、证据保存在 Git 项目中。该原因来自本次请求。此记录只建立可核实的当前状态，不为以往技术选型补写未记录的理由。

## 基线证据

- 代码：`native/Cargo.toml`、`native/src/*.rs`、`native/ui/app.slint`、`electron/*.cjs`、`src/*.tsx`、`package.json`。
- 行为承诺：`native/src/{domain,store,views,api,note_links}.rs` 中的测试、`electron/store.test.cjs`；本次只阅读测试，未执行测试套件。
- 文档：`README.md`、`native/README.md`、`docs/REWRITE_PLAN.md`、`docs/NATIVE_UI.md`、`AGENTS.md`、`RULE.md`。
- 历史：`a4c5bb3`（旧版初始提交）、`3d7379d`（合并原生版并保留 Electron）、`7f9e5cd`（原生 v2.0.0 便携存储）、`d11c2ae`（固定 Slint 重绘修复提交）、`7a42324`（本机 API 与备注链接）；这些提交标题及明确文档支持的变化可追溯，但不能单凭标题推断全部决策动机。

## 工作区边界

初始化时 `HEAD` 为 `f7d9e56`，工作区已有未提交修改：`RULE.md`、`native/README.md`、`native/src/main.rs`、`native/ui/app.slint`。基线建立没有改动这些业务文件；当前设计说明包含当时可见的动态高度实现，发布状态仍以实际发布记录为准。

## 替代方案与影响

本次只按用户要求使用仓库内 Markdown 与 Git 证据；没有评估外部知识库方案。后续设计变化需更新当前文档，长期取舍新建编号记录，避免把未提交实现误写为已发布事实。

## 未知及待确认

- macOS/Linux 实机行为、中文输入法和系统托盘跨平台体验尚无本次验证。
- 本机 API 的跨进程和并发运行测试覆盖程度需进一步核实。
- 当前未提交 UI 修改何时提交、发布，由后续明确任务决定。
