import Lake
open Lake DSL

package janus_conditional_tokensProofs

require qedgenSupport from
  "./lean_solana"

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "v4.30.0-rc2"

@[default_target]
lean_lib Janus_conditional_tokensSpec where
  roots := #[`Spec, `Proofs, `SVM.Market]
