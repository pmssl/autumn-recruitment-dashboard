# Cloudflare Pages 网页版

本目录将现有 Rust + Slint 界面编译为浏览器 WebAssembly，并在**同一个 Pages 项目**中使用 Pages Functions 读写 D1。浏览器不持有 D1 凭据；`/api/data` 需要访问口令。原生 EXE 继续使用其本地 JSON，两个版本不会自动同步。

## 首次配置

1. 在 Cloudflare 创建一个**只授权目标账号**的 API Token，授予该账号 `Cloudflare Pages: Edit` 和 `D1: Edit` 权限。该 Token 还须能列出其授权账号；如果它能访问多个账号，脚本会停止，避免建错位置。
2. 在 GitHub 仓库的 **Settings → Secrets and variables → Actions** 中，只添加一个 Repository Secret：`CLOUDFLARE_API_TOKEN`。不要把 Token 写入仓库文件。
3. 将本仓库更改推送到 `main`，或手动运行 `Deploy autumn dashboard to Cloudflare Pages` 工作流。工作流自动识别账号，创建或复用名为 `autumn-dashboard` 的 D1 数据库，以及名为 `autumn-dashboard-<账号 ID 前 12 位>` 的 Direct Upload Pages 项目，生成 `DB` 绑定，执行迁移，配置加密访问口令，最后部署网页。以后相关文件推送到 `main` 会重复执行；已有资源会复用，迁移只应用尚未执行的文件。

部署日志会显示实际 Pages 项目名，访问地址为 `https://<项目名>.pages.dev`。项目名包含账号 ID 后缀，以避免常见名称冲突。网页访问口令由部署脚本使用 API Token 派生，**不是** Cloudflare API Token 本身。要在自己的电脑上查看口令，设置同一个环境变量并执行：

```powershell
cd web
$env:CLOUDFLARE_API_TOKEN = '<你的 Cloudflare API Token>'
npm run access-token
```

不要把 API Token 输入网页。更换 API Token 后，下次部署会更新网页访问口令；旧口令随之失效。2026-11-13 已从本机完成首次远端部署，验证正式域名根页面 200、未授权 API 401、授权读取 200，以及空数据的远端写入和回读；已在浏览器验证登录显示看板，完整业务交互仍需验收。

此项目包含个人投递和面试信息。请勿把真实 `applications.json`、口令、Cloudflare Token 或生成的 `wrangler.jsonc` 提交到 Git。

## 本地构建和预览

安装 Rust、`wasm32-unknown-unknown` target、`wasm-pack`、Node.js 22+。在 Windows GNU 工具链下，构建缓存建议放在纯英文路径，避免旧 MinGW 无法链接中文路径。

网页构建会将 `public/fonts/NotoSansSC-Regular.otf` 作为独立静态资源复制到 `dist/fonts/`，在 Slint 事件循环开始前注册为中文回退字体。字体采用 SIL Open Font License 1.1，许可文本随字体保存在 `public/fonts/OFL.txt`。

```powershell
cd web
npm ci
$env:CARGO_TARGET_DIR = 'C:\temp\autumn-web-target'
npm run build
$env:CLOUDFLARE_D1_DATABASE_ID = '00000000-0000-0000-0000-000000000000' # 仅供本地 D1 预览
npm run configure
npx wrangler d1 migrations apply DB --local
```

在 `web/.dev.vars` 写入 `DASHBOARD_TOKEN=<本地测试口令>`（至少 24 字符），然后运行 `npm run dev`，浏览器访问 Wrangler 显示的本地地址。`.dev.vars` 已被 Git 忽略。

## 将本地数据导入云端

只有在云端没有投递时运行一次。脚本先读取云端版本，若已有投递则停止，不覆盖现有数据；原始 JSON 文件保持不变。建议先备份源文件。

```powershell
cd web
$env:IMPORT_FILE = 'D:\path\to\applications.json'
$env:DASHBOARD_URL = 'https://<部署日志中的项目名>.pages.dev'
$env:DASHBOARD_TOKEN = '<npm run access-token 输出的网页访问口令>'
npm run import
```

## 数据协议

`GET /api/data` 返回 `{revision, applications}`，`PUT /api/data` 接收同样结构。`applications` 沿用原生版 camelCase JSON 结构，并保留未知字段和 `postponed` 状态。D1 使用一行 JSON 保存看板文档，`revision` 条件更新阻止多窗口的旧版本覆盖新数据。浏览器会话中保存口令，关闭会话后需重新输入。所有 API 响应禁用缓存。

Cloudflare D1 当前单行/单字符串限制为 2,000,000 字节，因此接口将单次文档写入限制为 1.9 MB；若数据增长到这一规模，需要迁移为按投递拆行的表结构。

网页目前沿用桌面界面布局，适合桌面尺寸浏览器；托盘、本地文件目录和本机 API 属于桌面功能，不在网页版中提供。导航栏“云端数据”显示数据位置，“刷新”重新加载页面。
