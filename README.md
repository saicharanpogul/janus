# Janus

A permissionless binary-markets primitive for Solana, designed for tail assets.

## What it is

Janus is the composable substrate for binary (YES/NO) markets on Solana. Four cooperating primitives:

1. **Conditional Tokens** — collateral splits into matching `YES` + `NO` SPL tokens, recombinable back to collateral; winning tokens redeem 1:1 after resolution.
2. **LMSR Market** — Logarithmic Market Scoring Rule (or pm-AMM) curve provides quotes from zero volume with bounded loss for the subsidizer; no LP bootstrap required.
3. **Resolver Registry** — standardized resolver interface with pluggable implementations: deterministic on-chain resolvers (Pyth, slot height, token balance, on-chain event) are free and instant; optimistic resolver with bonded dispute window for off-chain truth.
4. **Market Factory** — permissionless market deployment binding a resolver + LMSR curve + outcome token pair in one transaction.

Outcome tokens are plain SPL tokens, so every other Solana protocol composes natively without knowing it's a market.

## Why it matters

Existing prediction-market shapes on Solana either bridge to regulated off-chain liquidity (DFlow → Kalshi) or are verticalized into a single product (Drift BET, MetaDAO for futarchy). The permissionless on-chain primitive with deterministic-first resolution — designed for the long tail of markets nobody else lists — is open ground.

The "tail" thesis: ~70% of crypto-native market outcomes are deterministic on-chain facts (price crossings, governance votes, balance thresholds, transaction events). Those don't need a heavy optimistic-oracle layer — they need a standardized resolver interface so the market can read its truth for free.

## Status

Early. Scaffolding only. Built on Pinocchio for CU efficiency.

```
janus/
├── programs/
│   ├── conditional-tokens/   ← starting here
│   ├── lmsr-market/          (todo)
│   ├── resolver-registry/    (todo)
│   ├── market-factory/       (todo)
│   └── resolvers/            (todo: pyth, slot-height, token-balance, on-chain-event, optimistic)
├── sdk/                      (todo)
├── app/                      (todo)
└── docs/                     (todo)
```

## Build

```bash
cargo build-sbf
```

(Requires Solana toolchain. Pinocchio versions in `Cargo.toml` may need bumping to current.)

## License

Apache-2.0
