# Scalar markets: design sketch

**Status**: deferred. Per `ROADMAP.md`: *"Scope-creep flag: only worth
doing if demand is concrete."* This doc captures the path so future
work doesn't restart from scratch.

## What changes vs. binary

Today: each market has exactly two outcomes (YES, NO) backed by two
SPL mints. Conditional-tokens splits 1 collateral into 1 YES + 1 NO;
exactly one mint pays out at 1:1 after resolution.

Scalar / N-outcome: each market has `N ∈ [2, 8]` outcomes, backed by
`N` SPL mints. Split 1 collateral into N tokens, one of each outcome.
Exactly one mint pays out at 1:1 after resolution. (For continuous
scalar markets like "BTC price at month end", you bucket into N
ranges.)

## Programs that need to change

### `conditional-tokens`
- **State** (`Market`): replace `vault: Pubkey` + 2-side mint refs
  with a fixed-cap array `outcome_mints: [Pubkey; N_MAX]`, where
  `N_MAX = 8`, plus an `n_outcomes: u8` count. The unused slots are
  zeroed.
- **Resolution outcome**: extend `ResolutionOutcome` from `{Unresolved,
  Yes, No, Invalid}` to `{Unresolved, Winning(u8), Invalid}` with the
  winning index ∈ [0, n_outcomes).
- **Split**: mints 1 of each outcome token to the user per unit of
  collateral. Loop over `n_outcomes` and emit N `mint_to` CPIs.
- **Merge**: burns 1 of each outcome token. Requires N input token
  accounts in the instruction.
- **Redeem**: takes the winning index from the resolver, burns the
  matching outcome's token from the user, transfers collateral.

### `lmsr-true-market`
- Generalises naturally: LMSR cost `C(b, q_1, ..., q_N) = b · log(Σ
  exp(q_i / b))`. Bounded-loss bound becomes `b · log(N)` instead of
  `b · log(2)`.
- Math crate (`janus-lmsr-math`) needs a `cost_n(b, &[Q32_32]) ->
  Option<Q32_32>` helper. The log-sum-exp trick generalises with one
  pass over the array.
- Pool state grows with N: `q_supply: [u64; N_MAX]`. With N_MAX=8,
  Pool grows by ~56 bytes.

### `lmsr-market` (CPMM)
- Doesn't generalise cleanly. CPMM with N>2 reserves needs a higher-
  dimensional invariant; the natural extension is a Balancer-style
  geometric-mean curve which is a different beast entirely. Easiest
  option: deprecate CPMM in the N>2 case and require true-LMSR for
  scalar.

### Resolvers
- `slot-height-resolver`: extend to return an outcome index per slot
  range. State adds `[outcome_index_for_range_i; N_MAX]`. Set-and-
  forget design.
- `pyth-price-resolver`: extend with N-1 threshold prices defining N
  buckets. Sort thresholds ascending; outcome i is selected if
  `thresholds[i-1] ≤ price < thresholds[i]` (with -∞ / +∞ at the ends).

## Wire format and SDK

- Add a "scalar" market-type byte to `MarketFactory::Register` so
  consumers can branch on outcome semantics.
- SDK gains a `createScalarMarket(n_outcomes, ...)` helper that wires
  N mints + the appropriate resolver shape.

## Proofs

- `conditional-tokens` collateral-conservation theorem still holds —
  it's an N-way sum invariant instead of a 2-way one. Update Lean
  spec from `Fin 2 → ...` to `Fin n_outcomes → ...`. Aristotle would
  close most of the generalisation.
- `lmsr-true-market` bounded-loss generalises with `Real.log N` in
  place of `Real.log 2`. The structural inequality
  `log(Σ exp(x_i)) ≥ max x_i` is straight from Mathlib's
  `Real.log_sum_exp` family.

## Out of scope (real continuous scalar)

A "true" scalar market — where the outcome is a real number, not a
bucket index — needs a continuous payout function (e.g., LP-style
"long the index"). That's a different primitive entirely:
collateral isn't pairwise-redeemable for outcome tokens; the LP /
trader split is more like Perp or option markets. Not Janus-shaped.
Stick to bucketed scalar markets.

## When to revisit

- A prospective integrator names a concrete scalar use case AND
- They have liquidity / distribution AND
- A binary market literally cannot serve their need (which is rare —
  most "scalar" questions can be decomposed into 3-5 binaries).

Until then, ship and iterate on the binary primitive.
