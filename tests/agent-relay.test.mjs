import assert from "node:assert/strict";
import { afterEach, mock, test } from "node:test";
import { relayAgentRequest } from "../lib/agent-relay.ts";

const origin = "https://hoodit-sepia.vercel.app";
const credential = "Bearer test-only-widget-token";
const request = (path = "chat", headers = {}, method = "POST") => new Request(`${origin}/api/agent/${path}`, {
  method, headers: { authorization: credential, origin, ...headers },
  ...(method === "POST" ? { body: JSON.stringify({ applicationId: "2938613" }) } : {}),
});
afterEach(() => mock.restoreAll());

test("forwards the caller token and bound origin, not cookies or server credentials", async () => {
  const upstream = mock.method(globalThis, "fetch", async (url, init) => {
    assert.equal(url, "https://chat.aomi.dev/v1/agent/chat");
    assert.equal(init.headers.get("authorization"), credential);
    assert.equal(init.headers.get("origin"), origin);
    assert.equal(init.headers.get("cookie"), null);
    assert.equal(init.headers.get("x-forwarded-host"), null);
    assert.equal(init.headers.get("idempotency-key"), "test-id");
    assert.equal(init.redirect, "manual");
    assert.equal(init.cache, "no-store");
    assert.deepEqual(JSON.parse(new TextDecoder().decode(init.body)), { applicationId: "2938613" });
    return Response.json({ ok: true });
  });
  const response = await relayAgentRequest(request("chat", { cookie: "test-cookie", "idempotency-key": "test-id", "x-forwarded-host": "attacker.invalid" }));
  assert.equal(response.status, 200);
  assert.equal(response.headers.get("cache-control"), "no-store");
  assert.equal(upstream.mock.callCount(), 1);
});

test("rejects missing credentials without contacting Aomi", async () => {
  const upstream = mock.method(globalThis, "fetch", () => { throw new Error("must not call"); });
  assert.equal((await relayAgentRequest(new Request(`${origin}/api/agent/chat`, { method: "POST" }))).status, 401);
  assert.equal(upstream.mock.callCount(), 0);
});

test("rejects foreign origins and browser cross-site requests", async () => {
  assert.equal((await relayAgentRequest(request("chat", { origin: "https://attacker.invalid" }))).status, 403);
  assert.equal((await relayAgentRequest(request("chat", { "sec-fetch-site": "cross-site" }))).status, 403);
});

test("limits paths and methods", async () => {
  for (const path of ["account", "chat/abc/unknown", "chat/%2Faccount", "sessions/abc/keys"]) {
    assert.equal((await relayAgentRequest(request(path))).status, 404);
  }
  assert.equal((await relayAgentRequest(request("chat", {}, "DELETE"))).status, 404);
});

test("polling carries query and the same authorization without a body", async () => {
  mock.method(globalThis, "fetch", async (url, init) => {
    assert.equal(url, "https://chat.aomi.dev/v1/agent/chat/test-session?cursor=test-cursor&wait=1000");
    assert.equal(init.headers.get("authorization"), credential);
    assert.equal(init.body, undefined);
    return Response.json({ events: [] });
  });
  assert.equal((await relayAgentRequest(request("chat/test-session?cursor=test-cursor&wait=1000", {}, "GET"))).status, 200);
});

test("preserves authorization rejection, strips upstream cookies", async () => {
  mock.method(globalThis, "fetch", async () => Response.json({ error: "invalid_token" }, { status: 401, headers: { "set-cookie": "test-only" } }));
  const response = await relayAgentRequest(request());
  assert.equal(response.status, 401);
  assert.equal(response.headers.get("set-cookie"), null);
});

test("never follows redirects or exposes provider failures", async () => {
  for (const status of [302, 500]) {
    mock.method(globalThis, "fetch", async () => new Response("private upstream detail", { status }));
    const response = await relayAgentRequest(request());
    assert.equal(response.status, 502);
    assert.equal((await response.text()).includes("private upstream detail"), false);
    mock.restoreAll();
  }
  mock.method(globalThis, "fetch", async () => { throw new Error("private upstream detail"); });
  assert.equal((await relayAgentRequest(request())).status, 502);
});
