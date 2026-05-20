/-
Proofs.lean — user-owned preservation proofs.

`qedgen codegen` bootstraps this file once and never touches it again.
Spec.lean is regenerated; this file is durable. `qedgen check`
(and `qedgen reconcile`) flag orphan theorems (handler removed from
spec) and missing obligations (new `preserved_by` declared).
-/
import Spec

namespace LmsrMarket

open QEDGen.Solana

-- Preservation obligations the spec expects.
-- Write each theorem against the signature generated in Spec.lean
-- (the handler's transition + the property predicate). Close with
-- tactics like `unfold`, `omega`, or `simp_all` as appropriate, or
-- `QEDGen.Solana.IndexedState.forall_update_pres` for per-account
-- invariants in Map-backed specs.
--
--   theorem fee_bps_bounded_preserved_by_initialize_pool
--   theorem fee_bps_bounded_preserved_by_mark_resolved_no
--   theorem fee_bps_bounded_preserved_by_mark_resolved_yes
--   theorem fee_bps_bounded_preserved_by_swap_no_for_yes
--   theorem fee_bps_bounded_preserved_by_swap_yes_for_no
--   theorem fee_bps_bounded_preserved_by_withdraw_no_after_resolution
--   theorem fee_bps_bounded_preserved_by_withdraw_yes_after_resolution
--   theorem status_monotone_preserved_by_initialize_pool
--   theorem status_monotone_preserved_by_mark_resolved_no
--   theorem status_monotone_preserved_by_mark_resolved_yes
--   theorem status_monotone_preserved_by_swap_no_for_yes
--   theorem status_monotone_preserved_by_swap_yes_for_no
--   theorem status_monotone_preserved_by_withdraw_no_after_resolution
--   theorem status_monotone_preserved_by_withdraw_yes_after_resolution
--   theorem swap_no_drain_preserved_by_initialize_pool
--   theorem swap_no_drain_preserved_by_mark_resolved_no
--   theorem swap_no_drain_preserved_by_mark_resolved_yes
--   theorem swap_no_drain_preserved_by_swap_no_for_yes
--   theorem swap_no_drain_preserved_by_swap_yes_for_no
--   theorem swap_no_drain_preserved_by_withdraw_no_after_resolution
--   theorem swap_no_drain_preserved_by_withdraw_yes_after_resolution

end LmsrMarket
