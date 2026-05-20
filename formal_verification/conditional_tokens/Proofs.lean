/-
Proofs.lean — user-owned preservation proofs.

`qedgen codegen` bootstraps this file once and never touches it again.
Spec.lean is regenerated; this file is durable. `qedgen check`
(and `qedgen reconcile`) flag orphan theorems (handler removed from
spec) and missing obligations (new `preserved_by` declared).
-/
import Spec

namespace ConditionalTokens

open QEDGen.Solana

-- Preservation obligations the spec expects.
-- Write each theorem against the signature generated in Spec.lean
-- (the handler's transition + the property predicate). Close with
-- tactics like `unfold`, `omega`, or `simp_all` as appropriate, or
-- `QEDGen.Solana.IndexedState.forall_update_pres` for per-account
-- invariants in Map-backed specs.
--
--   theorem status_monotone_preserved_by_initialize_market
--   theorem status_monotone_preserved_by_merge
--   theorem status_monotone_preserved_by_redeem_no
--   theorem status_monotone_preserved_by_redeem_yes
--   theorem status_monotone_preserved_by_resolve_no
--   theorem status_monotone_preserved_by_resolve_yes
--   theorem status_monotone_preserved_by_split
--   theorem vault_covers_supply_preserved_by_initialize_market
--   theorem vault_covers_supply_preserved_by_merge
--   theorem vault_covers_supply_preserved_by_redeem_no
--   theorem vault_covers_supply_preserved_by_redeem_yes
--   theorem vault_covers_supply_preserved_by_resolve_no
--   theorem vault_covers_supply_preserved_by_resolve_yes
--   theorem vault_covers_supply_preserved_by_split
--   theorem yes_no_paired_while_active_preserved_by_initialize_market
--   theorem yes_no_paired_while_active_preserved_by_merge
--   theorem yes_no_paired_while_active_preserved_by_redeem_no
--   theorem yes_no_paired_while_active_preserved_by_redeem_yes
--   theorem yes_no_paired_while_active_preserved_by_resolve_no
--   theorem yes_no_paired_while_active_preserved_by_resolve_yes
--   theorem yes_no_paired_while_active_preserved_by_split

end ConditionalTokens
