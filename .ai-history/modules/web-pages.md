# Pages 网页适配

**已确认**：`web/frontend/build.rs` 从 `native/ui/app.slint` 生成不含原生托盘的网页界面；`web/frontend/src/lib.rs` 复用 `native/src/{domain,views,note_links}.rs`，在浏览器运行投递、进展、筛选、日历和统计回调。浏览器通过同源 HTTP API 读写，不使用原生的 `Store`、托盘、本机 API 或点击统计。`web/public/` 提供口令输入界面与 Slint canvas。WebAssembly 构建在 2026-11-13 使用 `wasm-pack` 完成；尚未完成真实浏览器交互验收。

**已确认**：`web/functions/api/data.js` 接受带 Bearer 口令的 `GET/PUT /api/data`，口令来自 Pages Secret `DASHBOARD_TOKEN`，无有效口令即拒绝请求。D1 表由 `web/migrations/0001_dashboard.sql` 创建，保存与原生版兼容的投递 JSON 数组和单调增加的 revision；写入前验证主要字段，旧 revision 返回 409。`web/functions/api/data.test.js` 覆盖鉴权、写入回读、冲突拒绝和无效字段。2026-11-13 在 Wrangler 本地 D1 上实际验证未授权 401、读取 200、写入 200、旧版本 409；根页面和 Wasm 资源响应 200。

**已确认（本地部署与模拟测试）**：`.github/workflows/pages.yml` 在 `main` 相关文件推送时构建并调用 `web/scripts/deploy.mjs`。脚本用唯一的 `CLOUDFLARE_API_TOKEN` 查询单个授权账号，创建或复用名为 `autumn-dashboard` 的 D1 和带账号 ID 后缀的 Direct Upload Pages 项目，生成不入库的 Wrangler 配置，执行远端 migration，设置 `DASHBOARD_TOKEN` Secret 后部署。2026-11-13 已从本机运行该脚本，远端迁移成功，正式域名根页面返回 200、未授权 API 返回 401、授权读取返回 200；向空云端写入空数组后回读 revision 1、投递 0 条。`web/scripts/cloudflare.test.mjs` 验证首次创建、后续复用、多账号拒绝、临时传输失败重试和口令派生；GitHub Actions 自动部署尚未验证。网页访问口令由该 API Token 派生，更换 Token 后须重新部署，原口令失效。网页与桌面数据独立；`web/scripts/import-data.mjs` 提供一次性、仅对空云端执行的数据导入。

**约束**：真实投递 JSON 和口令不入库；初始页面仅包含空白登录表单；会话口令保存在浏览器 `sessionStorage`，不是 D1 凭据。多窗口冲突不会自动合并，用户需要刷新再编辑。当前布局沿用桌面尺寸，移动端体验尚未验收。
单文档写入上限为 1.9 MB，以低于 D1 的 2,000,000 字节单行/字符串限制；接近此规模时需拆分数据表。
