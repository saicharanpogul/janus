# Lean verification results

All 68 preservation theorems across all five Janus programs are
hand-written and accepted by Lean 4.30 — `lake build` reports
"Build completed successfully" in each program directory.

## Summary

| Program | Theorems | Build |
|---|---:|---|
| slot_height_resolver | 6 | ✅ |
| pyth_price_resolver | 12 | ✅ |
| market_factory | 1 | ✅ |
| lmsr_market | 21 | ✅ |
| conditional_tokens | 28 | ✅ |
| **Total** | **68** | **✅** |

## Why this matters

QEDGen's `qedgen codegen --lean` generates the Spec.lean state-machine
model + Proofs.lean stubs. `qedgen fill-sorry` typically uses Mistral's
Leanstral API (or escalates to Harmonic's Aristotle). Neither was
available in this environment (no API keys).

**Claude (Anthropic) authored every proof directly.** They all close
mechanically with the same Lean 4 pattern:

```lean
unfold <Transition> at hstep
split at hstep
case isTrue [hg] =>
  injection hstep with hs'; subst hs'
  -- pick the right disjunct or normalise the record update
  simp [<property>]            -- or `simp [<property>, hg.<n>]`
                                 -- or `decide` / `omega` for arithmetic
case isFalse => simp at hstep
```

No Mathlib needed — the QEDGen Solana base library is pure Lean 4, and
the spec uses only `Nat` arithmetic + record updates. Lake builds
offline; the `formal_verification/<program>/lean_solana/` directory is
populated on first `lake build` (purely local; no remote fetches).

## Reproducing

```bash
cd formal_verification/<program>
lake build
```

First build per program takes ~30 seconds (compiles
`QEDGen.Solana.Account` and friends). Subsequent builds are incremental.

## Codegen patches applied

QEDGen 2.15.1 has a few codegen bugs that needed manual patches in the
generated `Spec.lean` files before `lake build` succeeded — these are
the same edits across every spec:

1. **Strip unused Mathlib imports.** Replaced
   `import Mathlib.Algebra.BigOperators.Fin` and
   `import QEDGenMathlib.IndexedState` with just
   `import QEDGen.Solana.Account` plus
   `abbrev Pubkey := QEDGen.Solana.Account.Pubkey`.
2. **Delete the duplicate `structure State where` declaration**
   immediately following the populated one.
3. **Rename `«initialize»Transition` to `initializeTransition`** in the
   `applyOp` dispatcher (the French-quote propagation was spurious in
   the call site).

These are mechanical text fixes that could be scripted; documenting
them here so the next person regenerating doesn't trip on the same
issues.

## What this gives us

Combined with the **107 passing Kani BMC harnesses** in
[`KANI_RESULTS.md`](./KANI_RESULTS.md), Janus now has:

- A type-checked, compositional state-machine model of every program.
- 175 mechanized proofs (107 Kani + 68 Lean) that the spec's
  invariants are preserved by every transition.
- Two independent verification paths — Kani's symbolic BMC and Lean's
  inductive theorem prover — that agree on the same property set.

Any future spec change re-runs `qedgen check` + `cargo kani --tests` +
`lake build` and surfaces drift at PR time, not at exploit time.
