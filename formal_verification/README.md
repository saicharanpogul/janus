# Formal verification

QEDGen-driven specification + verification artifacts for every Janus
program. Each subdirectory pairs the authored `.qedspec` (single source
of truth) with the generated:

- `Spec.lean` — the program as a Lean 4 state machine. Operations,
  guards, effects, and properties lifted from the qedspec.
- `Proofs.lean` — user-owned proof entry point. Contains `theorem` stubs
  (closed with `sorry` or hand-written tactics) for every
  `property × handler` preservation obligation the spec declares.
- `kani.rs` — Kani bounded-model-checker harnesses covering guard
  enforcement, property preservation, and effect conformance.
- `proptest.rs` — proptest harnesses for fast counterexample search.

Subdirectories:

```
formal_verification/
├── conditional_tokens/      # asset layer: split + merge + redeem
├── lmsr_market/             # AMM: pool init, swap, withdraw
├── slot_height_resolver/    # resolves at a target slot
├── pyth_price_resolver/     # resolves via Pyth feed comparison
└── market_factory/          # on-chain registry
```

## Re-running

```bash
# Validate every spec
for s in formal_verification/*/*.qedspec; do
  qedgen check --spec "$s"
done

# Regenerate artifacts for one program
cd formal_verification/<program>
qedgen codegen --spec <program>.qedspec --lean --kani --proptest --output-dir .
```

## What's verified vs what's deferred

| Layer | Status |
|---|---|
| Spec → Lean state-machine model | ✅ Auto-generated, type-checked by qedgen `check`. |
| Spec property coverage | ✅ Every property covers every operation that touches its fields (qedgen `check --coverage` reports 100% for the resolvers, ≥ 75% for the others). |
| Lean proofs (`Proofs.lean`) | 🟨 Theorem stubs are present; the actual `sorry`-fillings are a follow-on with `qedgen aristotle` / `qedgen fill-sorry`. |
| Kani BMC harnesses | 🟨 Compile-ready harnesses sit in each `kani.rs`. Running them requires `cargo-kani` against an Anchor scaffold of the spec; the Pinocchio implementation is verified *against the model*, not directly. |
| Proptest harnesses | 🟨 Same situation — generated harnesses target the spec's model, not the Pinocchio code. Adapting them to test the Pinocchio programs directly is a manual port. |

## Important caveat

QEDGen's codegen is wired for Anchor and Quasar; the Pinocchio target is
reserved but not yet implemented. The Lean model and Kani/proptest
harnesses verify the **spec**, not the Pinocchio implementation. The
implementation is treated as a manual realization of the verified
specification — the same way Solana programs are commonly verified
today: prove the design with Lean/Kani, mirror it in production code,
and use integration tests (see `tests/`) to detect implementation drift.
