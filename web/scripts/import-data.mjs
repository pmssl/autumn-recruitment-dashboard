import { readFileSync } from "node:fs";

const path = process.env.IMPORT_FILE;
const base = process.env.DASHBOARD_URL;
const token = process.env.DASHBOARD_TOKEN;
if (!path || !base || !token) {
  throw new Error("请设置 IMPORT_FILE、DASHBOARD_URL 和 DASHBOARD_TOKEN");
}
const url = new URL("/api/data", base);
if (url.protocol !== "https:" && url.hostname !== "localhost" && url.hostname !== "127.0.0.1") {
  throw new Error("远端导入只允许 HTTPS");
}
const applications = JSON.parse(readFileSync(path, "utf8"));
if (!Array.isArray(applications)) throw new Error("导入文件必须是投递 JSON 数组");
const headers = { Authorization: `Bearer ${token}`, "Content-Type": "application/json" };
const currentResponse = await fetch(url, { headers });
if (!currentResponse.ok) throw new Error(`无法读取云端数据：HTTP ${currentResponse.status}`);
const current = await currentResponse.json();
if (current.applications.length !== 0) throw new Error("云端已有投递，已停止导入，避免覆盖现有数据");
const response = await fetch(url, {
  method: "PUT", headers,
  body: JSON.stringify({ revision: current.revision, applications }),
});
if (!response.ok) {
  const error = await response.json().catch(() => ({}));
  throw new Error(`导入失败：${error.error || `HTTP ${response.status}`}`);
}
console.log(`导入成功：${applications.length} 条投递。源文件未修改。`);
