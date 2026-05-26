# Level 2 step 2 — proposal before touching qedsvm

Stop point per the standing instruction. This doc captures (a) what
the next concrete actions look like, (b) what I'd touch in qedsvm and
why, and (c) the magnitude estimate now that I have real disassembly
in front of me.

## What changed after the disasm pass

I ran `llvm-objdump --triple=sbpf -d` on `target/deploy/janus_conditional_tokens.so`
and snapshotted it at `tests-qedsvm/disasm/janus_conditional_tokens.S`
(3193 lines, ~30 KB). Two findings I didn't expect when drafting
[`LEVEL_2_SPLIT_SPEC.md`](LEVEL_2_SPLIT_SPEC.md):

1. **The release build inlined every handler into `entrypoint`.**
   Pinocchio + `lto = "fat"` (set in our workspace `Cargo.toml`)
   collapses `process_initialize_market`, `process_split`,
   `process_merge`, `process_redeem`, `process_resolve` all into one
   273-line `<entrypoint>` block. There is **no standalone
   `process_split` symbol** to lift in isolation. The Split path is
   a branch *inside* the entrypoint dispatch on the instruction-tag
   byte.

2. **`entrypoint` contains 15 `call` instructions.** Most are
   bookkeeping (memcpy, panic infra). The Split path traverses some
   subset of them — the actual SPL Token CPIs (Transfer, MintTo,
   MintTo) are 3 of those 15.

## What this means for the proof

The naive plan from the spec doc ("lift `process_split` bytes,
discharge a triple over those bytes") doesn't apply unchanged.
Two ways forward:

### Option A — prove the entrypoint dispatch (broader scope)

Triple becomes "for any 9-account, tag=1, amount-suffixed instruction
data, executing `entrypoint` from initial register state results in
the Split-spec post-state (or a documented failure)."

- Scope: the full 273-line entrypoint.
- Why broader: we prove not just Split's logic but also the dispatcher
  that selects it. Bonus correctness coverage.
- Cost: ~3-5× the proof effort of the original plan. The dispatcher
  branches multiple times before reaching Split's first CPI.

### Option B — rebuild with split debug binaries (narrower scope)

Add a debug build profile that disables LTO and emits per-handler
symbols, then prove against that binary. Tradeoff: that binary is
**not what we deploy**, so the proof's bytecode-conformance claim
weakens. We'd have to argue "the optimised binary's branch behaviour
matches the unoptimised binary's standalone-handler behaviour" —
which is exactly what qedsvm conformance is supposed to give us, but
only at the IO level (return data, account writes), not at the
internal-control-flow level.

### Recommendation

**Option A.** The whole point of bytecode-layer proof is to verify
what mainnet runs. Going via a separate debug binary undermines the
guarantee. The 3-5× effort is real but bounded.

## What I'd touch in qedsvm for step 2

Listed in order of intrusiveness, smallest first.

### 1. Read-only: invoke `disasm-to-lean` on the entrypoint

Their CLI tool. Pipe `entrypoint`-range disasm through it, get the
`List Insn` Lean encoding. Output lands in our repo. **Zero qedsvm
modifications.**

### 2. lakefile dependency

Add to `formal_verification/conditional_tokens/lakefile.lean`:

```lean
require qedsvm from git
  "https://github.com/QEDGen/qedsvm.git" @ "main"
```

Risks:
- Mathlib version unification. They pin `v4.30.0-rc2`; we pin
  `v4.30.0-rc2`. Should be fine but `lake update` will rebuild
  everything — ~30 min the first time, cached after.
- Their lake setup expects the `lake-bridge` Rust staticlib to be
  buildable. That's a cargo dep in our Lean project's build path,
  which is unusual but tractable.

**No commit to qedsvm.** Pure local config change.

### 3. Predicate gaps to fill upstream

The Split triple needs the following SL predicates. Status from their
`SVM/Solana/` library + `ROADMAP.md`:

| Predicate | Shape | Status | Action |
|---|---|---|---|
| `tokenAcctBalance(acct, amt)` | account holds token-account with `amount = amt` | ✅ shipped | use as-is |
| `tokenMintSupply(mint, sup)` | mint has supply = sup | ⚠️ partial (no `supply` field yet) | upstream PR if Phase E hasn't shipped it by then |
| `marketState(addr, m)` | our 248-byte Market struct | ❌ ours | already started in `SVM/Market.lean`; nothing to upstream |
| `mintToTriple` | Token::MintTo Hoare lemma | ❌ not shipped per ROADMAP | upstream PR — this is the bigger contribution |
| `transferTriple` | Token::Transfer Hoare lemma | ⚠️ partial — see PToken `BalanceSpec.lean` | use their lifting lemma; may need a wrapper |
| `pdaSigner(seeds, prog, pda)` | PDA signer-promotion | ✅ shipped | use as-is |

**The MintTo predicate + Hoare lemma is the real upstream contribution.**
It's ~400 lines of separation-logic-over-bytecode (estimating from
their `examples/lean/PToken/RefinesTransfer.lean`, the closest analog).

Strategy: open a draft PR (their `feature/spl-token-mintto-predicate`
branch on our fork), they review. Same workflow as PR #8.

## Concrete next-action menu

Tell me which to do:

**(a)** Run `disasm-to-lean` on the entrypoint range, save the
`List Insn` Lean file in our repo. Pure invocation of their tool, no
qedsvm changes. ~10 minutes.

**(b)** Add the lakefile dependency on qedsvm and verify the combined
`lake build` works. Local config change, no commit to qedsvm. ~30
minutes first time (Mathlib rebuild), ~5 min after.

**(c)** Open the upstream MintTo-predicate PR on QEDGen/qedsvm. This
is the big one — 1-2 weeks of Lean work, including the proof against
their decompiled Token program. I'd send a one-line "we're planning
to upstream MintTo, OK?" comment on their roadmap issue first so they
know it's coming.

**(d)** Stop Level 2 here for now. We have:
- 6 byte+CU-identical differential fixtures (Level 1 done)
- `SVM/Market.lean` ready (step 1)
- Full disasm of conditional-tokens at hand
- This proposal doc

That's a respectable defensive line. Could pause and pivot to a
product/distribution item.

## My honest read

Level 2 to completion is **2-4 months of focused Lean work** at the
scale I'm seeing now. The conditional-tokens entrypoint alone has 15
calls and 270+ instructions; the proof would have to step through
every path leading to the SPL Token CPIs.

We'd get a meaningfully stronger guarantee than the byte+CU
conformance we already have — but at a cost most teams wouldn't pay
unless there's a real audit trigger (a CVE, a hack, an audit firm
requiring it for sign-off).

If you want to commit to that arc, **(c)** is the right opening move
and it puts us on the qedsvm collaboration roadmap. If you'd rather
hold Level 1 as the verification stake and ship product instead,
**(d)** is the sober call.

Either way, I won't touch qedsvm without your nod.
