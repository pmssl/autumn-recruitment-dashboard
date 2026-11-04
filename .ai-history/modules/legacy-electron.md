# 保留的 Electron 旧版

**已确认。** 根 `src/` 是 React/Vite 界面，`electron/main.cjs` 管理窗口、托盘与 IPC，`electron/preload.cjs` 向渲染层暴露受限接口，`electron/store.cjs` 保存旧版 JSON；`package.json` 的 `desktop`、`dist`、`test` 为旧版入口。证据：这些源码、`package.json`、根 `README.md`。

**已确认。** 旧版数据位于 Electron 用户配置目录；旧版存储测试覆盖保存、重启读取、备份和损坏数据拒绝覆盖。原生版不直接写此目录，允许显式导入副本。证据：`electron/main.cjs`、`electron/store.cjs`、`electron/store.test.cjs`、`native/src/store.rs`。

**已确认的历史变化**：提交 `3d7379d` 将原生 v2.0.0 合并到主线并保留 Electron v1 源码；`docs/REWRITE_PLAN.md` 记录 2026-10-28 用户将原先等待完整跨平台验收后合并的安排改为当时发布。旧版与原生版具体字段语义后续是否完全兼容，**未知**。
