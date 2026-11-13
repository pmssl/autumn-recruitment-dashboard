# 0003：单 Pages 项目的 Slint 网页适配与 D1 存储

- **日期**：2026-11-13（Asia/Shanghai）
- **状态**：已从本机部署到 Cloudflare；GitHub Actions 自动部署尚未验证
- **背景与确认意图**：用户要求保留现有 Rust + Slint 技术栈，完成网页适配，仅使用一个 Cloudflare Pages 项目，并配置推送后的自动部署。浏览器无法安全持有 D1 绑定，必须由服务端接口访问。
- **决定**：将 Slint 界面编译为浏览器 WebAssembly，Pages 项目内的 Functions 作为同源 API，通过 D1 绑定读写。暂用一行 JSON 文档保存完整 `Application` 数组和 revision，保留现有 camelCase 数据与未知字段；条件更新拒绝旧版本覆盖。EXE 仍使用本地 JSON，不在此阶段自动同步。
- **理由**：此项目当前数据模型以完整 JSON 数组为边界，原生领域代码保留未知字段。单文档结构便于一次性导入并复用 Rust 的验证与视图代码；revision 与 D1 条件更新提供明确的并发失败。单个 Pages 项目满足用户的部署约束。
- **考虑的替代方案**：旧 React 网页版需要重新追齐当前 Slint 界面；独立 Rust Worker 会增加第二个 Cloudflare 部署单元；浏览器直接使用 D1 管理令牌会暴露凭据；投递/进展拆表可增强查询扩展性，但需要额外迁移映射和未知字段处理。现阶段先选择单文档存储，若数据量或查询需求变化再评估拆表。
- **影响与边界**：Pages Functions 使用 JavaScript，前端和领域仍为 Rust + Slint。部署脚本从环境变量取得单账号范围的 Cloudflare API Token，自动创建或复用 Pages 和 D1，并从该 Token 派生网页访问口令写入 Pages Secret；GitHub Actions 需配置同名 Secret 才能自动部署。Token 轮换会改变网页口令。网页协调代码与原生入口有重复，后续功能变化要同步检查。已完成本机构建、远端部署及 API 读写入口的读取验证；浏览器视觉交互和 GitHub Actions 尚未验收。
- **证据**：`web/frontend/{Cargo.toml,build.rs,src/lib.rs}`、`web/functions/api/data.js`、`web/migrations/0001_dashboard.sql`、`web/scripts/{cloudflare.mjs,cloudflare.test.mjs,deploy.mjs}`、`.github/workflows/pages.yml`、`web/README.md`；2026-11-13 本地 `cargo check --target wasm32-unknown-unknown`、`npm run build`、`npm test`、Wrangler 本地请求及远端迁移结果；正式域名根页面 200、未授权 API 401、授权读取 200、空数据写入及回读成功。用户明确要求首次自动创建 Pages 和 D1，且 GitHub 仅提供一个 Cloudflare API Token。
