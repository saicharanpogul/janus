# Janus

[![verify](https://github.com/saicharanpogul/janus/actions/workflows/verify.yml/badge.svg)](https://github.com/saicharanpogul/janus/actions/workflows/verify.yml)

A permissionless binary-markets primitive for Solana, designed for tail assets.

Verified end-to-end at every layer:
- **5 Pinocchio programs** build clean for SBF
- **14 Mollusk integration tests** exercise real SPL Token CPIs
- **Localnet E2E** runs the SDK against `solana-test-validator`
- **107 Kani BMC harnesses** verify spec transitions
- **75 Lean 4 theorems** prove all preservation obligations with Mathlib
- **Collateral conservation theorem** formally proven across every conditional-tokens handler

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

v0 — every program scaffolded and the SDK shipped. All Rust crates
build clean on host; SBF deploy + integration testing is the next
milestone.

```
janus/
├── crates/
│   └── resolver-interface/      ✅ shared resolver protocol types
├── programs/
│   ├── conditional-tokens/      ✅ asset layer: init, split, merge, redeem, resolve
│   ├── lmsr-market/             ✅ binary AMM (CPMM in v1; true LMSR planned)
│   ├── slot-height-resolver/    ✅ reference resolver — outcome at target slot
│   ├── pyth-price-resolver/     ✅ resolves YES/NO via Pyth feed crossing
│   └── market-factory/          ✅ on-chain registry for discovery
├── sdk/                         ✅ TypeScript client + createMarket helper
└── app/                         (frontend — next)
```

## Build

```bash
# Rust programs (host check):
cargo build --release --workspace

# SBF programs (for deploy):
cargo build-sbf

# TypeScript SDK:
cd sdk && pnpm install && pnpm build
```

Before deploying, replace each program's placeholder `declare_id!`
pubkey with a real keypair generated via `solana-keygen new -o
target/deploy/<program_name>-keypair.json`, and update the matching
constant in `sdk/src/constants.ts` plus the cross-references in
`market-factory/src/lib.rs`.

## Architecture

A market is the composition of four objects:

1. **A resolver state account**, owned by some resolver program that
   implements the standard interface defined in
   [`crates/resolver-interface`](./crates/resolver-interface).
2. **A `conditional-tokens` market PDA**, which mints YES + NO outcome
   tokens against deposited collateral, holds a vault of that collateral,
   and binds itself to a specific resolver at creation.
3. **An `lmsr-market` pool PDA**, holding reserves of YES + NO tokens
   contributed by the creator as subsidy, that quotes prices via a
   constant-product curve.
4. **A `market-factory` registration PDA**, recording the full bundle
   for trustless discovery.

Outcome tokens are plain SPL tokens. Any DEX, lending market, or vault
on Solana composes against them without knowing they're prediction-market
positions. That's the primitive: not a product, a substrate.

## License

Apache-2.0
