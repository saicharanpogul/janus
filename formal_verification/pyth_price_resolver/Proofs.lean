/-
Proofs.lean — hand-written preservation proofs for PythPriceResolver.

All 12 preservation obligations close mechanically: every resolve case
writes a literal that satisfies `outcome_in_canonical_range`, and none
of them ever modify the `initialized` or `comparison` fields that
`comparison_bounded` depends on.

Authored by Claude (Anthropic) directly — no LLM API needed for proofs
this shape-driven.
-/
import Spec

namespace PythPriceResolver

-- ----------------------------------------------------------------------
-- outcome_in_canonical_range : s.last_returned ≤ 3
-- ----------------------------------------------------------------------

theorem outcome_in_canonical_range_preserved_by_initialize
    (s : State) (signer : Pubkey)
    (earliest_slot threshold_price threshold_expo comparison : Nat)
    (h : outcome_in_canonical_range s)
    (s' : State)
    (hstep : initializeTransition s signer earliest_slot threshold_price threshold_expo comparison = some s') :
    outcome_in_canonical_range s' := by
  unfold initializeTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    -- initialize doesn't touch last_returned
    exact h
  case isFalse => simp at hstep

theorem outcome_in_canonical_range_preserved_by_resolve_case_0
    (s : State) (signer : Pubkey) (clock_slot feed_price feed_expo : Nat)
    (_h : outcome_in_canonical_range s)
    (s' : State)
    (hstep : resolve_case_0Transition s signer clock_slot feed_price feed_expo = some s') :
    outcome_in_canonical_range s' := by
  unfold resolve_case_0Transition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    -- post-state has last_returned = 0
    unfold outcome_in_canonical_range
    simp
  case isFalse => simp at hstep

theorem outcome_in_canonical_range_preserved_by_resolve_case_1
    (s : State) (signer : Pubkey) (clock_slot feed_price feed_expo : Nat)
    (_h : outcome_in_canonical_range s)
    (s' : State)
    (hstep : resolve_case_1Transition s signer clock_slot feed_price feed_expo = some s') :
    outcome_in_canonical_range s' := by
  unfold resolve_case_1Transition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold outcome_in_canonical_range
    simp
  case isFalse => simp at hstep

theorem outcome_in_canonical_range_preserved_by_resolve_case_2
    (s : State) (signer : Pubkey) (clock_slot feed_price feed_expo : Nat)
    (_h : outcome_in_canonical_range s)
    (s' : State)
    (hstep : resolve_case_2Transition s signer clock_slot feed_price feed_expo = some s') :
    outcome_in_canonical_range s' := by
  unfold resolve_case_2Transition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold outcome_in_canonical_range
    simp
  case isFalse => simp at hstep

theorem outcome_in_canonical_range_preserved_by_resolve_case_3
    (s : State) (signer : Pubkey) (clock_slot feed_price feed_expo : Nat)
    (_h : outcome_in_canonical_range s)
    (s' : State)
    (hstep : resolve_case_3Transition s signer clock_slot feed_price feed_expo = some s') :
    outcome_in_canonical_range s' := by
  unfold resolve_case_3Transition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold outcome_in_canonical_range
    simp
  case isFalse => simp at hstep

theorem outcome_in_canonical_range_preserved_by_resolve_otherwise
    (s : State) (signer : Pubkey) (clock_slot feed_price feed_expo : Nat)
    (_h : outcome_in_canonical_range s)
    (s' : State)
    (hstep : resolve_otherwiseTransition s signer clock_slot feed_price feed_expo = some s') :
    outcome_in_canonical_range s' := by
  unfold resolve_otherwiseTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold outcome_in_canonical_range
    simp
  case isFalse => simp at hstep

-- ----------------------------------------------------------------------
-- comparison_bounded : initialized = 0 ∨ comparison ∈ {0, 1}
-- ----------------------------------------------------------------------

theorem comparison_bounded_preserved_by_initialize
    (s : State) (signer : Pubkey)
    (earliest_slot threshold_price threshold_expo comparison : Nat)
    (_h : comparison_bounded s)
    (s' : State)
    (hstep : initializeTransition s signer earliest_slot threshold_price threshold_expo comparison = some s') :
    comparison_bounded s' := by
  unfold initializeTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    rcases hg with ⟨_, hle⟩
    -- s'.comparison = comparison and comparison ≤ 1
    right
    have h01 : comparison = 0 ∨ comparison = 1 := by omega
    rcases h01 with heq | heq
    · left; exact heq
    · right; exact heq
  case isFalse => simp at hstep

theorem comparison_bounded_preserved_by_resolve_case_0
    (s : State) (signer : Pubkey) (clock_slot feed_price feed_expo : Nat)
    (h : comparison_bounded s)
    (s' : State)
    (hstep : resolve_case_0Transition s signer clock_slot feed_price feed_expo = some s') :
    comparison_bounded s' := by
  unfold resolve_case_0Transition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem comparison_bounded_preserved_by_resolve_case_1
    (s : State) (signer : Pubkey) (clock_slot feed_price feed_expo : Nat)
    (h : comparison_bounded s)
    (s' : State)
    (hstep : resolve_case_1Transition s signer clock_slot feed_price feed_expo = some s') :
    comparison_bounded s' := by
  unfold resolve_case_1Transition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem comparison_bounded_preserved_by_resolve_case_2
    (s : State) (signer : Pubkey) (clock_slot feed_price feed_expo : Nat)
    (h : comparison_bounded s)
    (s' : State)
    (hstep : resolve_case_2Transition s signer clock_slot feed_price feed_expo = some s') :
    comparison_bounded s' := by
  unfold resolve_case_2Transition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem comparison_bounded_preserved_by_resolve_case_3
    (s : State) (signer : Pubkey) (clock_slot feed_price feed_expo : Nat)
    (h : comparison_bounded s)
    (s' : State)
    (hstep : resolve_case_3Transition s signer clock_slot feed_price feed_expo = some s') :
    comparison_bounded s' := by
  unfold resolve_case_3Transition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem comparison_bounded_preserved_by_resolve_otherwise
    (s : State) (signer : Pubkey) (clock_slot feed_price feed_expo : Nat)
    (h : comparison_bounded s)
    (s' : State)
    (hstep : resolve_otherwiseTransition s signer clock_slot feed_price feed_expo = some s') :
    comparison_bounded s' := by
  unfold resolve_otherwiseTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

end PythPriceResolver
