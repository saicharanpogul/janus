import Lake
open Lake DSL

package janus_account_layerProofs

require qedgenSupport from
  "./lean_solana"

require qedgenSupportMathlib from
  "./lean_solana_mathlib"

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "v4.30.0-rc2"

@[default_target]
lean_lib Janus_account_layerSpec where
  roots := #[`Spec, `Proofs]
