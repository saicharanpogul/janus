/-
Proofs.lean — hand-written preservation proofs for SlotHeightResolver.

These close every theorem the spec's `property … preserved_by` clause
generates. Each obligation has the shape "if the property holds in the
pre-state and the transition fires, the property holds in the
post-state". For this resolver the proofs are mechanical: every
transition writes a fixed value or `s.outcome_at_or_after`, both of
which trivially satisfy the predicates.

Authored by Claude (Anthropic) directly — `qedgen fill-sorry` ships
with Mistral Leanstral wiring; for proofs this simple, a Lean-aware
coding agent gets there in one pass.
-/
import Spec

namespace SlotHeightResolver

-- ----------------------------------------------------------------------
-- outcome_terminal_after_init :
--   s.initialized = 0 ∨ (outcome_at_or_after ∈ [1, 3])
-- ----------------------------------------------------------------------

theorem outcome_terminal_after_init_preserved_by_initialize
    (s : State) (signer : Pubkey) (target_slot outcome : Nat)
    (_h : outcome_terminal_after_init s)
    (s' : State)
    (hstep : initializeTransition s signer target_slot outcome = some s') :
    outcome_terminal_after_init s' := by
  unfold initializeTransition at hstep
  split at hstep
  case isTrue hguard =>
    -- hguard : s.initialized = 0 ∧ outcome ≥ 1 ∧ outcome ≤ 3
    obtain ⟨_, hge, hle⟩ := hguard
    injection hstep with hs'
    subst hs'
    exact Or.inr ⟨hge, hle⟩
  case isFalse =>
    cases hstep

theorem outcome_terminal_after_init_preserved_by_resolve_case_0
    (s : State) (signer : Pubkey) (clock_slot : Nat)
    (h : outcome_terminal_after_init s)
    (s' : State)
    (hstep : resolve_case_0Transition s signer clock_slot = some s') :
    outcome_terminal_after_init s' := by
  unfold resolve_case_0Transition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'
    subst hs'
    -- post-state writes only `last_returned`; the property's fields are
    -- untouched, so the hypothesis carries through.
    exact h
  case isFalse =>
    cases hstep

theorem outcome_terminal_after_init_preserved_by_resolve_otherwise
    (s : State) (signer : Pubkey) (clock_slot : Nat)
    (h : outcome_terminal_after_init s)
    (s' : State)
    (hstep : resolve_otherwiseTransition s signer clock_slot = some s') :
    outcome_terminal_after_init s' := by
  unfold resolve_otherwiseTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'
    subst hs'
    exact h
  case isFalse =>
    cases hstep

-- ----------------------------------------------------------------------
-- returned_value_bounded :
--   s.last_returned = 0 ∨ s.last_returned = s.outcome_at_or_after
-- ----------------------------------------------------------------------

theorem returned_value_bounded_preserved_by_initialize
    (s : State) (signer : Pubkey) (target_slot outcome : Nat)
    (_h : returned_value_bounded s)
    (s' : State)
    (hstep : initializeTransition s signer target_slot outcome = some s') :
    returned_value_bounded s' := by
  unfold initializeTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'
    subst hs'
    -- post-state has last_returned = 0; left disjunct closes.
    exact Or.inl rfl
  case isFalse =>
    cases hstep

theorem returned_value_bounded_preserved_by_resolve_case_0
    (s : State) (signer : Pubkey) (clock_slot : Nat)
    (_h : returned_value_bounded s)
    (s' : State)
    (hstep : resolve_case_0Transition s signer clock_slot = some s') :
    returned_value_bounded s' := by
  unfold resolve_case_0Transition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'
    subst hs'
    -- pre-deadline case: post-state has last_returned = 0.
    exact Or.inl rfl
  case isFalse =>
    cases hstep

theorem returned_value_bounded_preserved_by_resolve_otherwise
    (s : State) (signer : Pubkey) (clock_slot : Nat)
    (_h : returned_value_bounded s)
    (s' : State)
    (hstep : resolve_otherwiseTransition s signer clock_slot = some s') :
    returned_value_bounded s' := by
  unfold resolve_otherwiseTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'
    subst hs'
    -- post-deadline case: post-state has last_returned = outcome_at_or_after;
    -- the right disjunct closes.
    exact Or.inr rfl
  case isFalse =>
    cases hstep

end SlotHeightResolver
