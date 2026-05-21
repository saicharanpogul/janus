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
- 105+ Lean theorems (Mathlib-enabled, all build via `lake build`):
  - **conditional-tokens**: 35 theorems incl. collateral conservation
    (`vault + user_balance == initial_collateral`)
  - **lmsr-market**: 21 (swap_no_drain, fee_bps_bounded, status_monotone)
  - **lmsr-true-market**: 24 state-machine theorems
    (vault_eq_initial_plus_net_flow, status_monotone, subsidy_field_stable,
    b_field_stable) + 6 real-analysis theorems in `LmsrCost.lean`
    (cost_at_zero, cost_lower_bound, cost_monotone_yes/no,
    bounded_loss, subsidizer_loss_bound). Proves the headline
    bounded-loss invariant: `vault ≥ initial_subsidy − b · log(2)`.
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

### Devnet deploy + canonical demo — **done (2026-05-21)**
All 6 programs deployed to devnet (see [DEVNET.md](DEVNET.md) for IDs
and operator playbook). Canonical demo market live at
`6W98N95Rko8jgMobawWtWQnrRNEfCjKvuRUV9RxxFQdv`. Cost: ~0.94 SOL for
the deploy.

### Real Pyth feed_id + staleness on devnet — **done (2026-05-21)**
The `scripts/devnet/pyth-verify.mjs` script fetches a `PriceUpdateV2`
account and confirms our hard-coded byte offsets (feed_id @ 41, price
@ 73, exponent @ 89, posted_slot @ 125) parse the on-chain layout
exactly. Operator playbook + `max_staleness_slots` guidance in
[DEVNET.md](DEVNET.md). Posting a fresh feed account requires the
`@pythnetwork/pyth-solana-receiver` SDK as documented there.

### Aristotle passes
* **account_layer** (2026-05-21, project `2e3590b5-...`): 4 sorry'd
  burn/transfer theorems closed via `sum_update_proj_bilinear` + `lia`
  / `grind`. 12/12 proven.
* **lmsr-true-market bounded-loss** (2026-05-21, project
  `0c1e9d43-cfd1-4f9f-94c8-ee303eafbf6e`): real-valued LMSR cost
  formalization in `LmsrCost.lean` — 3 lemmas closed by Aristotle:
  `cost_lower_bound` (via `max_cases` + `nlinarith` over
  `Real.log_exp` + `Real.log_le_log`), `cost_monotone_yes` /
  `cost_monotone_no` (via `mul_le_mul_of_nonneg_left` +
  `Real.log_le_log` + `Real.exp_le_exp.mpr` + `gcongr`). Combined
  with the hand-written `bounded_loss` and `subsidizer_loss_bound`,
  this gives a fully-proven, axiom-free `vault ≥ initial_subsidy -
  b · log(2)` floor.

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
demand is concrete. Full design path in
[SCALAR_MARKETS_DESIGN.md](SCALAR_MARKETS_DESIGN.md).

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
