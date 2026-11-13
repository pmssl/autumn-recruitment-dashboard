import { test } from "node:test";
import assert from "node:assert/strict";
import { onRequestGet, onRequestPut } from "./data.js";

const token = "test-access-token-longer-than-24-characters";

function database() {
  let row;
  return {
    prepare(sql) {
      let args = [];
      return {
        bind(...values) { args = values; return this; },
        async first() { return row && { ...row }; },
        async run() {
          if (sql.startsWith("INSERT OR IGNORE")) {
            row ??= { revision: 0, payload: "[]" };
            return { meta: { changes: 1 } };
          }
          if (sql.startsWith("UPDATE")) {
            if (row.revision !== args[1]) return { meta: { changes: 0 } };
            row = { revision: row.revision + 1, payload: args[0] };
            return { meta: { changes: 1 } };
          }
          throw new Error(`Unexpected SQL: ${sql}`);
        },
      };
    },
  };
}

function context(db, method = "GET", body, key = token) {
  return {
    env: { DB: db, DASHBOARD_TOKEN: token },
    request: new Request("https://example.pages.dev/api/data", {
      method,
      headers: { Authorization: `Bearer ${key}` },
      body: body === undefined ? undefined : JSON.stringify(body),
    }),
  };
}

test("requires the server-side access token", async () => {
  const db = database();
  assert.equal((await onRequestGet(context(db, "GET", undefined, "wrong"))).status, 401);
  assert.equal((await onRequestGet({ ...context(db), env: { DB: db } })).status, 401);
});

test("reads and writes the current JSON model with optimistic concurrency", async () => {
  const db = database();
  assert.deepEqual(await (await onRequestGet(context(db))).json(), { revision: 0, applications: [] });
  const applications = [{
    id: "a1", company: "示例公司", position: "开发", createdAt: "2026-11-13T00:00:00Z",
    updatedAt: "2026-11-13T00:00:00Z", extraFromOldVersion: 7,
    stages: [{ id: "s1", applicationId: "a1", type: "interview", title: "面试",
      status: "postponed", sortOrder: 10, notes: "保留原始备注",
      createdAt: "2026-11-13T00:00:00Z", updatedAt: "2026-11-13T00:00:00Z" }],
  }];
  const saved = await onRequestPut(context(db, "PUT", { revision: 0, applications }));
  assert.equal(saved.status, 200);
  assert.equal((await saved.json()).revision, 1);
  assert.deepEqual((await (await onRequestGet(context(db))).json()).applications, applications);
  const stale = await onRequestPut(context(db, "PUT", { revision: 0, applications: [] }));
  assert.equal(stale.status, 409);
  assert.deepEqual((await (await onRequestGet(context(db))).json()).applications, applications);
});

test("rejects invalid ownership and status before changing data", async () => {
  const db = database();
  const invalid = [{ id: "a1", company: "示例", position: "开发", stages: [
    { id: "s1", applicationId: "other", type: "interview", title: "面试", status: "other", sortOrder: 10 },
  ] }];
  assert.equal((await onRequestPut(context(db, "PUT", { revision: 0, applications: invalid }))).status, 400);
  assert.deepEqual((await (await onRequestGet(context(db))).json()).applications, []);
});
