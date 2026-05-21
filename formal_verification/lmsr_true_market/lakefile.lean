import Lake
open Lake DSL

package janus_lmsr_true_marketProofs

require qedgenSupport from
  "./lean_solana"

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "v4.30.0-rc2"

@[default_target]
lean_lib Janus_lmsr_true_marketSpec where
  roots := #[`Spec, `Proofs, `LmsrCost]
