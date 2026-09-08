/** Relay only the Agent API. Session issuance and wallet login still talk to
 * Aomi directly, which binds the credential to this site's browser Origin.
 */
export const chatFetch: typeof fetch = async (input, init) => {
  const url = new URL(input instanceof Request ? input.url : String(input), window.location.href);
  if (url.origin !== "https://chat.aomi.dev" || !url.pathname.startsWith("/v1/agent/")) {
    return fetch(input, init);
  }
  const target = `${window.location.origin}/api/agent/${url.pathname.slice("/v1/agent/".length)}${url.search}`;
  if (!(input instanceof Request)) return fetch(target, init);
  const request = new Request(input, init);
  return fetch(target, {
    method: request.method,
    headers: request.headers,
    body: ["GET", "HEAD"].includes(request.method) ? undefined : await request.arrayBuffer(),
    signal: request.signal,
    cache: "no-store",
  });
};
