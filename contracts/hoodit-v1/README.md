# Hoodit v1.1.0 tool contracts

This bundle is the canonical public contract for Hoodit’s two skill-owned, seven read-only tools on Robinhood Chain (`4663`). Market reads use GeckoTerminal. Wallet inventory and exact balances use Blockscout’s free authenticated API. Optional valuation uses read-only LI.FI quotes; actual swaps remain in the inherited host execution flow.

## Amendments from 1.0.0

- `blockscout` replaces `etherscan`; no paid data subscription is required.
- `hoodit_get_portfolio` accepts an opaque `cursor` instead of `page`/`page_size`, consumes one Blockscout provider page, and includes native ETH only initially. Its first-page cursor is nullable because strict model tool schemas require every declared property; later pages accept only an exact returned continuation.
- Portfolio reads default to balances only (`include_quotes=false`). Requested valuation is limited to 20 non-USDG quote attempts and a 30-second overall deadline. Unscheduled holdings remain visible with `budget_exhausted` or `deadline_exceeded`.
- A response contains at most 50 ERC-20 rows plus initial native ETH. Complete-wallet totals require an initial page with no continuation and a successful native read.

`examples.json` includes synthetic continuation, 50-row, quote-budget, and wrong-wallet cursor cases.

## Validate

```bash
python3 validate_contracts.py
```

Validation covers schemas, fixtures, and documentation consistency. It does not call providers, compile Rust, validate host skill gating, or execute trades.

To validate serialized Rust results independently from Rust's own tests, dump
`{ "tool": "...", "input": {...}, "output": {...} }` cases to a directory and run:

```bash
python3 validate_contracts.py --implementation-fixtures path/to/generated-contract-fixtures
```

This mode requires valid emitted outputs for all seven tools. It catches wire
drift such as omitted required nulls, stale schema versions, invalid warning
objects, and response fields that Rust's compile-time types do not constrain.
