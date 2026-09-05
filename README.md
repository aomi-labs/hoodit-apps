# hoodit-apps

Robinhood-focused Aomi applications. The repository is intentionally plural so
future products, such as a separately permissioned copy-trading app, can live
beside the first application without expanding its trust boundary.

## Applications

| App | Status | Purpose |
|---|---|---|
| `hoodit` | implemented | Research, inspect portfolios, and trade canonical Robinhood Stock Tokens on Robinhood Chain |

`hoodit` combines Robinhood's public Stock Token API with indexed Robinhood
Chain balances. It delegates transaction construction, simulation, and wallet
execution to Aomi's built-in `robinhood_stocks` and `lifi_swap` skills.

## Data sources

- Robinhood Stock Token API: asset metadata, prices, and corporate actions
- Robinhood Chain RPC: direct per-token wallet balances
- Alchemy Data API or Blockscout: indexed Robinhood Chain token balances
- Aomi host tools: current news, connected-wallet context, canonical token
  resolution, LI.FI quotes, simulation, and signing

Robinhood's public data endpoints need no credential. For portfolio reads,
`ALCHEMY_API_KEY` is preferred and must belong to an Alchemy app with Robinhood
Chain enabled. `BLOCKSCOUT_API_KEY` is an optional fallback; without either,
the app uses the public Robinhood Chain explorer endpoint.

## Local development

```sh
cargo test --manifest-path apps/hoodit/Cargo.toml
cargo fmt --manifest-path apps/hoodit/Cargo.toml -- --check
cargo clippy --manifest-path apps/hoodit/Cargo.toml --all-targets -- -D warnings
```

The real-model end-to-end spec lives at `apps/hoodit/test.json`. After building
the plugin, run it from `product-mono/aomi`:

```sh
AOMI_E2E_APP_PATH="$PWD/../../hoodit-apps/apps/hoodit/target/debug/libhoodit.dylib" \
  cargo test -p aomi-runtime --test local-app-e2e app_e2e_specs -- --nocapture
```
