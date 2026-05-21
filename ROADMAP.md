# Janus roadmap

Where Janus stands today and what's queued for future work. Sorted by
distance-to-ship: the top items are concrete next moves; the bottom
items are multi-day or research-level projects with clear entry points.

## Shipped (current state, `main`)

- 5 Pinocchio programs: conditional-tokens, lmsr-market, slot-height-
  resolver, pyth-price-resolver, market-factory.
- TypeScript SDK with high-level `createMarketWithSlotResolver()` flow.
- 14 Mollusk integration tests covering real SPL Token CPIs.
- Localnet end-to-end test against `solana-test-validator`.
- 107 Kani BMC harnesses verifying transition properties.
- 75 Lean theorems (Mathlib-enabled, all build via `lake build`),
  including a collateral conservation theorem proven across every
  conditional-tokens handler.
- 5-job GitHub Actions CI (rust, kani, qedgen, lean matrix, sdk) that
  fails on any drift.
- QEDGen spec validation on every push.

## Queued (small, well-scoped)

### Devnet deploy + canonical demo

Generate fresh program keypairs for devnet, deploy via the `solana
program deploy` flow already in `scripts/e2e-localnet/run.sh`, and ship
a tiny Next.js page that lists registered markets and trades them.
Hooks into the existing SDK; no new on-chain code. Half a day.

### Real Pyth feed integration on devnet

The pyth-price-resolver already validates the PriceUpdateV2
discriminator. The remaining integration work is:

1. Add `feed_id : [u8; 32]` and `max_staleness_slots : u64` fields to
   `PythPriceResolverState`. Update the spec + Lean proofs.
2. In `compute_outcome`, read the feed_id at offset 41 and posted_slot
   at offset 125; reject feeds that don't match `state.feed_id` or are
   stale.
3. Use a real Pyth feed (e.g. SOL/USD on devnet) in an end-to-end
   integration test.

Estimated: 1 day, including spec + Lean updates.

### `qedgen aristotle submit`

Once `ARISTOTLE_API_KEY` is set, run
`qedgen aristotle submit --project-dir formal_verification/<program>
--wait` per program. Harmonic's Aristotle is a heavyweight theorem
prover that produces independent Lean proofs we can cross-check
against ours. Diff failures = either our proof or theirs is wrong.

Setup: an account at https://aristotle.harmonic.fun. Wall-clock per
program: 15 min to several hours depending on theorem difficulty.

## Research projects (multi-day)

### Multi-user `Map[N]` state invariants

The current spec uses scalar state (`yes_supply : U64`) because the
program tracks aggregate values; individual user balances live in
external SPL Token accounts. To prove the cross-account invariant
"sum of user YES balances equals yes_supply equals vault", we'd need
to either:

(a) Model the SPL Token program in Lean alongside ours, then derive
    the multi-user invariant as a corollary of SPL Token's known
    semantics + our yes_supply/vault invariants.
(b) Introduce a new top-level Janus spec module that abstractly models
    "user accounts" as a `Map[N] UserBalance` field, with operations
    parameterised by user index, and prove `Finset.sum(accounts[i]) ==
    yes_supply` directly using Mathlib's BigOperators.

Approach (b) is faster but creates a parallel state model that doesn't
match the actual program; (a) is more honest but requires a substantial
Lean library for the SPL Token program.

Entry point: start a new `crates/spl-token-model/` Lean library that
captures `MintTo` / `Burn` / `Transfer` as state transitions over a
`Map[N] TokenAccountBalance`, then import it into a new
`token_conservation.qedspec` that composes with the existing
conditional-tokens spec. Estimated 3-5 days.

### True LMSR with fixed-point exp/ln on BPF

The current lmsr-market is CPMM with creator subsidy (commit
b6f3b74). True LMSR — Hanson's logarithmic market scoring rule —
needs:

```
C(q_yes, q_no) = b * ln(exp(q_yes / b) + exp(q_no / b))
```

To implement this on BPF you need:
1. A fixed-point arithmetic type. Q48.16 or similar — wide enough to
   absorb the `exp` blow-up without overflow.
2. `exp` and `ln` implementations. Options: CORDIC, Taylor series with
   range reduction, or a precomputed lookup table. CORDIC has the
   best CU/accuracy tradeoff but takes time to get right.
3. CU budget analysis. Each swap needs two exp + two ln + some
   multiplies/divides. Goal: < 50K CU per swap.
4. New Lean proofs over `Mathlib.Analysis.SpecialFunctions.Log` to
   capture the bounded-loss property of LMSR (max loss for the
   subsidizer = `b * ln(2)` regardless of how many trades execute).

Entry point: prototype in a separate `crates/lmsr-math/` crate with
proptest harnesses covering the fixed-point error bounds, then port
into the program once accuracy is acceptable. Estimated 5-10 days.

### Conditional-tokens scalar markets

Currently every market is binary (YES / NO). Scalar markets — bets
where the payout is a function of the outcome rather than 0/1 — would
extend the primitive substantially. Gnosis Conditional Tokens supports
this via outcome partitions; we'd need to:

1. Generalise the `yes_mint` / `no_mint` pair to a `Vec<outcome_mint>`
   of arbitrary length.
2. Add a payout vector (e.g. `[0, 25, 50, 75, 100]`) recorded at
   resolution.
3. Reshape the spec + Lean conservation theorems to sum across all
   outcome supplies.

Significant scope creep — only worth doing if there's clear demand for
scalar markets specifically (typically prediction-market-on-numbers
products: "SOL price at end of month").

## Distribution (if you ever decide to ship)

When/if Janus moves from "side quest" to "thing people use":

- Mainnet program keypairs (separate from devnet); generate fresh and
  guard authority carefully.
- Audit pass — ideally something like Code4rena or OtterSec, focused on
  the conditional-tokens program (highest blast radius).
- One curated frontend launched against a small set of markets (the
  Indian cricket / Polymarket-shape vertical from earlier
  conversation if that's the wedge you pick).
- A short launch post that leans on the verification depth: "175
  mechanized proofs, formal collateral conservation theorem, full BMC
  + Lean inductive coverage" — currently rare on Solana and a real
  differentiator.

That's distribution, not building. Stay clearly on one side at a time.
