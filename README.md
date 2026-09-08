# Hoodit

Hoodit is a Telegram-first trading assistant for Robinhood Stock Tokens on Robinhood Chain. This repository contains both the public landing page and the hosted Aomi application.

## Repository layout

- `app/` and `public/` — Next.js landing page and product UI
- `apps/hoodit/` — Rust dynamic application loaded by Aomi
- `.aomi/config.json` — Aomi Project manifest used by Build's community repository import
- `Cargo.toml` — shared Rust workspace and backend-compatible Aomi SDK pin

The repository remains plural so future products, such as a separately permissioned copy-trading app, can live beside Hoodit without expanding its trust boundary.

## Landing page

Requires Node.js 24.3 or newer and npm 10.9.2.

```bash
npm ci
npm run dev
```

Use `npm test`, `npm run lint` and `npm run build` before publishing frontend changes.

### In-page chat

`/app` mounts the native Aomi widget, pinned to Hoodit application `2938613`.
Guest and wallet sessions are issued directly by `chat.aomi.dev` and bound to
the browser origin. Agent requests use `/api/agent/*` on this site because the
hosted `/v1/agent/*` endpoint does not currently supply cross-origin CORS headers.
The relay forwards the caller's bearer unchanged with this site's origin; Aomi
still validates identity, scope and session ownership. It does not forward
cookies, mint credentials, follow redirects, or use a shared server API key.
Guest credentials are page-scoped, so reloading starts a fresh conversation;
local thread-ID persistence is disabled to avoid restoring another guest's session.

Assistant UI dependencies are pinned through `overrides` to avoid the render
loop and incompatible Markdown peer dependency in the freely resolved versions.
When upgrading, verify rendering and a real read-only chat response in the
browser, not just a successful build. Regression tests cover the relay's route,
origin, credential and error boundaries. Wallet signing requires separate tests.

## Aomi application

The workspace pins `aomi-sdk = "=4.0.0"`, matching the Aomi backend runtime. The Hoodit app itself declares no API-key secrets; market data is public and wallet indexing stays backend-owned.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
aomi-build sdk check --path . --required-version 4.0.0
```

The deterministic application scenario lives at `apps/hoodit/test.json`.

## Connect to Aomi Build

In **Deployments → New app → Connect an existing repository**, enter:

```text
aomi-labs/hoodit-apps
```

The root Project manifest selects the `community` platform and publishes `apps/hoodit/aomi.toml`. Commit and push changes before importing so Build can read the same revision.
