import Lake
open Lake DSL

package janus_slot_height_resolverProofs

require qedgenSupport from
  "./lean_solana"

-- Pinned to the tag that matches `lean-toolchain` (v4.30.0-rc2).
-- `qedgen setup --mathlib` populates the shared workspace cache so
-- `lake update` resolves the Mathlib package locally instead of
-- pulling from the network.
require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "v4.30.0-rc2"

@[default_target]
lean_lib Janus_slot_height_resolverSpec where
  roots := #[`Spec, `Proofs]
