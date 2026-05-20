# Lean verification results

All 68 preservation theorems across all five Janus programs are
hand-written and accepted by Lean 4.30 with Mathlib enabled — `lake
build` reports "Build completed successfully" in each program
directory.

## Summary

| Program | Theorems | Build time (proofs only) |
|---|---:|---:|
| slot_height_resolver | 6 | 4.9s |
| pyth_price_resolver | 12 | 6.4s |
| market_factory | 1 | 2.2s |
| lmsr_market | 21 | 2.2s |
| conditional_tokens | 28 | 2.9s |
| **Total** | **68** | — |

Each project pulls Mathlib v4.30.0-rc2 (pinned to match
`lean-toolchain`) and the QEDGen Solana support library. First `lake
update` decompresses ~8300 cached files in ~15 seconds; subsequent
incremental builds compile just `Spec.lean` + `Proofs.lean`.

## Why this matters

QEDGen's `qedgen codegen --lean` generates the Spec.lean state-machine
model + Proofs.lean stubs. `qedgen fill-sorry` typically uses Mistral's
Leanstral API (or escalates to Harmonic's Aristotle). Neither was
available in this environment (no API keys).

**Claude (Anthropic) authored every proof directly.** They all close
mechanically with the same Lean 4 + Mathlib pattern:

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

## Mathlib setup

Each program's `lakefile.lean` pulls Mathlib at the exact tag matching
the Lean toolchain:

```lean
require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "v4.30.0-rc2"
```

To populate the local Lean cache the first time, run:

```bash
qedgen setup --mathlib
```

This populates `~/.qedgen/workspace/.lake/packages/mathlib/` with the
pre-built Mathlib cache (multi-GB on first run, instant on subsequent
runs of `lake update`).

After setup, each program's `lake update` decompresses the cached
Mathlib in ~15 seconds — no network refetch.

## Reproducing

```bash
# One-time per machine (populates the shared Mathlib cache):
qedgen setup --mathlib

# Per program:
cd formal_verification/<program>
lake update      # ~15s — decompresses cached Mathlib
lake build       # ~2-7s — incremental compile of Spec.lean + Proofs.lean
```

## Codegen patches applied

QEDGen 2.15.1 has a few codegen bugs that needed manual patches in the
generated `Spec.lean` files before `lake build` succeeded:

1. **`«initialize»Transition` → `initializeTransition`** in the
   `applyOp` dispatcher. The French-quote propagation was spurious in
   the call site.
2. **Delete the duplicate `structure State where` declaration**
   immediately following the populated one.

Trivial mechanical fixes that we documented in the commit history for
the next person regenerating from a clean checkout.

## What this gives us

Combined with the **107 passing Kani BMC harnesses** in
[`KANI_RESULTS.md`](./KANI_RESULTS.md), Janus now has:

- A type-checked, compositional state-machine model of every program.
- 175 mechanized proofs (107 Kani + 68 Lean) that the spec's
  invariants are preserved by every transition.
- Two independent verification paths — Kani's symbolic BMC and Lean's
  inductive theorem prover — that agree on the same property set.
- Mathlib available for future invariants requiring `BigOperators`,
  `IndexedState`, or symbolic algebra (e.g. when v2 ships fixed-point
  exp/ln for true LMSR).

Any future spec change re-runs `qedgen check` + `cargo kani --tests` +
`lake build` and surfaces drift at PR time, not at exploit time.
