const routes = [
  { path: /^chat$/, methods: ["POST"] },
  { path: /^chat\/[^/]+$/, methods: ["GET"] },
  { path: /^chat\/[^/]+\/interrupt$/, methods: ["POST"] },
  { path: /^chat\/[^/]+\/actions\/[^/]+\/result$/, methods: ["POST"] },
  { path: /^sessions$/, methods: ["GET"] },
  { path: /^sessions\/[^/]+$/, methods: ["GET", "PATCH", "DELETE"] },
];

/** Transport only: Aomi validates the caller's origin-bound widget session.
 * Never substitute server credentials, forward cookies, or follow redirects.
 */
export async function relayAgentRequest(request: Request): Promise<Response> {
  const url = new URL(request.url);
  const path = url.pathname.replace(/^\/api\/agent\//, "");
  if (!/^\/api\/agent\//.test(url.pathname) || /[%\\]/.test(path) ||
      !routes.some((route) => route.path.test(path) && route.methods.includes(request.method))) {
    return Response.json({ error: "Not found" }, { status: 404 });
  }
  const origin = request.headers.get("origin");
  if ((origin && origin !== url.origin) || request.headers.get("sec-fetch-site") === "cross-site") {
    return Response.json({ error: "Origin not allowed" }, { status: 403 });
  }
  const authorization = request.headers.get("authorization");
  if (!authorization?.startsWith("Bearer ")) {
    return Response.json({ error: "Chat session required" }, { status: 401 });
  }
  const headers = new Headers({ authorization, origin: url.origin });
  for (const name of ["content-type", "accept", "idempotency-key"]) {
    const value = request.headers.get(name);
    if (value) headers.set(name, value);
  }
  try {
    const upstream = await fetch(`https://chat.aomi.dev/v1/agent/${path}${url.search}`, {
      method: request.method,
      headers,
      body: ["GET", "HEAD"].includes(request.method) ? undefined : await request.arrayBuffer(),
      redirect: "manual",
      cache: "no-store",
      signal: request.signal,
    });
    if ((upstream.status >= 300 && upstream.status < 400) || upstream.status >= 500) {
      return Response.json({ error: "Chat temporarily unavailable" }, { status: 502 });
    }
    return new Response(upstream.body, {
      status: upstream.status,
      headers: {
        "content-type": upstream.headers.get("content-type") || "application/json",
        "cache-control": "no-store",
        "x-content-type-options": "nosniff",
      },
    });
  } catch {
    return Response.json({ error: "Chat temporarily unavailable" }, { status: 502 });
  }
}
