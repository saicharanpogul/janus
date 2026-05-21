/-
Proofs.lean — hand-written preservation proofs for ConditionalTokens.

28 obligations total (4 properties × 7 transitions). Authored by
Claude (Anthropic) directly. Key invariants:

  - yes_no_paired_while_active: split/merge bump both supplies by the
    same amount; redeems only fire post-resolution so the first
    disjunct (`status ≠ 0`) closes them.
  - vault_tracks_yes / vault_tracks_no: split & merge keep
    vault = yes = no in lock-step; redeem_yes drops vault and
    yes_supply by the same amount; resolve_no satisfies the first
    disjunct of vault_tracks_yes (status = 2); symmetric for the
    other side.
  - status_monotone: every status write is a literal in [0, 2].
-/
import Spec

namespace ConditionalTokens

-- ----------------------------------------------------------------------
-- status_monotone : s.status ≤ 3
-- ----------------------------------------------------------------------

theorem status_monotone_preserved_by_initialize_market
    (s : State) (signer : Pubkey) (initial_collateral : Nat)
    (_h : status_monotone s)
    (s' : State)
    (hstep : initialize_marketTransition s signer initial_collateral = some s') :
    status_monotone s' := by
  unfold initialize_marketTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    simp [status_monotone]
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_split
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : splitTransition s signer amount = some s') :
    status_monotone s' := by
  unfold splitTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_merge
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : mergeTransition s signer amount = some s') :
    status_monotone s' := by
  unfold mergeTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
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
    simp [status_monotone]
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
    simp [status_monotone]
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_redeem_yes
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : redeem_yesTransition s signer amount = some s') :
    status_monotone s' := by
  unfold redeem_yesTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

theorem status_monotone_preserved_by_redeem_no
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : status_monotone s)
    (s' : State)
    (hstep : redeem_noTransition s signer amount = some s') :
    status_monotone s' := by
  unfold redeem_noTransition at hstep
  split at hstep
  case isTrue => injection hstep with hs'; subst hs'; exact h
  case isFalse => simp at hstep

-- ----------------------------------------------------------------------
-- yes_no_paired_while_active : status ≠ 0 ∨ yes_supply = no_supply
-- ----------------------------------------------------------------------

theorem yes_no_paired_while_active_preserved_by_initialize_market
    (s : State) (signer : Pubkey) (initial_collateral : Nat)
    (_h : yes_no_paired_while_active s)
    (s' : State)
    (hstep : initialize_marketTransition s signer initial_collateral = some s') :
    yes_no_paired_while_active s' := by
  unfold initialize_marketTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    simp [yes_no_paired_while_active]
  case isFalse => simp at hstep

theorem yes_no_paired_while_active_preserved_by_split
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : yes_no_paired_while_active s)
    (s' : State)
    (hstep : splitTransition s signer amount = some s') :
    yes_no_paired_while_active s' := by
  unfold splitTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hstatus : s.status = 0 := hg.2.1
    have hyn : s.yes_supply = s.no_supply := by
      rcases h with hne | heq
      · exact absurd hstatus hne
      · exact heq
    simp [yes_no_paired_while_active]
    omega
  case isFalse => simp at hstep

theorem yes_no_paired_while_active_preserved_by_merge
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : yes_no_paired_while_active s)
    (s' : State)
    (hstep : mergeTransition s signer amount = some s') :
    yes_no_paired_while_active s' := by
  unfold mergeTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hstatus : s.status = 0 := hg.2.1
    have hyn : s.yes_supply = s.no_supply := by
      rcases h with hne | heq
      · exact absurd hstatus hne
      · exact heq
    simp [yes_no_paired_while_active]
    omega
  case isFalse => simp at hstep

theorem yes_no_paired_while_active_preserved_by_resolve_yes
    (s : State) (signer : Pubkey)
    (_h : yes_no_paired_while_active s)
    (s' : State)
    (hstep : resolve_yesTransition s signer = some s') :
    yes_no_paired_while_active s' := by
  unfold resolve_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    simp [yes_no_paired_while_active]
  case isFalse => simp at hstep

theorem yes_no_paired_while_active_preserved_by_resolve_no
    (s : State) (signer : Pubkey)
    (_h : yes_no_paired_while_active s)
    (s' : State)
    (hstep : resolve_noTransition s signer = some s') :
    yes_no_paired_while_active s' := by
  unfold resolve_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    simp [yes_no_paired_while_active]
  case isFalse => simp at hstep

theorem yes_no_paired_while_active_preserved_by_redeem_yes
    (s : State) (signer : Pubkey) (amount : Nat)
    (_h : yes_no_paired_while_active s)
    (s' : State)
    (hstep : redeem_yesTransition s signer amount = some s') :
    yes_no_paired_while_active s' := by
  unfold redeem_yesTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    simp [yes_no_paired_while_active, hg.1]
  case isFalse => simp at hstep

theorem yes_no_paired_while_active_preserved_by_redeem_no
    (s : State) (signer : Pubkey) (amount : Nat)
    (_h : yes_no_paired_while_active s)
    (s' : State)
    (hstep : redeem_noTransition s signer amount = some s') :
    yes_no_paired_while_active s' := by
  unfold redeem_noTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    simp [yes_no_paired_while_active, hg.1]
  case isFalse => simp at hstep

-- ----------------------------------------------------------------------
-- vault_tracks_yes : status = 2 ∨ vault = yes_supply
-- ----------------------------------------------------------------------

theorem vault_tracks_yes_preserved_by_initialize_market
    (s : State) (signer : Pubkey) (initial_collateral : Nat)
    (_h : vault_tracks_yes s)
    (s' : State)
    (hstep : initialize_marketTransition s signer initial_collateral = some s') :
    vault_tracks_yes s' := by
  unfold initialize_marketTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    simp [vault_tracks_yes]
  case isFalse => simp at hstep

theorem vault_tracks_yes_preserved_by_split
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : vault_tracks_yes s)
    (s' : State)
    (hstep : splitTransition s signer amount = some s') :
    vault_tracks_yes s' := by
  unfold splitTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hstatus : s.status = 0 := hg.2.1
    have hv : s.vault = s.yes_supply := by
      rcases h with heq | heq
      · omega
      · exact heq
    simp [vault_tracks_yes]
    omega
  case isFalse => simp at hstep

theorem vault_tracks_yes_preserved_by_merge
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : vault_tracks_yes s)
    (s' : State)
    (hstep : mergeTransition s signer amount = some s') :
    vault_tracks_yes s' := by
  unfold mergeTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hstatus : s.status = 0 := hg.2.1
    have hv : s.vault = s.yes_supply := by
      rcases h with heq | heq
      · omega
      · exact heq
    simp [vault_tracks_yes]
    omega
  case isFalse => simp at hstep

theorem vault_tracks_yes_preserved_by_resolve_yes
    (s : State) (signer : Pubkey)
    (h : vault_tracks_yes s)
    (s' : State)
    (hstep : resolve_yesTransition s signer = some s') :
    vault_tracks_yes s' := by
  unfold resolve_yesTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hstatus : s.status = 0 := hg.2
    have hv : s.vault = s.yes_supply := by
      rcases h with heq | heq
      · omega
      · exact heq
    simp [vault_tracks_yes, hv]
  case isFalse => simp at hstep

theorem vault_tracks_yes_preserved_by_resolve_no
    (s : State) (signer : Pubkey)
    (_h : vault_tracks_yes s)
    (s' : State)
    (hstep : resolve_noTransition s signer = some s') :
    vault_tracks_yes s' := by
  unfold resolve_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    simp [vault_tracks_yes]
  case isFalse => simp at hstep

theorem vault_tracks_yes_preserved_by_redeem_yes
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : vault_tracks_yes s)
    (s' : State)
    (hstep : redeem_yesTransition s signer amount = some s') :
    vault_tracks_yes s' := by
  unfold redeem_yesTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hstatus : s.status = 1 := hg.1
    have hv : s.vault = s.yes_supply := by
      rcases h with heq | heq
      · omega
      · exact heq
    simp [vault_tracks_yes, hstatus]
    omega
  case isFalse => simp at hstep

theorem vault_tracks_yes_preserved_by_redeem_no
    (s : State) (signer : Pubkey) (amount : Nat)
    (_h : vault_tracks_yes s)
    (s' : State)
    (hstep : redeem_noTransition s signer amount = some s') :
    vault_tracks_yes s' := by
  unfold redeem_noTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    simp [vault_tracks_yes, hg.1]
  case isFalse => simp at hstep

-- ----------------------------------------------------------------------
-- vault_tracks_no : status = 1 ∨ vault = no_supply
-- ----------------------------------------------------------------------

theorem vault_tracks_no_preserved_by_initialize_market
    (s : State) (signer : Pubkey) (initial_collateral : Nat)
    (_h : vault_tracks_no s)
    (s' : State)
    (hstep : initialize_marketTransition s signer initial_collateral = some s') :
    vault_tracks_no s' := by
  unfold initialize_marketTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    simp [vault_tracks_no]
  case isFalse => simp at hstep

theorem vault_tracks_no_preserved_by_split
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : vault_tracks_no s)
    (s' : State)
    (hstep : splitTransition s signer amount = some s') :
    vault_tracks_no s' := by
  unfold splitTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hstatus : s.status = 0 := hg.2.1
    have hv : s.vault = s.no_supply := by
      rcases h with heq | heq
      · omega
      · exact heq
    simp [vault_tracks_no]
    omega
  case isFalse => simp at hstep

theorem vault_tracks_no_preserved_by_merge
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : vault_tracks_no s)
    (s' : State)
    (hstep : mergeTransition s signer amount = some s') :
    vault_tracks_no s' := by
  unfold mergeTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hstatus : s.status = 0 := hg.2.1
    have hv : s.vault = s.no_supply := by
      rcases h with heq | heq
      · omega
      · exact heq
    simp [vault_tracks_no]
    omega
  case isFalse => simp at hstep

theorem vault_tracks_no_preserved_by_resolve_yes
    (s : State) (signer : Pubkey)
    (_h : vault_tracks_no s)
    (s' : State)
    (hstep : resolve_yesTransition s signer = some s') :
    vault_tracks_no s' := by
  unfold resolve_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    simp [vault_tracks_no]
  case isFalse => simp at hstep

theorem vault_tracks_no_preserved_by_resolve_no
    (s : State) (signer : Pubkey)
    (h : vault_tracks_no s)
    (s' : State)
    (hstep : resolve_noTransition s signer = some s') :
    vault_tracks_no s' := by
  unfold resolve_noTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hstatus : s.status = 0 := hg.2
    have hv : s.vault = s.no_supply := by
      rcases h with heq | heq
      · omega
      · exact heq
    simp [vault_tracks_no, hv]
  case isFalse => simp at hstep

theorem vault_tracks_no_preserved_by_redeem_yes
    (s : State) (signer : Pubkey) (amount : Nat)
    (_h : vault_tracks_no s)
    (s' : State)
    (hstep : redeem_yesTransition s signer amount = some s') :
    vault_tracks_no s' := by
  unfold redeem_yesTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    simp [vault_tracks_no, hg.1]
  case isFalse => simp at hstep

theorem vault_tracks_no_preserved_by_redeem_no
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : vault_tracks_no s)
    (s' : State)
    (hstep : redeem_noTransition s signer amount = some s') :
    vault_tracks_no s' := by
  unfold redeem_noTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hstatus : s.status = 2 := hg.1
    have hv : s.vault = s.no_supply := by
      rcases h with heq | heq
      · omega
      · exact heq
    simp [vault_tracks_no, hstatus]
    omega
  case isFalse => simp at hstep

-- ----------------------------------------------------------------------
-- total_collateral_conserved :
--   collateral_balance + vault == initial_collateral
--
-- Every handler either leaves both fields untouched, or moves the same
-- amount between them (split / merge / redeem_*). omega closes each
-- case using the guard's `state.<x> >= amount` hypothesis to discharge
-- the Nat subtraction.
-- ----------------------------------------------------------------------

theorem total_collateral_conserved_preserved_by_initialize_market
    (s : State) (signer : Pubkey) (initial_collateral : Nat)
    (_h : total_collateral_conserved s)
    (s' : State)
    (hstep : initialize_marketTransition s signer initial_collateral = some s') :
    total_collateral_conserved s' := by
  unfold initialize_marketTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    -- post: collateral_balance = initial_collateral, vault = 0, initial_collateral = initial_collateral
    -- ⇒ initial_collateral + 0 = initial_collateral.
    simp [total_collateral_conserved]
  case isFalse => simp at hstep

theorem total_collateral_conserved_preserved_by_split
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : total_collateral_conserved s)
    (s' : State)
    (hstep : splitTransition s signer amount = some s') :
    total_collateral_conserved s' := by
  unfold splitTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    -- guard ensures s.collateral_balance >= amount.
    have hcb : s.collateral_balance ≥ amount := hg.2.2.2
    simp [total_collateral_conserved]
    unfold total_collateral_conserved at h
    omega
  case isFalse => simp at hstep

theorem total_collateral_conserved_preserved_by_merge
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : total_collateral_conserved s)
    (s' : State)
    (hstep : mergeTransition s signer amount = some s') :
    total_collateral_conserved s' := by
  unfold mergeTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    -- guard ensures s.vault >= amount.
    have hv : s.vault ≥ amount := hg.2.2.2.2.2
    simp [total_collateral_conserved]
    unfold total_collateral_conserved at h
    omega
  case isFalse => simp at hstep

theorem total_collateral_conserved_preserved_by_resolve_yes
    (s : State) (signer : Pubkey)
    (h : total_collateral_conserved s)
    (s' : State)
    (hstep : resolve_yesTransition s signer = some s') :
    total_collateral_conserved s' := by
  unfold resolve_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    -- resolve_yes only flips status; balances untouched.
    exact h
  case isFalse => simp at hstep

theorem total_collateral_conserved_preserved_by_resolve_no
    (s : State) (signer : Pubkey)
    (h : total_collateral_conserved s)
    (s' : State)
    (hstep : resolve_noTransition s signer = some s') :
    total_collateral_conserved s' := by
  unfold resolve_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    exact h
  case isFalse => simp at hstep

theorem total_collateral_conserved_preserved_by_redeem_yes
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : total_collateral_conserved s)
    (s' : State)
    (hstep : redeem_yesTransition s signer amount = some s') :
    total_collateral_conserved s' := by
  unfold redeem_yesTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hv : s.vault ≥ amount := hg.2.2.2
    simp [total_collateral_conserved]
    unfold total_collateral_conserved at h
    omega
  case isFalse => simp at hstep

theorem total_collateral_conserved_preserved_by_redeem_no
    (s : State) (signer : Pubkey) (amount : Nat)
    (h : total_collateral_conserved s)
    (s' : State)
    (hstep : redeem_noTransition s signer amount = some s') :
    total_collateral_conserved s' := by
  unfold redeem_noTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    have hv : s.vault ≥ amount := hg.2.2.2
    simp [total_collateral_conserved]
    unfold total_collateral_conserved at h
    omega
  case isFalse => simp at hstep

end ConditionalTokens
