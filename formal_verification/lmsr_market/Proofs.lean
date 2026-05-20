/-
Proofs.lean — hand-written preservation proofs for LmsrMarket.

21 obligations total (3 properties × 7 transitions). All close
mechanically because every transition either:
  - leaves the property's fields untouched (so the hypothesis carries
    through), or
  - writes a literal that satisfies the property by construction
    (status := 0/1/2, fee_bps := bounded input, reserves := input
    or strictly-positive sum), or
  - puts the system in a `status ≠ 0` regime that trivially closes
    swap_no_drain by its first disjunct.

Authored by Claude (Anthropic) directly.
-/
import Spec

namespace LmsrMarket

-- ----------------------------------------------------------------------
-- fee_bps_bounded : s.fee_bps ≤ 1000
-- ----------------------------------------------------------------------

theorem fee_bps_bounded_preserved_by_initialize_pool
    (s : State) (signer : Pubkey) (subsidy_yes subsidy_no fee_bps : Nat)
    (_h : fee_bps_bounded s)
    (s' : State)
    (hstep : initialize_poolTransition s signer subsidy_yes subsidy_no fee_bps = some s') :
    fee_bps_bounded s' := by
  unfold initialize_poolTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    -- s'.fee_bps = fee_bps and fee_bps ≤ 1000 from the guard.
    exact hg.2.2.2
  case isFalse => simp at hstep

theorem fee_bps_bounded_preserved_by_swap_yes_for_no
    (s : State) (signer : Pubkey) (amount_in amount_out min_amount_out : Nat)
    (h : fee_bps_bounded s)
    (s' : State)
    (hstep : swap_yes_for_noTransition s signer amount_in amount_out min_amount_out = some s') :
    fee_bps_bounded s' := by
  unfold swap_yes_for_noTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem fee_bps_bounded_preserved_by_swap_no_for_yes
    (s : State) (signer : Pubkey) (amount_in amount_out min_amount_out : Nat)
    (h : fee_bps_bounded s)
    (s' : State)
    (hstep : swap_no_for_yesTransition s signer amount_in amount_out min_amount_out = some s') :
    fee_bps_bounded s' := by
  unfold swap_no_for_yesTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem fee_bps_bounded_preserved_by_mark_resolved_yes
    (s : State) (signer : Pubkey)
    (h : fee_bps_bounded s)
    (s' : State)
    (hstep : mark_resolved_yesTransition s signer = some s') :
    fee_bps_bounded s' := by
  unfold mark_resolved_yesTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem fee_bps_bounded_preserved_by_mark_resolved_no
    (s : State) (signer : Pubkey)
    (h : fee_bps_bounded s)
    (s' : State)
    (hstep : mark_resolved_noTransition s signer = some s') :
    fee_bps_bounded s' := by
  unfold mark_resolved_noTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem fee_bps_bounded_preserved_by_withdraw_yes_after_resolution
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : fee_bps_bounded s)
    (s' : State)
    (hstep : withdraw_yes_after_resolutionTransition s signer amount = some s') :
    fee_bps_bounded s' := by
  unfold withdraw_yes_after_resolutionTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem fee_bps_bounded_preserved_by_withdraw_no_after_resolution
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : fee_bps_bounded s)
    (s' : State)
    (hstep : withdraw_no_after_resolutionTransition s signer amount = some s') :
    fee_bps_bounded s' := by
  unfold withdraw_no_after_resolutionTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

-- ----------------------------------------------------------------------
-- status_monotone : s.status ≤ 2
-- ----------------------------------------------------------------------

theorem status_monotone_preserved_by_initialize_pool
    (s : State) (signer : Pubkey) (subsidy_yes subsidy_no fee_bps : Nat)
    (_h : status_monotone s)
    (s' : State)
    (hstep : initialize_poolTransition s signer subsidy_yes subsidy_no fee_bps = some s') :
    status_monotone s' := by
  unfold initialize_poolTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold status_monotone; simp
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_swap_yes_for_no
    (s : State) (signer : Pubkey) (amount_in amount_out min_amount_out : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : swap_yes_for_noTransition s signer amount_in amount_out min_amount_out = some s') :
    status_monotone s' := by
  unfold swap_yes_for_noTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_swap_no_for_yes
    (s : State) (signer : Pubkey) (amount_in amount_out min_amount_out : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : swap_no_for_yesTransition s signer amount_in amount_out min_amount_out = some s') :
    status_monotone s' := by
  unfold swap_no_for_yesTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_mark_resolved_yes
    (s : State) (signer : Pubkey)
    (_h : status_monotone s)
    (s' : State)
    (hstep : mark_resolved_yesTransition s signer = some s') :
    status_monotone s' := by
  unfold mark_resolved_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold status_monotone; simp
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_mark_resolved_no
    (s : State) (signer : Pubkey)
    (_h : status_monotone s)
    (s' : State)
    (hstep : mark_resolved_noTransition s signer = some s') :
    status_monotone s' := by
  unfold mark_resolved_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold status_monotone; simp
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_withdraw_yes_after_resolution
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : withdraw_yes_after_resolutionTransition s signer amount = some s') :
    status_monotone s' := by
  unfold withdraw_yes_after_resolutionTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_withdraw_no_after_resolution
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : withdraw_no_after_resolutionTransition s signer amount = some s') :
    status_monotone s' := by
  unfold withdraw_no_after_resolutionTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

-- ----------------------------------------------------------------------
-- swap_no_drain : s.status ≠ 0 ∨ s.yes_reserves > 0 ∨ s.no_reserves > 0
-- ----------------------------------------------------------------------

theorem swap_no_drain_preserved_by_initialize_pool
    (s : State) (signer : Pubkey) (subsidy_yes subsidy_no fee_bps : Nat)
    (_h : swap_no_drain s)
    (s' : State)
    (hstep : initialize_poolTransition s signer subsidy_yes subsidy_no fee_bps = some s') :
    swap_no_drain s' := by
  unfold initialize_poolTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    -- post-state: yes_reserves = subsidy_yes > 0 (from guard).
    right; left
    exact hg.2.1
  case isFalse => simp at hstep

theorem swap_no_drain_preserved_by_swap_yes_for_no
    (s : State) (signer : Pubkey) (amount_in amount_out min_amount_out : Nat)
    (_h : swap_no_drain s)
    (s' : State)
    (hstep : swap_yes_for_noTransition s signer amount_in amount_out min_amount_out = some s') :
    swap_no_drain s' := by
  unfold swap_yes_for_noTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    -- post-state: yes_reserves = s.yes_reserves + amount_in ≥ amount_in > 0.
    obtain ⟨_, h_in, _, _⟩ := hg
    simp [swap_no_drain]
    omega
  case isFalse => simp at hstep

theorem swap_no_drain_preserved_by_swap_no_for_yes
    (s : State) (signer : Pubkey) (amount_in amount_out min_amount_out : Nat)
    (_h : swap_no_drain s)
    (s' : State)
    (hstep : swap_no_for_yesTransition s signer amount_in amount_out min_amount_out = some s') :
    swap_no_drain s' := by
  unfold swap_no_for_yesTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    obtain ⟨_, h_in, _, _⟩ := hg
    simp [swap_no_drain]
    omega
  case isFalse => simp at hstep

theorem swap_no_drain_preserved_by_mark_resolved_yes
    (s : State) (signer : Pubkey)
    (_h : swap_no_drain s)
    (s' : State)
    (hstep : mark_resolved_yesTransition s signer = some s') :
    swap_no_drain s' := by
  unfold mark_resolved_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    -- post-state: status = 1 ≠ 0; simp normalises the record projection.
    simp [swap_no_drain]
  case isFalse => simp at hstep

theorem swap_no_drain_preserved_by_mark_resolved_no
    (s : State) (signer : Pubkey)
    (_h : swap_no_drain s)
    (s' : State)
    (hstep : mark_resolved_noTransition s signer = some s') :
    swap_no_drain s' := by
  unfold mark_resolved_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    simp [swap_no_drain]
  case isFalse => simp at hstep

theorem swap_no_drain_preserved_by_withdraw_yes_after_resolution
    (s : State) (signer : Pubkey) (amount : Nat)
    (_h : swap_no_drain s)
    (s' : State)
    (hstep : withdraw_yes_after_resolutionTransition s signer amount = some s') :
    swap_no_drain s' := by
  unfold withdraw_yes_after_resolutionTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    -- s.status = 1 from guard; post-state inherits it (record update only
    -- touches yes_reserves).
    simp [swap_no_drain, hg.1]
  case isFalse => simp at hstep

theorem swap_no_drain_preserved_by_withdraw_no_after_resolution
    (s : State) (signer : Pubkey) (amount : Nat)
    (_h : swap_no_drain s)
    (s' : State)
    (hstep : withdraw_no_after_resolutionTransition s signer amount = some s') :
    swap_no_drain s' := by
  unfold withdraw_no_after_resolutionTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    simp [swap_no_drain, hg.1]
  case isFalse => simp at hstep

end LmsrMarket
