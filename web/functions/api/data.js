// D1 limits each string/row to 2,000,000 bytes; leave room for row metadata.
const MAX_BYTES = 1_900_000;

function json(value, status = 200) {
  return Response.json(value, {
    status,
    headers: { "Cache-Control": "no-store" },
  });
}

async function authorized(request, secret) {
  if (typeof secret !== "string" || secret.length < 24) return false;
  const supplied = request.headers.get("Authorization") || "";
  const expected = `Bearer ${secret}`;
  const encoder = new TextEncoder();
  const [left, right] = await Promise.all([
    crypto.subtle.digest("SHA-256", encoder.encode(supplied)),
    crypto.subtle.digest("SHA-256", encoder.encode(expected)),
  ]);
  const a = new Uint8Array(left);
  const b = new Uint8Array(right);
  let difference = 0;
  for (let i = 0; i < a.length; i++) difference |= a[i] ^ b[i];
  return difference === 0 && supplied.length === expected.length;
}

async function readState(db) {
  const select = "SELECT revision, payload FROM dashboard_state WHERE id = 1";
  const existing = await db.prepare(select).first();
  if (existing) return existing;
  await db.prepare("INSERT OR IGNORE INTO dashboard_state (id, revision, payload, updated_at) VALUES (1, 0, '[]', CURRENT_TIMESTAMP)").run();
  return db.prepare(select).first();
}

function validApplications(value) {
  if (!Array.isArray(value) || value.length > 10000) return false;
  const appIds = new Set();
  const stageIds = new Set();
  const statuses = new Set(["completed", "pending", "failed", "postponed"]);
  for (const app of value) {
    if (!app || typeof app !== "object" || Array.isArray(app) ||
        typeof app.id !== "string" || !app.id || appIds.has(app.id) ||
        typeof app.company !== "string" || !app.company.trim() ||
        typeof app.position !== "string" || !app.position.trim() ||
        typeof app.createdAt !== "string" || typeof app.updatedAt !== "string" ||
        (app.url != null && typeof app.url !== "string") ||
        (app.notes != null && typeof app.notes !== "string") ||
        !Array.isArray(app.stages)) return false;
    appIds.add(app.id);
    for (const stage of app.stages) {
      if (!stage || typeof stage !== "object" || Array.isArray(stage) ||
          typeof stage.id !== "string" || !stage.id || stageIds.has(stage.id) ||
          stage.applicationId !== app.id || typeof stage.type !== "string" ||
          typeof stage.title !== "string" || !statuses.has(stage.status) ||
          typeof stage.createdAt !== "string" || typeof stage.updatedAt !== "string" ||
          (stage.date != null && typeof stage.date !== "string") ||
          (stage.notes != null && typeof stage.notes !== "string") ||
          typeof stage.sortOrder !== "number" || !Number.isFinite(stage.sortOrder)) return false;
      stageIds.add(stage.id);
    }
  }
  return true;
}

async function handle(context, write) {
  if (!context.env.DB || !(await authorized(context.request, context.env.DASHBOARD_TOKEN))) {
    return json({ error: "未授权或服务尚未配置" }, 401);
  }
  try {
    if (!write) {
      const row = await readState(context.env.DB);
      return json({ revision: row.revision, applications: JSON.parse(row.payload) });
    }
    const bytes = await context.request.arrayBuffer();
    if (bytes.byteLength > MAX_BYTES) return json({ error: "数据超过 1.9 MB" }, 413);
    let body;
    try {
      body = JSON.parse(new TextDecoder().decode(bytes));
    } catch {
      return json({ error: "请求不是有效 JSON" }, 400);
    }
    if (!Number.isSafeInteger(body?.revision) || body.revision < 0 ||
        !validApplications(body.applications)) {
      return json({ error: "投递数据格式无效" }, 400);
    }
    const payload = JSON.stringify(body.applications);
    if (new TextEncoder().encode(payload).length > MAX_BYTES) return json({ error: "数据超过 1.9 MB" }, 413);
    await readState(context.env.DB);
    const result = await context.env.DB.prepare(
      "UPDATE dashboard_state SET payload = ?, revision = revision + 1, updated_at = CURRENT_TIMESTAMP WHERE id = 1 AND revision = ?"
    ).bind(payload, body.revision).run();
    if (result.meta.changes !== 1) {
      const current = await readState(context.env.DB);
      return json({ error: "数据已在别处修改，请刷新后重试", revision: current.revision }, 409);
    }
    return json({ revision: body.revision + 1 });
  } catch (error) {
    console.error("dashboard D1 error", error);
    return json({ error: "数据库操作失败" }, 500);
  }
}

export const onRequestGet = (context) => handle(context, false);
export const onRequestPut = (context) => handle(context, true);
