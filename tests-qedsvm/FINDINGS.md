# qedsvm integration: findings to share with QEDGen

What follows is a list of issues encountered while wiring qedsvm
(commit at `https://github.com/QEDGen/qedsvm` main, 2026-05-25) into
the Janus differential test harness. Reported in the order encountered.

Janus is a Pinocchio-based binary-markets stack with 6 deployed
programs on devnet. Goal of the integration: run every Mollusk
fixture through qedsvm too and assert byte+CU equality.

Result: **0 of our 6 programs round-trip through qedsvm**, blocked at
issue #5 below. Issues #1–#4 are friction; #5 is the showstopper.

---

## #1. Two account-crate versions

`qedsvm::Svm::process_instruction` takes
`&[(Pubkey, AccountSharedData)]` from `solana-account 4.x`.

`mollusk_svm::Mollusk::process_instruction` (0.12.1-agave-4.0,
which qedsvm's own examples use) takes
`&[(Pubkey, mollusk_account::Account)]` where `mollusk_account` is
aliased to `solana-account 3.4.0`.

Differential test code must build the same fixture in both shapes.
We wrote a `mollusk_to_qedsvm(&[(Pubkey, mollusk_account::Account)])
→ Vec<(Pubkey, AccountSharedData)>` converter — straightforward but
boilerplate every consumer will rewrite.

**Suggestion**: ship the converter from qedsvm under the
`diff-mollusk` feature, e.g. `qedsvm::diff::mollusk_to_qedsvm(...)`.

---

## #2. `cargo:rustc-link-arg` doesn't propagate to downstream crates

This is a **critical** consumability issue.

`qedsvm-rs/build.rs` emits ~80 `cargo:rustc-link-arg=<dylib_path>`
directives (one per `qedsvm_*.dylib`) plus `-Wl,-u,<sym>` directives
for forcing FFI symbol pulls. These run correctly when qedsvm-rs is
built.

But cargo's rule for `rustc-link-arg` is: **it only applies to bins,
tests, examples, etc. **within the same package** as the build.rs.**
Downstream packages that depend on qedsvm as a library don't inherit
these flags.

Concrete symptom from our crate (path-dep on qedsvm):

```
Undefined symbols for architecture arm64:
  "_initialize_qedsvm_SVM_Ffi"
  "_qedsvm_precompile_dispatch"
  "_qedsvm_run_with_registry_and_pid"
ld: symbol(s) not found
```

Verified by inspection: qedsvm's build.rs emits the link-args
correctly (`cargo build -vv` shows them), but they're absent from
our test crate's `cc` invocation.

**Our workaround**: replicate the entire link-arg setup in our own
`build.rs` (see `tests-qedsvm/build.rs`). 95 lines mirroring qedsvm's
build.rs.

**Suggestion**: ship a tiny `qedsvm-buildscript` crate that exposes a
`emit_link_args(qedsvm_root: &Path)` function downstream `build.rs`
files can call. That makes integration a one-line copy, and any
future changes to the link strategy land in one place.

---

## #3. `solana_account::Account` field-by-field copy
   needs `WritableAccount` trait

When converting `mollusk_account::Account` (3.x) to
`AccountSharedData` (4.x), I needed `set_executable`, `set_rent_epoch`,
`set_data_from_slice`. These are methods on the `WritableAccount`
trait, not direct fields/methods on `AccountSharedData`. Cargo's
error suggests the import. **Minor** — just an FYI for downstream
docs.

---

## #4. Two `solana-account` versions resolve in the same crate

Mollusk 0.12.1-agave-4.0 — which qedsvm itself uses for diff-testing
— internally calls `process_instruction(&[(Pubkey,
solana_account::Account)])` with **solana-account 3.x**, not 4.x.
Yet mollusk's own crate name is "agave-4.0".

This forces a downstream consumer to pull `solana-account` at TWO
versions:
- `solana-account = "4.3.0"` for qedsvm's input type
- `mollusk-account = { package = "solana-account", version = "3.4.0" }`
  for mollusk's

We're mirroring the trick qedsvm-rs uses in its own dev-deps. Real
fix is upstream of qedsvm (mollusk should unify on agave 4.x or
qedsvm should accept 3.x), but worth documenting.

---

## #5. **Showstopper**: every Janus program fails `BufferParse(InvalidDupIndex(0))`

This blocks our entire integration. Repro is two test files in
`tests-qedsvm/tests/`:

### Test A — slot-resolver Initialize (CPI to System Program)

```rust
// Instruction: 4 accounts (payer signer/writable, state pda, authority
// signer, system program). Pubkeys all unique. Pre-state has uninitialized
// state PDA (Account::default()).
let result = svm.process_instruction(&ix, &qedsvm_accounts);
// → Err(BufferParse(InvalidDupIndex(0)))
```

Mollusk output for the same fixture:
```
Program 3y75gGqFK1KhNF5k1sMy6ydnw6WLcbn1SPRoYbyRkjMj invoke [1]
Program 11111111111111111111111111111111 invoke [2]
Program 11111111111111111111111111111111 success
Program 3y75gGqFK1KhNF5k1sMy6ydnw6WLcbn1SPRoYbyRkjMj consumed 3265 of 1400000 compute units
Program 3y75gGqFK1KhNF5k1sMy6ydnw6WLcbn1SPRoYbyRkjMj success
```

### Test B — slot-resolver Resolve (read-only, NO CPI, single account)

Stripped down further: just one account, program-owned, pre-populated
with serialized state, instruction tag = Resolve (0). No system
program. No CPI. Should be the simplest possible path.

```rust
let ix = Instruction {
    program_id: <slot-resolver pubkey>,
    accounts: vec![AccountMeta::new_readonly(state, false)],
    data: vec![0u8], // Resolve tag
};
let accounts = vec![(state, AccountSharedData { /* pre-initialized */ })];
let result = svm.process_instruction(&ix, &accounts);
// → Err(BufferParse(InvalidDupIndex(0)))
```

Mollusk on the same fixture: success, 478 CU, return data `AA==`
(0x00 = Unresolved).

### Root-cause hypothesis

Looking at `qedsvm-rs/src/deserialize.rs:55-66`:

```rust
for i in 0..num_accounts {
    let dup_info = r.read_u8()?;
    if dup_info == NON_DUP_MARKER {
        // fresh record
    } else {
        let src = dup_info as usize;
        if src >= num_accounts || by_first_occurrence[src].is_none() {
            return Err(DeserializeError::InvalidDupIndex(src));
        }
    }
}
```

At i=0, qedsvm reads `dup_info = 0` (i.e., a dup marker pointing to
itself). NON_DUP_MARKER is 0xFF; so for the first occurrence of any
account, the marker byte should be 0xFF, not 0.

Either:
- qedsvm's pre-execution serializer is writing 0 instead of 0xFF for
  the first account's marker byte, OR
- The program's modified buffer overwrites that byte during
  execution (but why would it?), OR
- There's some Pinocchio-specific entrypoint shape qedsvm's
  serializer doesn't account for.

**Note**: their own p-token Transfer fixture (also Pinocchio-based)
runs fine. So it's not a generic Pinocchio incompatibility. Something
about our specific programs trips it.

### What I can confirm

- All 6 of our `.so` files run in Mollusk's interpreter without
  issue (we have 14 passing integration tests on Mollusk 0.12.0).
- Both Initialize (with CPI) AND Resolve (no CPI) fail in qedsvm
  with identical errors. So it's NOT a System Program CPI bug.
- Our binaries are compiled with `cargo-build-sbf` (Solana 2.2.20)
  against Pinocchio 0.8, no nightly.
- The single-account Resolve case is the smallest repro.

### Asks for the qedsvm dev

1. Is there a serialization-format assumption qedsvm makes that
   wouldn't hold for `cargo-build-sbf` 2.2.20 + Pinocchio 0.8?
2. Would a hex dump of `raw.modified_input` help diagnose? (I can
   patch qedsvm-rs to print the buffer if useful — the harness is
   ready to capture it.)
3. The successful p-token Transfer fixture in conformance_demo
   suggests Pinocchio works in general. Is there a Pinocchio
   build-flag or entrypoint shape that the test fixture happens
   to hit and ours doesn't?

The repro is reproducible from this commit. The harness is in
`tests-qedsvm/`:

```bash
cd tests-qedsvm
cargo test --test slot_resolver_resolve -- --nocapture
```

It will fail with the error above. The .so under test is
`target/deploy/janus_slot_height_resolver.so` (built from
`programs/slot-height-resolver/` in this repo).
