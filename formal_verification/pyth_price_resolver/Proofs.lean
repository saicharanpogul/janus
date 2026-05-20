/-
Proofs.lean — user-owned preservation proofs.

`qedgen codegen` bootstraps this file once and never touches it again.
Spec.lean is regenerated; this file is durable. `qedgen check`
(and `qedgen reconcile`) flag orphan theorems (handler removed from
spec) and missing obligations (new `preserved_by` declared).
-/
import Spec

namespace PythPriceResolver

open QEDGen.Solana

-- Preservation obligations the spec expects.
-- Write each theorem against the signature generated in Spec.lean
-- (the handler's transition + the property predicate). Close with
-- tactics like `unfold`, `omega`, or `simp_all` as appropriate, or
-- `QEDGen.Solana.IndexedState.forall_update_pres` for per-account
-- invariants in Map-backed specs.
--
--   theorem comparison_bounded_preserved_by_initialize
--   theorem comparison_bounded_preserved_by_resolve_case_0
--   theorem comparison_bounded_preserved_by_resolve_case_1
--   theorem comparison_bounded_preserved_by_resolve_case_2
--   theorem comparison_bounded_preserved_by_resolve_case_3
--   theorem comparison_bounded_preserved_by_resolve_otherwise
--   theorem outcome_in_canonical_range_preserved_by_initialize
--   theorem outcome_in_canonical_range_preserved_by_resolve_case_0
--   theorem outcome_in_canonical_range_preserved_by_resolve_case_1
--   theorem outcome_in_canonical_range_preserved_by_resolve_case_2
--   theorem outcome_in_canonical_range_preserved_by_resolve_case_3
--   theorem outcome_in_canonical_range_preserved_by_resolve_otherwise

end PythPriceResolver
