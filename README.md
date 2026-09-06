# Hoodit

Hoodit is a Telegram-first trading assistant for Robinhood Stock Tokens on Robinhood Chain. This repository contains both the public landing page and the hosted Aomi application.

## Repository layout

- `app/` and `public/` — Next.js landing page and product UI
- `apps/hoodit/` — Rust dynamic application loaded by Aomi
- `.aomi/config.json` — Aomi Project manifest used by Build's community repository import
- `Cargo.toml` — shared Rust workspace and backend-compatible Aomi SDK pin

The repository remains plural so future products, such as a separately permissioned copy-trading app, can live beside Hoodit without expanding its trust boundary.

## Landing page

Requires Node.js 24.3 or newer.

```bash
npm ci
npm run dev
```

Use `npm run lint` and `npm run build` before publishing frontend changes.

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
