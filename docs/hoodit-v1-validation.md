# Hoodit v1 validation record

Validated locally and against staging on 2026-09-17. This record separates
contract and build checks, direct live-provider reads, and deployed chat
evidence. No wallet transaction was prepared, signed, broadcast, or simulated
by these checks.

## Contract and frontend checks

```bash
python3 contracts/hoodit-v1/validate_contracts.py
PATH=/home/aron/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PATH npm test
npm run lint
npm run build
fixture_dir=$(mktemp -d)
HOODIT_CONTRACT_FIXTURE_DIR="$fixture_dir" cargo test --workspace
python3 contracts/hoodit-v1/validate_contracts.py --implementation-fixtures "$fixture_dir"
```

Results at the time of this record:

- `cargo test --workspace --locked` passed 19 unit tests and three contract
  tests. Formatting and `cargo clippy --workspace --all-targets -- -D warnings`
  also passed independently.
- The SDK 5.1.0 compile and manifest checks passed. The manifest contains
  exactly seven tool descriptors, two app skills, and no user-declared secret
  slots. Blockscout is resolved only from the host-injected call context; LI.FI
  is keyless. A locked standalone build of `apps/hoodit` outside the workspace
  passed.
- Contract validator: 69 schemas and 84 synthetic assertions passed across
  seven tools.
- Frontend relay: seven tests passed with the bundled Node runtime. The system
  Node 22 build lacks TypeScript stripping and is not a valid test runtime.
- ESLint and the optimized Next.js build passed.
- Rust adversarial input, generated-manifest schema, and deterministic
  seven-tool emitted-output tests passed. These
  enforce closed inputs, explicit-null rejection, cursor pagination, valuation
  opt-in, holding fraction bounds/defaults, and output-envelope compatibility.

The independent validator's `--implementation-fixtures DIR` mode validated 29
Rust-emitted input/output assertions covering all seven tools. Coverage includes
a successful echoed LI.FI quote, cursor continuation with page scope, native
holding, unknown-decimals partial output, typed missing-provider and invalid
argument errors, rejection of the string `"null"` as a cursor, and rejection of
a cursor bound to a different wallet. The deterministic
transport used local synthetic upstream responses and included high-precision
numeric candle lexemes; it made no live provider calls.

A separate read-only live Rust run exercised all seven tools and every emitted
envelope passed the canonical v1.1.0 schemas. A deliberate burst reached
GeckoTerminal's rate limit and returned the typed retryable `RATE_LIMITED`
error; the affected reads succeeded after the provider window recovered. Two
live 50-row Blockscout pages proved valid continuation replay, page scope, and
native-balance exclusion after page one. An additional exact holding read for
a nonzero WETH balance returned a `quoted` keyless LI.FI valuation. These
checks requested data and quotes only; they did not prepare, authorize, sign,
or broadcast a transaction.

## Sanitized live provider proof

Read-only probes confirmed:

- Blockscout authenticated inventory returned two consecutive 50-row pages;
  a separate EOA returned an 11-row terminal page. Continuations contained
  `id`, `value`, `items_count`, and nullable `fiat_value`.
- Blockscout exact ERC-20 and native balance endpoints succeeded under the
  chain-prefixed `/4663/api` path. Inventory balances and token decimals arrive
  as decimal strings. USDG decimals were reported as 6.
- GeckoTerminal token, v4 pool, candles, and trades reads succeeded. The v4
  pool identifier is a 32-byte opaque hex identifier rather than an address.
- A keyless same-chain LI.FI PONS-to-USDG read-only quote succeeded. Sanitized
  evidence retained token metadata and integer amount strings, with transaction
  request fields excluded.

These reads establish provider shape and availability at one point in time.
They do not establish future uptime, complete wallet snapshots across pages,
execution-route reliability, wallet authorization, or successful broadcasts.

## Staging chat and cursor remediation

Application `2937810` was first observed on deployed Hoodit v1.1.1. Both an
authenticated locked-chat run and a fresh guest-chat run activated
`hoodit/portfolio`, then invoked `hoodit_get_portfolio` with the sanitized
arguments `{"cursor":"null","refresh":false,"include_quotes":false,...}`.
The tool correctly rejected that fabricated continuation with
`INVALID_ARGUMENT`; provider credential resolution had already succeeded.

The root cause was the generated optional-string schema advertising
`default: null`, combined with insufficient first-page guidance. Hoodit v1.1.2
removes that schema default, explicitly instructs the model to omit
the cursor on page one, and continues to reject the string `"null"`, malformed
cursors, and cursors bound to another wallet. The staging smoke adapter now
retains sanitized legacy `tool_arguments`, emits the transcript before failing
an assertion, and binds guest tokens to the Chat staging origin rather than the
Build origin.

## Staging deployment and post-activation status

Source commit `bdc383a6fb5e020afa8dbe2773805f9e8e7235bd` was deployed only to
staging as deployment `dep_162207273_rff4cf0103f_bdc383a6fb5e`. Platform CI
run `35203743039` passed validation, the locked Linux build, and immutable
release publication. The resulting release tag is
`apps-162207273-rff4cf0103f-hoodit-bdc383a6fb5e`; its candidate commit is
`70e19346c160d4a3ac159b57062ad22ee6ad18cc`. The published plugin digest is
`sha256:31831334a17325ab48767d9746ce867081487be53534c246bda311e7d71ee5b4`.

The Build control plane reports application `2937810` active on that exact
release. Its deployment timeline labels the source commit current, and live
observability labels the release healthy on SDK 5.1.0. The published deployment
record retains `is_public: true`; the Environment view contains the builder-set
runtime `BLOCKSCOUT_API_KEY`, with its value hidden, and the locked Chat surface
does not ask an end user for a provider key. No transaction or signing activity
was recorded.

Post-activation app-bound chat verification is blocked upstream of Hoodit.
Fresh authenticated-browser and anonymous-browser turns, plus three fresh
origin-bound API attempts, are rejected before model or tool execution with a
backend 403. A no-app staging control turn made through the same guest bootstrap
completed successfully. During the rejected Hoodit attempts, live observability
continued to report the exact release healthy, `Inflight 0`, no loaded instance,
and unchanged tool-call counters; the app log recorded no post-activation tool
invocation. Earlier 409 `session_busy` responses likewise occurred with no
inflight work and later resolved to the consistent 403 admission failure.

Accordingly, the seven current-source tools are covered by the successful
direct live-provider run and schema validation above, but the deployed-chat
smoke cannot be counted as passed until the staging app-admission path accepts a
turn. Re-running activation was idempotent and left the same release active; no
production state or shared backend infrastructure was changed to work around
the platform failure.

Run the reusable smoke after application `2937810` is active on v1.1.2:

```bash
python3 scripts/hoodit-staging-smoke.py \
  --secret-file /path/to/optional-local-secrets.env
```

The script targets `https://chat-staging.aomi.dev`, obtains a fresh
origin-bound guest token, and keeps it in memory. Its default turns cover a
greeting, `hoodit_discover_pools`, and a balances-only
`hoodit_get_portfolio` read for the public fixture address. Each read must
produce the expected tool name and a non-error, non-null Hoodit envelope;
unrelated skill-activation events do not satisfy the check. It long-polls each
turn to completion and aborts if any signing action appears. Values loaded from
an existing `--secret-file` are removed from recorded output, and raw HTTP
error bodies and action payloads are never printed. The pre-fix v1.1.1 failure
above is not counted as a pass, and neither are the post-activation admission
failures described above.
