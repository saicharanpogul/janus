/-
Proofs.lean — preservation proofs for LmsrTrueMarket.

Three invariants × 8 transitions = 24 obligations. All discharged here
by hand:

* `vault_eq_initial_plus_net_flow` is the headline bookkeeping
  invariant. `initialize_pool` establishes it (`initial_subsidy + 0 =
  initial_subsidy`); buys and sells preserve it via Nat arithmetic
  closed by `omega`. Resolution handlers don't touch the relevant
  fields.

* `status_monotone` (status ≤ 3) is preserved by every transition
  because each writes a literal ≤ 3 or doesn't touch status.

* `subsidy_field_stable` / `b_field_stable` are framed as relations
  between pre- and post-states; `initialize_pool` is the only handler
  that writes these fields, and only when status was previously
  UNINIT, which is precisely the disjunct.

Bounded-loss-via-LMSR-cost — i.e., `net_flow ≥ -b·ln(2)` — is *not*
proven here. That requires modeling the LMSR cost function in Lean
and reasoning about `C(q_yes, q_no) - C(0, 0)` over reals; it is the
follow-up Aristotle deliverable.
-/
import Spec

namespace LmsrTrueMarket

-- ---------------------------------------------------------------------------
-- vault_eq_initial_plus_net_flow
-- ---------------------------------------------------------------------------

theorem vault_eq_preserved_by_initialize_pool
    (s : State) (signer : Pubkey) (b initial_subsidy b_ln2_ceil : Nat)
    (_h : vault_eq_initial_plus_net_flow s)
    (s' : State)
    (hstep : initialize_poolTransition s signer b initial_subsidy b_ln2_ceil = some s') :
    vault_eq_initial_plus_net_flow s' := by
  unfold initialize_poolTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    -- s' has status = ACTIVE, vault = initial_subsidy, net_flow = 0
    right; simp
  case isFalse => simp at hstep

theorem vault_eq_preserved_by_buy_yes
    (s : State) (signer : Pubkey) (delta cost : Nat)
    (h : vault_eq_initial_plus_net_flow s)
    (s' : State)
    (hstep : buy_yesTransition s signer delta cost = some s') :
    vault_eq_initial_plus_net_flow s' := by
  unfold buy_yesTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    -- status was ACTIVE, so the s.status = UNINIT disjunct of h is false;
    -- take the equality branch.
    rcases h with h_uninit | h_eq
    · -- s.status = UNINIT contradicts hg.1 : s.status = ACTIVE.
      have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg.1
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right
      -- s'.vault = s.vault + cost; s'.net_flow = s.net_flow + cost
      -- s'.initial_subsidy = s.initial_subsidy (record update preserves it)
      simp [h_eq, Nat.add_assoc]
  case isFalse => simp at hstep

theorem vault_eq_preserved_by_buy_no
    (s : State) (signer : Pubkey) (delta cost : Nat)
    (h : vault_eq_initial_plus_net_flow s)
    (s' : State)
    (hstep : buy_noTransition s signer delta cost = some s') :
    vault_eq_initial_plus_net_flow s' := by
  unfold buy_noTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    rcases h with h_uninit | h_eq
    · have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg.1
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right; simp [h_eq, Nat.add_assoc]
  case isFalse => simp at hstep

theorem vault_eq_preserved_by_sell_yes
    (s : State) (signer : Pubkey) (delta payout : Nat)
    (h : vault_eq_initial_plus_net_flow s)
    (s' : State)
    (hstep : sell_yesTransition s signer delta payout = some s') :
    vault_eq_initial_plus_net_flow s' := by
  unfold sell_yesTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    rcases h with h_uninit | h_eq
    · have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg.1
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right
      simp only []
      obtain ⟨_, _, _, hpv, hpn⟩ := hg
      omega
  case isFalse => simp at hstep

theorem vault_eq_preserved_by_sell_no
    (s : State) (signer : Pubkey) (delta payout : Nat)
    (h : vault_eq_initial_plus_net_flow s)
    (s' : State)
    (hstep : sell_noTransition s signer delta payout = some s') :
    vault_eq_initial_plus_net_flow s' := by
  unfold sell_noTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    rcases h with h_uninit | h_eq
    · have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg.1
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right
      simp only []
      obtain ⟨_, _, _, hpv, hpn⟩ := hg
      omega
  case isFalse => simp at hstep

theorem vault_eq_preserved_by_resolve_yes
    (s : State) (signer : Pubkey)
    (h : vault_eq_initial_plus_net_flow s)
    (s' : State)
    (hstep : resolve_yesTransition s signer = some s') :
    vault_eq_initial_plus_net_flow s' := by
  unfold resolve_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    -- s' differs only by status; vault/initial_subsidy/net_flow unchanged.
    -- s'.status = RESOLVED_YES, so the UNINIT disjunct is false; take the right.
    rcases h with h_uninit | h_eq
    · -- s.status = UNINIT, but the guard required s.status = ACTIVE.
      rename_i hg
      have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right; exact h_eq
  case isFalse => simp at hstep

theorem vault_eq_preserved_by_resolve_no
    (s : State) (signer : Pubkey)
    (h : vault_eq_initial_plus_net_flow s)
    (s' : State)
    (hstep : resolve_noTransition s signer = some s') :
    vault_eq_initial_plus_net_flow s' := by
  unfold resolve_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    rcases h with h_uninit | h_eq
    · rename_i hg
      have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right; exact h_eq
  case isFalse => simp at hstep

theorem vault_eq_preserved_by_resolve_invalid
    (s : State) (signer : Pubkey)
    (h : vault_eq_initial_plus_net_flow s)
    (s' : State)
    (hstep : resolve_invalidTransition s signer = some s') :
    vault_eq_initial_plus_net_flow s' := by
  unfold resolve_invalidTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    rcases h with h_uninit | h_eq
    · rename_i hg
      have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right; exact h_eq
  case isFalse => simp at hstep

-- ---------------------------------------------------------------------------
-- status_monotone : status = UNINIT ∨ status ≤ 3
-- ---------------------------------------------------------------------------

theorem status_monotone_preserved_by_initialize_pool
    (s : State) (signer : Pubkey) (b initial_subsidy b_ln2_ceil : Nat)
    (_h : status_monotone s)
    (s' : State)
    (hstep : initialize_poolTransition s signer b initial_subsidy b_ln2_ceil = some s') :
    status_monotone s' := by
  unfold initialize_poolTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    right; simp [STATUS_ACTIVE]
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_buy_yes
    (s : State) (signer : Pubkey) (delta cost : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : buy_yesTransition s signer delta cost = some s') :
    status_monotone s' := by
  unfold buy_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    -- status unchanged; carry h through.
    rcases h with h_uninit | h_le
    · rename_i hg
      have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg.1
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right; exact h_le
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_buy_no
    (s : State) (signer : Pubkey) (delta cost : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : buy_noTransition s signer delta cost = some s') :
    status_monotone s' := by
  unfold buy_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    rcases h with h_uninit | h_le
    · rename_i hg
      have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg.1
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right; exact h_le
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_sell_yes
    (s : State) (signer : Pubkey) (delta payout : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : sell_yesTransition s signer delta payout = some s') :
    status_monotone s' := by
  unfold sell_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    rcases h with h_uninit | h_le
    · rename_i hg
      have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg.1
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right; exact h_le
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_sell_no
    (s : State) (signer : Pubkey) (delta payout : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : sell_noTransition s signer delta payout = some s') :
    status_monotone s' := by
  unfold sell_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    rcases h with h_uninit | h_le
    · rename_i hg
      have : (STATUS_UNINIT : Nat) = STATUS_ACTIVE := by rw [← h_uninit]; exact hg.1
      simp [STATUS_UNINIT, STATUS_ACTIVE] at this
    · right; exact h_le
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_resolve_yes
    (s : State) (signer : Pubkey)
    (_h : status_monotone s)
    (s' : State)
    (hstep : resolve_yesTransition s signer = some s') :
    status_monotone s' := by
  unfold resolve_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    right; simp [STATUS_RESOLVED_YES]
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_resolve_no
    (s : State) (signer : Pubkey)
    (_h : status_monotone s)
    (s' : State)
    (hstep : resolve_noTransition s signer = some s') :
    status_monotone s' := by
  unfold resolve_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    right; simp [STATUS_RESOLVED_NO]
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_resolve_invalid
    (s : State) (signer : Pubkey)
    (_h : status_monotone s)
    (s' : State)
    (hstep : resolve_invalidTransition s signer = some s') :
    status_monotone s' := by
  unfold resolve_invalidTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    right; simp [STATUS_INVALID]
  case isFalse => simp at hstep

-- ---------------------------------------------------------------------------
-- subsidy_field_stable and b_field_stable: cross-state relations.
-- These follow uniformly: non-init handlers do not touch initial_subsidy
-- or b, and init only fires when prev status = UNINIT.
-- ---------------------------------------------------------------------------

theorem subsidy_field_stable_initialize_pool
    (s : State) (signer : Pubkey) (b initial_subsidy b_ln2_ceil : Nat)
    (s' : State)
    (hstep : initialize_poolTransition s signer b initial_subsidy b_ln2_ceil = some s') :
    subsidy_field_stable s s' := by
  unfold initialize_poolTransition at hstep
  split at hstep
  case isTrue hg =>
    -- Guard establishes s.status = STATUS_UNINIT; left disjunct.
    left; exact hg.1
  case isFalse => simp at hstep

theorem b_field_stable_initialize_pool
    (s : State) (signer : Pubkey) (b initial_subsidy b_ln2_ceil : Nat)
    (s' : State)
    (hstep : initialize_poolTransition s signer b initial_subsidy b_ln2_ceil = some s') :
    b_field_stable s s' := by
  unfold initialize_poolTransition at hstep
  split at hstep
  case isTrue hg =>
    left; exact hg.1
  case isFalse => simp at hstep

/-- Bulk: every non-init transition preserves both `initial_subsidy`
and `b` because it doesn't write those fields. We express this as a
single helper that says s' = { s with ... non-subsidy fields ... }
implies the relation. The proofs reduce to picking the right
disjunct + reflexivity. -/

theorem subsidy_field_stable_buy_yes
    (s : State) (signer : Pubkey) (delta cost : Nat) (s' : State)
    (hstep : buy_yesTransition s signer delta cost = some s') :
    subsidy_field_stable s s' := by
  unfold buy_yesTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem b_field_stable_buy_yes
    (s : State) (signer : Pubkey) (delta cost : Nat) (s' : State)
    (hstep : buy_yesTransition s signer delta cost = some s') :
    b_field_stable s s' := by
  unfold buy_yesTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem subsidy_field_stable_buy_no
    (s : State) (signer : Pubkey) (delta cost : Nat) (s' : State)
    (hstep : buy_noTransition s signer delta cost = some s') :
    subsidy_field_stable s s' := by
  unfold buy_noTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem b_field_stable_buy_no
    (s : State) (signer : Pubkey) (delta cost : Nat) (s' : State)
    (hstep : buy_noTransition s signer delta cost = some s') :
    b_field_stable s s' := by
  unfold buy_noTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem subsidy_field_stable_sell_yes
    (s : State) (signer : Pubkey) (delta payout : Nat) (s' : State)
    (hstep : sell_yesTransition s signer delta payout = some s') :
    subsidy_field_stable s s' := by
  unfold sell_yesTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem b_field_stable_sell_yes
    (s : State) (signer : Pubkey) (delta payout : Nat) (s' : State)
    (hstep : sell_yesTransition s signer delta payout = some s') :
    b_field_stable s s' := by
  unfold sell_yesTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem subsidy_field_stable_sell_no
    (s : State) (signer : Pubkey) (delta payout : Nat) (s' : State)
    (hstep : sell_noTransition s signer delta payout = some s') :
    subsidy_field_stable s s' := by
  unfold sell_noTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem b_field_stable_sell_no
    (s : State) (signer : Pubkey) (delta payout : Nat) (s' : State)
    (hstep : sell_noTransition s signer delta payout = some s') :
    b_field_stable s s' := by
  unfold sell_noTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem subsidy_field_stable_resolve_yes
    (s : State) (signer : Pubkey) (s' : State)
    (hstep : resolve_yesTransition s signer = some s') :
    subsidy_field_stable s s' := by
  unfold resolve_yesTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem b_field_stable_resolve_yes
    (s : State) (signer : Pubkey) (s' : State)
    (hstep : resolve_yesTransition s signer = some s') :
    b_field_stable s s' := by
  unfold resolve_yesTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem subsidy_field_stable_resolve_no
    (s : State) (signer : Pubkey) (s' : State)
    (hstep : resolve_noTransition s signer = some s') :
    subsidy_field_stable s s' := by
  unfold resolve_noTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem b_field_stable_resolve_no
    (s : State) (signer : Pubkey) (s' : State)
    (hstep : resolve_noTransition s signer = some s') :
    b_field_stable s s' := by
  unfold resolve_noTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem subsidy_field_stable_resolve_invalid
    (s : State) (signer : Pubkey) (s' : State)
    (hstep : resolve_invalidTransition s signer = some s') :
    subsidy_field_stable s s' := by
  unfold resolve_invalidTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

theorem b_field_stable_resolve_invalid
    (s : State) (signer : Pubkey) (s' : State)
    (hstep : resolve_invalidTransition s signer = some s') :
    b_field_stable s s' := by
  unfold resolve_invalidTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; right; rfl
  case isFalse => simp at hstep

end LmsrTrueMarket
