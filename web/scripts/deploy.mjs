import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { ensureResources, deriveDashboardToken } from "./cloudflare.mjs";
import { writeConfig } from "./configure.mjs";

const root = resolve(import.meta.dirname, "..");
const apiToken = process.env.CLOUDFLARE_API_TOKEN;
const projectName = process.env.CLOUDFLARE_PAGES_PROJECT || "autumn-dashboard";
const databaseName = process.env.CLOUDFLARE_D1_DATABASE_NAME || "autumn-dashboard";
const resources = await ensureResources({ apiToken, projectName, databaseName });
writeConfig({ projectName: resources.projectName, databaseName, databaseId: resources.databaseId });
const wrangler = resolve(root, "node_modules/wrangler/bin/wrangler.js");
const env = { ...process.env, CLOUDFLARE_ACCOUNT_ID: resources.accountId };

function runWrangler(args, input) {
  const result = spawnSync(process.execPath, [wrangler, ...args], {
    cwd: root,
    env,
    stdio: input === undefined ? "inherit" : ["pipe", "inherit", "inherit"],
    input,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`wrangler ${args.slice(0, 3).join(" ")} 失败（退出码 ${result.status}）`);
}

console.log(`${resources.createdDatabase ? "创建" : "复用"} D1 ${databaseName}`);
console.log(`${resources.createdProject ? "创建" : "复用"} Pages ${resources.projectName}`);
runWrangler(["d1", "migrations", "apply", "DB", "--remote"]);
const dashboardToken = deriveDashboardToken(apiToken);
runWrangler(["pages", "secret", "put", "DASHBOARD_TOKEN", "--project-name", resources.projectName], `${dashboardToken}\n`);
runWrangler(["pages", "deploy", "dist", "--project-name", resources.projectName, "--branch", "main"]);
console.log(`Pages ${resources.projectName} 部署完成；访问口令可在本地运行 npm run access-token 重新计算。`);
