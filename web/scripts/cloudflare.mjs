import { createHmac } from "node:crypto";

const API = "https://api.cloudflare.com/client/v4";

function validateName(name) {
  if (!/^[a-z0-9][a-z0-9-]*$/.test(name)) {
    throw new Error(`Cloudflare 资源名称无效：${name}`);
  }
  return name;
}

export function deriveDashboardToken(apiToken) {
  if (!apiToken || apiToken.length < 24) throw new Error("缺少有效的 CLOUDFLARE_API_TOKEN");
  return createHmac("sha256", apiToken)
    .update("autumn-dashboard-pages-access:v1")
    .digest("hex");
}

export async function ensureResources({
  apiToken,
  projectName = "autumn-dashboard",
  databaseName = "autumn-dashboard",
  fetchImpl = fetch,
}) {
  deriveDashboardToken(apiToken);
  validateName(projectName);
  validateName(databaseName);

  async function request(path, options = {}, allow404 = false) {
    let response;
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        response = await fetchImpl(`${API}${path}`, {
          ...options,
          headers: {
            Authorization: `Bearer ${apiToken}`,
            ...(options.body ? { "Content-Type": "application/json" } : {}),
          },
        });
        break;
      } catch (error) {
        if (attempt === 2 || (options.method && options.method !== "GET")) throw error;
        await new Promise((resolve) => setTimeout(resolve, 500 * (attempt + 1)));
      }
    }
    if (allow404 && response.status === 404) return null;
    const body = await response.json().catch(() => ({}));
    if (!response.ok || body.success !== true) {
      const reason = body.errors?.map((error) => error.message).filter(Boolean).join("; ") || response.statusText;
      throw new Error(`Cloudflare API ${response.status} ${path}: ${reason}`);
    }
    return body;
  }

  const accounts = await request("/accounts?page=1&per_page=50");
  const totalAccounts = accounts.result_info?.total_count ?? accounts.result?.length ?? 0;
  if (totalAccounts !== 1 || accounts.result?.length !== 1) {
    throw new Error("API Token 必须只授权一个 Cloudflare 账号，避免自动部署到错误账号");
  }
  const accountId = accounts.result[0].id;
  if (!/^[a-f\d]{32}$/i.test(accountId)) throw new Error("Cloudflare 返回了无效账号 ID");
  const accountPath = `/accounts/${accountId}`;

  const databases = await request(`${accountPath}/d1/database?name=${encodeURIComponent(databaseName)}&per_page=100`);
  const exact = (databases.result || []).filter((database) => database.name === databaseName);
  if (exact.length > 1) throw new Error(`D1 数据库名称重复：${databaseName}`);
  let databaseId = exact[0]?.uuid;
  let createdDatabase = false;
  if (!databaseId) {
    const created = await request(`${accountPath}/d1/database`, {
      method: "POST", body: JSON.stringify({ name: databaseName }),
    });
    databaseId = created.result?.uuid;
    createdDatabase = true;
  }
  if (!/^[a-f\d]{8}(?:-[a-f\d]{4}){3}-[a-f\d]{12}$/i.test(databaseId || "")) {
    throw new Error("Cloudflare 返回了无效 D1 数据库 ID");
  }

  const actualProjectName = `${projectName}-${accountId.slice(0, 12)}`;
  let project = await request(`${accountPath}/pages/projects/${actualProjectName}`, {}, true);
  let createdProject = false;
  if (!project) {
    project = await request(`${accountPath}/pages/projects`, {
      method: "POST", body: JSON.stringify({ name: actualProjectName, production_branch: "main" }),
    });
    createdProject = true;
  }
  if (project.result?.source?.type === "github" || project.result?.source?.type === "gitlab") {
    throw new Error(`Pages 项目 ${actualProjectName} 已连接 Git 仓库；为避免重复部署，请使用一个未占用的项目名`);
  }
  if (project.result?.name !== actualProjectName) throw new Error("Cloudflare 返回了意外的 Pages 项目");
  return { accountId, databaseId, projectName: actualProjectName, databaseName, createdDatabase, createdProject };
}
