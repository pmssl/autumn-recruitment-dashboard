import { test } from "node:test";
import assert from "node:assert/strict";
import { deriveDashboardToken, ensureResources } from "./cloudflare.mjs";

const token = "cloudflare-test-token-with-enough-entropy";
const accountId = "a".repeat(32);
const databaseId = "12345678-1234-1234-1234-123456789abc";
const projectName = `autumn-dashboard-${accountId.slice(0, 12)}`;

function fakeCloudflare(existing = false, accountCount = 1) {
  const calls = [];
  const fetchImpl = async (url, options = {}) => {
    const path = new URL(url).pathname;
    const method = options.method || "GET";
    calls.push(`${method} ${path}`);
    assert.equal(options.headers.Authorization, `Bearer ${token}`);
    if (path === "/client/v4/accounts") {
      return Response.json({ success: true, result: [{ id: accountId }],
        result_info: { total_count: accountCount } });
    }
    if (path.endsWith("/d1/database") && method === "GET") {
      return Response.json({ success: true,
        result: existing ? [{ name: "autumn-dashboard", uuid: databaseId }] : [] });
    }
    if (path.endsWith("/d1/database") && method === "POST") {
      assert.deepEqual(JSON.parse(options.body), { name: "autumn-dashboard" });
      return Response.json({ success: true, result: { uuid: databaseId } });
    }
    if (path.endsWith(`/pages/projects/${projectName}`)) {
      return existing
        ? Response.json({ success: true, result: { name: projectName } })
        : Response.json({ success: false, errors: [{ message: "not found" }] }, { status: 404 });
    }
    if (path.endsWith("/pages/projects") && method === "POST") {
      assert.deepEqual(JSON.parse(options.body), { name: projectName, production_branch: "main" });
      return Response.json({ success: true, result: { name: projectName } });
    }
    throw new Error(`Unexpected ${method} ${path}`);
  };
  return { fetchImpl, calls };
}

test("first deployment creates D1 and Pages from one scoped token", async () => {
  const fake = fakeCloudflare();
  const result = await ensureResources({ apiToken: token, fetchImpl: fake.fetchImpl });
  assert.equal(result.accountId, accountId);
  assert.equal(result.databaseId, databaseId);
  assert.equal(result.projectName, projectName);
  assert.equal(result.createdDatabase, true);
  assert.equal(result.createdProject, true);
  assert.equal(fake.calls.filter((call) => call.startsWith("POST ")).length, 2);
});

test("later deployment reuses both resources", async () => {
  const fake = fakeCloudflare(true);
  const result = await ensureResources({ apiToken: token, fetchImpl: fake.fetchImpl });
  assert.equal(result.createdDatabase, false);
  assert.equal(result.createdProject, false);
  assert.equal(fake.calls.filter((call) => call.startsWith("POST ")).length, 0);
});

test("temporary transport failure is retried", async () => {
  const fake = fakeCloudflare(true);
  let attempts = 0;
  const result = await ensureResources({ apiToken: token, fetchImpl: async (...args) => {
    if (attempts++ === 0) throw new Error("temporary connection reset");
    return fake.fetchImpl(...args);
  } });
  assert.equal(result.projectName, projectName);
  assert.equal(attempts, fake.calls.length + 1);
});

test("multiple accounts stop before creating anything", async () => {
  const fake = fakeCloudflare(false, 2);
  await assert.rejects(ensureResources({ apiToken: token, fetchImpl: fake.fetchImpl }), /只授权一个/);
  assert.equal(fake.calls.length, 1);
});

test("dashboard login token is derived without exposing the Cloudflare token", () => {
  const derived = deriveDashboardToken(token);
  assert.match(derived, /^[a-f\d]{64}$/);
  assert.equal(derived, deriveDashboardToken(token));
  assert.notEqual(derived, deriveDashboardToken(`${token}-rotated`));
  assert.ok(!derived.includes(token));
});
