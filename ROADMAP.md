# Janus roadmap

Where Janus stands today and what's queued for future work. Sorted by
distance-to-ship: the top items are concrete next moves; the bottom
items are multi-day research projects with clear entry points.

## Shipped (current state, `main`)

### Core primitives
- 6 Pinocchio programs: conditional-tokens, lmsr-market (CPMM v1),
  **lmsr-true-market (true LMSR, Q32.32-priced)**,
  slot-height-resolver, pyth-price-resolver (with feed_id +
  staleness checks + Anchor discriminator), market-factory.
- TypeScript SDK with high-level `createMarketWithSlotResolver()` flow.

### Testing
- 14 Mollusk integration tests covering real SPL Token CPIs.
- Localnet end-to-end test against `solana-test-validator`.
- Devnet deploy script + canonical demo script.

### Formal verification
- 107 Kani BMC harnesses verifying transition properties.
- 99+ Lean theorems (Mathlib-enabled, all build via `lake build`):
  - **conditional-tokens**: 35 theorems incl. collateral conservation
    (`vault + user_balance == initial_collateral`)
  - **lmsr-market**: 21 (swap_no_drain, fee_bps_bounded, status_monotone)
  - **lmsr-true-market**: 24 (vault_eq_initial_plus_net_flow,
    status_monotone, subsidy_field_stable, b_field_stable across
    init/buy/sell/resolve)
  - **slot-height-resolver**: 6
  - **pyth-price-resolver**: 12
  - **market-factory**: 1
  - **account_layer** (multi-user Map[8] Account): 12/12 proven
    (Aristotle filled the 4 burn/transfer sorries on 2026-05-21,
    no non-standard axioms)
- janus-lmsr-math crate: Q32.32 fixed-point arithmetic, exp/ln, full
  LMSR cost function. 19 tests passing (15 unit + 4 proptest).

### Infrastructure
- 5-job GitHub Actions CI (rust, kani, qedgen, lean matrix x6, sdk).
- QEDGen spec validation on every push.

## Queued (small, well-scoped)

### Devnet deploy + canonical demo
Generate fresh program keypairs for devnet, deploy via
`scripts/devnet/deploy.sh`, run the demo with `node
scripts/devnet/demo.mjs`. Hooks into the existing SDK; no new on-chain
code. **~half a day.**

### Real Pyth feed_id + staleness on devnet
The pyth-price-resolver now validates discriminator + feed_id +
staleness. Remaining: hook to a live Pyth feed (SOL/USD on devnet),
write an end-to-end integration test, document the operator playbook
for choosing `max_staleness_slots`. **~1 day.**

### Aristotle pass for outstanding sorries — **done**
4 sorry'd theorems in account_layer (burn_yes, burn_no, two transfer
branches) submitted to Aristotle on 2026-05-21 (project
`2e3590b5-3b70-4a48-8ca2-06ed9ac47f10`); proofs landed the same day.
Tactic mix: `sum_update_proj_bilinear` + `lia` for burn arithmetic;
`grind` (and `grind +splitImp`) for the transfer same-index branches;
two bilinear applications + `grind` for the distinct-index transfer
branches. account_layer now 12/12 proven, depending only on standard
axioms (propext, Classical.choice, Quot.sound). Same flow available
for the true-LMSR proofs once written.

## Research projects (multi-day)

### True LMSR — **shipped (2026-05-21)**

`programs/lmsr-true-market/` is live with the collateral-in /
outcome-out model. Pool mints/burns YES + NO; pricing via
`janus-lmsr-math::{buy_yes_cost, buy_no_cost}` on Q32.32.

* **CU budget**: Init 9.4K, Buy 23.5K, Sell 23.5K — well under
  the 50K target. Taylor-9 exp stays.
* **Mollusk integration tests** (2/2 passing): full init → buy →
  sell round-trip plus a subsidy-below-b·ln(2) reject case.
* **Lean spec** (24 theorems, all proven, `lake build` clean):
  vault conservation, status monotone, b + initial_subsidy
  immutability, with the abstract `net_flow` ghost variable. The
  next-layer claim — `net_flow ≥ -b·ln(2)` ⇒ bounded loss — needs
  modeling LMSR cost over reals; queued for an Aristotle pass once
  the analysis lemmas are formalized.

### Multi-user `Map[N]` extensions

`formal_verification/account_layer/` ships the multi-user model with
all 12 obligations proven (Aristotle closed the last 4 on 2026-05-21).

Beyond closing the sorries, the next research extension is to **wire
account_layer as a callee interface from conditional-tokens via the
SPL Token program** — making the abstract multi-user invariant
literally compose with the real CPI chain. This would let us prove
"the entire Janus + SPL Token interaction preserves sum invariants"
as a theorem.

### Conditional-tokens scalar markets

Generalise the YES/NO binary primitive to N-outcome scalar markets
(e.g., for `"SOL price at end of month"` with `[<$200, $200-300,
$300-400, >$400]` buckets). Scope-creep flag: only worth doing if
demand is concrete. Generalising touches every program + spec + Lean
proof.

## Distribution (when/if Janus moves past "side quest")

- Mainnet program keypairs (separate from devnet); generate fresh
  and guard authority carefully.
- Audit pass — Code4rena or OtterSec, focused on conditional-tokens
  (highest blast radius).
- One curated frontend launched against a small set of markets.
- Launch post leaning on the verification depth: "175+ mechanized
  proofs, formal collateral conservation theorem, full BMC + Lean
  inductive coverage" — currently rare on Solana, real differentiator.

That's distribution, not building. Stay clearly on one side at a time.
