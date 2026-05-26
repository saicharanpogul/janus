# Level 2 — Bytecode Hoare triple for `conditional-tokens::process_split`

Paper draft. The byte+CU equivalence in Level 1 (see
`tests/conditional_tokens_split.rs`) means that an SL spec proven
against the qedsvm-decoded bytecode constrains the same behavior that
mainnet exhibits. This document captures what we want to prove,
phrased as a pre/post separation-logic triple. Code lands later.

---

## What `Split` does on chain

Source: `programs/conditional-tokens/src/processor.rs::process_split`.

1. Decode 8-byte `amount` from instruction data; reject `amount == 0`.
2. Destructure the 9 accounts; reject if `user` is not a signer.
3. Read `Market` from `market_ai`. Reject if:
   - `market.status != Active`
   - `market.vault != vault_ai.key()`
   - `market.yes_mint != yes_mint_ai.key()`
   - `market.no_mint != no_mint_ai.key()`
4. CPI `Token::Transfer`: `user_collateral_ai → vault_ai`, amount = `amount`, signer = `user`.
5. CPI `Token::MintTo`: `yes_mint_ai → user_yes_ai`, amount = `amount`, signer = `market_ai` (PDA, seeds derived from `Market`).
6. CPI `Token::MintTo`: `no_mint_ai → user_no_ai`, amount = `amount`, signer = `market_ai`.

No other state writes. Either the whole sequence succeeds atomically or none of it takes effect (Solana's instruction-level transaction model).

---

## Separation-logic predicates we need

Some already exist in qedsvm's `SVM/Solana/` library (see [their roadmap](https://github.com/QEDGen/qedsvm/blob/main/ROADMAP.md)); others we'd contribute upstream.

| Predicate | Shape | Status |
|---|---|---|
| `tokenAcctBalance(acct, amount)` | account holds an SPL token account with `amount = amount` | ✅ shipped — `SVM/Solana/TokenAccount.lean` |
| `tokenMintSupply(mint, supply)` | account holds an SPL mint with `supply = supply` | ⚠️ partial — needs supply-tracking field; queued in their Phase E |
| `marketState(acct, fields...)` | account holds the conditional-tokens Market struct with named fields | ❌ ours; would extend their predicate library with `programs/conditional-tokens/spec` |
| `pdaSigner(seeds, prog, pda)` | `pda = find_program_address(seeds, prog)` | ✅ shipped — `SVM/Solana/Pda.lean` |
| `signer(acct)` | account is marked as a signer in the instruction | ✅ shipped — `SVM/Solana/AccountInfo.lean` |
| `tokenProgram(acct)` | account is the SPL Token program executable | ✅ shipped (Cpi.lean's program-account-asserts) |

What we'd write for our specific case:

```
marketState(market, {
  bump        = b,
  status      = Active,
  collateral_mint = m_coll,
  yes_mint    = m_yes,
  no_mint     = m_no,
  vault       = v,
  resolver_state = rs,
  deadline_slot = ds,
  authority   = auth,
})
```

This is a fixed 8-byte-aligned 248-byte layout; lift each field as a separate atom and prove the conjunction.

---

## The triple

Let `Split(amount)` denote the entrypoint dispatch for a Split call
with the given amount. Let pre/post-state be (separation-logic) heaps
over the account address space, and let `mintAuthority(mint) = market`
be a side condition we discharge from the `marketState` predicate
(MintTo requires market_ai to be the mint authority for both mints).

```
{
  // ---- Preconditions ----
  amount > 0
  ∧ signer(user)
  ∧ tokenAcctBalance(user_collateral, U)   // any starting balance U
  ∧ tokenAcctBalance(vault, V)             // any starting balance V
  ∧ tokenAcctBalance(user_yes, Y)
  ∧ tokenAcctBalance(user_no, N)
  ∧ tokenMintSupply(yes_mint, S_yes)
  ∧ tokenMintSupply(no_mint, S_no)
  ∧ marketState(market, {
      status = Active,
      vault = v_pubkey,         // matches vault account passed in
      yes_mint = ym_pubkey,
      no_mint = nm_pubkey,
      ..fields
    })
  ∧ amount ≤ U                  // SPL Transfer would otherwise fail
  ∧ U + V ≤ u64::MAX            // overflow safety
  ∧ Y + amount ≤ u64::MAX
  ∧ N + amount ≤ u64::MAX
  ∧ S_yes + amount ≤ u64::MAX
  ∧ S_no + amount ≤ u64::MAX
}

  Split(amount)

{
  // ---- Postconditions ----
  tokenAcctBalance(user_collateral, U - amount)
  ∧ tokenAcctBalance(vault, V + amount)
  ∧ tokenAcctBalance(user_yes, Y + amount)
  ∧ tokenAcctBalance(user_no, N + amount)
  ∧ tokenMintSupply(yes_mint, S_yes + amount)
  ∧ tokenMintSupply(no_mint, S_no + amount)
  ∧ marketState(market, ..fields)         // unchanged
  ∧ exitCode = 0
}
∨
{
  // ---- Failure branch (negation of any precondition) ----
  // State unchanged from pre. exitCode ≠ 0.
}
```

### Cu bound

Empirically measured at 4840 CU (the diff test in
`tests/conditional_tokens_split.rs`). The triple should carry
`cuTripleWithin(maxCU = 5000, ...)` so a CU regression in the bytecode
also breaks the proof. The precise number can be tightened via
`native_decide` once the proof closes — that's the pattern in
qedsvm's own `examples/lean/ByteIncrement.lean`.

---

## Collateral-conservation invariant (the headline)

The post-state implies, by definition:

```
(V_post + balance_change) = (V_pre + amount)
where balance_change = U_post - U_pre = -amount
⟹ V_post - V_pre = amount = U_pre - U_post
```

i.e., **every `amount` that leaves `user_collateral` lands in `vault`,
and an exactly matching amount of YES + NO appears in the user's
holdings**. This is the bytecode-layer twin of the state-machine
theorem already proved in
`formal_verification/conditional_tokens/Proofs.lean::collateralConservation`.

What this proof adds, that the state-machine theorem doesn't:
- The high-level spec proves the *abstract handler* respects the
  invariant. Our triple proves the *compiled binary* respects the
  invariant.
- The high-level proof doesn't see CPI semantics. Our triple sees
  the CPI through qedsvm's `sol_invoke_signed_c` handler — meaning a
  bug like "we accidentally passed a different signer-seeds tuple"
  would surface here even if the abstract spec missed it.
- The binary-layer proof carries a verified CU bound, which the
  abstract spec doesn't.

---

## Order of work

1. **Write `marketState` predicate** in a new
   `formal_verification/conditional_tokens/SVM/Market.lean` module
   alongside qedsvm's library. (~1 day; we own the struct layout.)
2. **Open a draft proof file**
   `formal_verification/conditional_tokens/SplitTriple.lean` that
   imports qedsvm and our `Market` predicate, declares the triple,
   and uses `sl_block_iter` to step through the decompiled bytecode
   (we can get the decompiled `List Insn` from
   `qedsvm-rs/src/bin/disasm_to_lean.rs`).
3. **Discharge the CPI sub-proofs**:
   - `Token::Transfer` — Phase E partial; we'd lean on their
     `tokenAcctBalance`-conservation lemma for `Token::Transfer`. May
     need to contribute the *exact* lemma we want if they only have
     a flat byte-level version.
   - `Token::MintTo` — likely needs new contribution (Phase E does
     not yet ship MintTo predicates per their ROADMAP).
4. **Close the top-level triple** using their composition tactics
   (`sl_block_iter`, `sl_branch`, `sl_rw_abs`).
5. **Add a `Runner.runElf` witness** like their `ByteIncrement.lean`
   demo so `lake build SplitTriple` is the user-facing claim.

Estimated effort: **2-4 weeks** of work, depending on how much of the
CPI-side predicate library we need to contribute upstream. If qedsvm
Phase E lands the MintTo predicate first, the inner work shrinks
considerably.

---

## Touchpoints with qedsvm (FLAG: requires their cooperation)

We can't avoid this. Level 2 is **inherently collaborative**:

- Step 3 may require new SL predicates upstream. We'd open a draft PR
  with the predicate and proof skeleton, get review before merging.
- We need their `disasm_to_lean` tool to lift our `.so` bytecode into
  the `List Insn` form their tactics consume.
- The `lake update` in our `formal_verification/` project will need
  to depend on qedsvm via its lake manifest — that's the first place
  to confirm the version of their library compiles with our existing
  Mathlib pin.

Per the user's standing instruction: **before any commit to
QEDGen/qedsvm, I'll surface a one-pager of what we want to change
and why.**

---

## What's NOT in scope here

- **lmsr-market Swap triple** — Same shape (PDA-signed Token::Transfer)
  but smaller invariant ("k grows by ≥ fee"). Mechanically cheaper
  once Split's done; defer until then.
- **Multi-instruction sequences** — Solana isn't a stack machine and
  qedsvm doesn't reason about cross-transaction state. The triple is
  per-instruction.
- **The bounded-loss theorem for true-LMSR** — that's `Real.log`
  reasoning, lives in the math layer, not the bytecode layer. Already
  proven against the abstract spec.
