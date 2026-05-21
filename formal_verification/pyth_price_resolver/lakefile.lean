import Lake
open Lake DSL

package janus_pyth_price_resolverProofs

require qedgenSupport from
  "./lean_solana"

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "v4.30.0-rc2"

@[default_target]
lean_lib Janus_pyth_price_resolverSpec where
  roots := #[`Spec, `Proofs]
