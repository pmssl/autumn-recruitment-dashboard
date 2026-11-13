import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function writeConfig({
  projectName = "autumn-dashboard", databaseName = "autumn-dashboard", databaseId,
}) {
  if (!databaseId || !/^[a-f\d]{8}(?:-[a-f\d]{4}){3}-[a-f\d]{12}$/i.test(databaseId)) {
    throw new Error("缺少有效的 D1 数据库 UUID");
  }
  const config = {
    name: projectName,
    pages_build_output_dir: "./dist",
    compatibility_date: "2026-11-01",
    d1_databases: [{
      binding: "DB", database_name: databaseName, database_id: databaseId,
      migrations_dir: "migrations",
    }],
  };
  writeFileSync(resolve(import.meta.dirname, "../wrangler.jsonc"), `${JSON.stringify(config, null, 2)}\n`);
  console.log(`已配置 Pages 项目 ${projectName} 与 D1 绑定 DB`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  writeConfig({
    projectName: process.env.CLOUDFLARE_PAGES_PROJECT || "autumn-dashboard",
    databaseName: process.env.CLOUDFLARE_D1_DATABASE_NAME || "autumn-dashboard",
    databaseId: process.env.CLOUDFLARE_D1_DATABASE_ID,
  });
}
