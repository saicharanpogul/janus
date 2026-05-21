/-
Proofs.lean — hand-written preservation proofs for AccountLayer.

12 obligations (2 properties × 6 handlers). The key lemma is
`QEDGen.Solana.IndexedState.sum_update_proj_bilinear`:

    (∑ j, proj (update f i v j)) + proj (f i)
      = (∑ j, proj (f j)) + proj v

For mint/burn ops we instantiate it with `proj = (·.yes_balance)` or
`(·.no_balance)` and the new account record at index i; rearrange to
get `sum_after = sum_before ± amount`, combine with the hypothesis.
For transfer we apply it twice (once per index updated); the two
deltas cancel.

For the orthogonal property (mint_yes vs no_supply_matches_sum etc.),
`sum_update_proj_eq` gives `sum_after = sum_before` when the projected
value didn't change.

Authored by Claude (Anthropic).
-/

import Spec
import Mathlib.Algebra.BigOperators.Fin
import Mathlib.Tactic

namespace AccountLayer

open QEDGen.Solana.IndexedState

-- ----------------------------------------------------------------------
-- yes_supply_matches_sum
-- ----------------------------------------------------------------------

theorem yes_supply_matches_sum_preserved_by_mint_yes
    (s : State) (signer : Pubkey) (i : AccountIdx) (amount : Nat)
    (h : yes_supply_matches_sum s)
    (s' : State)
    (hstep : mint_yesTransition s signer i amount = some s') :
    yes_supply_matches_sum s' := by
  unfold mint_yesTransition at hstep
  injection hstep with hs'; subst hs'
  unfold yes_supply_matches_sum at h ⊢
  simp only [Map.set]
  have key := sum_update_proj_bilinear (β := Account) (γ := Nat)
    s.accounts i
    { (s.accounts i) with yes_balance := (s.accounts i).yes_balance + amount }
    (fun a => a.yes_balance)
  simp at key
  -- Substitute h into key so omega sees a single "total_yes_supply" term.
  rw [← h] at key
  linarith

theorem yes_supply_matches_sum_preserved_by_mint_no
    (s : State) (signer : Pubkey) (i : AccountIdx) (amount : Nat)
    (h : yes_supply_matches_sum s)
    (s' : State)
    (hstep : mint_noTransition s signer i amount = some s') :
    yes_supply_matches_sum s' := by
  unfold mint_noTransition at hstep
  injection hstep with hs'; subst hs'
  unfold yes_supply_matches_sum at h ⊢
  simp only [Map.set]
  have key := sum_update_proj_eq (β := Account) (γ := Nat)
    s.accounts i
    { (s.accounts i) with no_balance := (s.accounts i).no_balance + amount }
    (fun a => a.yes_balance)
    (by rfl)
  simp at key
  rw [key]
  exact h

theorem yes_supply_matches_sum_preserved_by_burn_yes
    (s : State) (signer : Pubkey) (i : AccountIdx) (amount : Nat)
    (h : yes_supply_matches_sum s)
    (s' : State)
    (hstep : burn_yesTransition s signer i amount = some s') :
    yes_supply_matches_sum s' := by
  unfold burn_yesTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    unfold yes_supply_matches_sum at h ⊢
    simp only [Map.set]
    -- TODO(burn arithmetic): the bilinear lemma + hsub_cancel gives the
    -- algebraic identity (yes_supply - amount = sum_after) but Lean's
    -- omega/linarith trip on the alpha-equivalence of two ∑ expressions
    -- across the rewrite. Fix: rewrite key in stages with explicit
    -- Finset.sum_update_of_mem from Mathlib. Leaving as sorry for now.
    have h_sum : (∑ j, (Function.update s.accounts i { yes_balance := (s.accounts i).yes_balance - amount, no_balance := (s.accounts i).no_balance } j).yes_balance) + (s.accounts i).yes_balance = (∑ j, (s.accounts j).yes_balance) + ((s.accounts i).yes_balance - amount) :=
      sum_update_proj_bilinear s.accounts i
        { yes_balance := (s.accounts i).yes_balance - amount, no_balance := (s.accounts i).no_balance } Account.yes_balance
    lia
  case isFalse => simp at hstep

theorem yes_supply_matches_sum_preserved_by_burn_no
    (s : State) (signer : Pubkey) (i : AccountIdx) (amount : Nat)
    (h : yes_supply_matches_sum s)
    (s' : State)
    (hstep : burn_noTransition s signer i amount = some s') :
    yes_supply_matches_sum s' := by
  unfold burn_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold yes_supply_matches_sum at h ⊢
    simp only [Map.set]
    have key := sum_update_proj_eq (β := Account) (γ := Nat)
      s.accounts i
      { (s.accounts i) with no_balance := (s.accounts i).no_balance - amount }
      (fun a => a.yes_balance)
      (by rfl)
    simp at key
    rw [key]
    exact h
  case isFalse => simp at hstep

theorem yes_supply_matches_sum_preserved_by_transfer_yes
    (s : State) (signer : Pubkey)
    (from_idx to_idx : AccountIdx) (amount : Nat)
    (h : yes_supply_matches_sum s)
    (s' : State)
    (hstep : transfer_yesTransition s signer from_idx to_idx amount = some s') :
    yes_supply_matches_sum s' := by
  unfold transfer_yesTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    unfold yes_supply_matches_sum at h ⊢
    simp only [Map.set]
    by_cases heq : from_idx = to_idx
    · -- Same index: net delta zero, sum unchanged.
      subst heq
      -- TODO: same-index branch — sum invariant under no-op composed
      -- update. Held up by the same sum-update normalization issue.
      grind
    · -- Distinct indices: from loses, to gains; deltas cancel.
      have key_from := sum_update_proj_bilinear (β := Account) (γ := Nat)
        s.accounts from_idx
        { (s.accounts from_idx) with yes_balance := (s.accounts from_idx).yes_balance - amount }
        (fun a => a.yes_balance)
      simp at key_from
      have key_to := sum_update_proj_bilinear (β := Account) (γ := Nat)
        (Function.update s.accounts from_idx
          { (s.accounts from_idx) with yes_balance := (s.accounts from_idx).yes_balance - amount })
        to_idx
        { (Function.update s.accounts from_idx
            { (s.accounts from_idx) with yes_balance := (s.accounts from_idx).yes_balance - amount }
            to_idx) with
          yes_balance := (Function.update s.accounts from_idx
            { (s.accounts from_idx) with yes_balance := (s.accounts from_idx).yes_balance - amount }
            to_idx).yes_balance + amount }
        (fun a => a.yes_balance)
      -- TODO: two bilinear applications cancel; same omega-on-sums issue.
      grind
  case isFalse => simp at hstep

theorem yes_supply_matches_sum_preserved_by_transfer_no
    (s : State) (signer : Pubkey)
    (from_idx to_idx : AccountIdx) (amount : Nat)
    (h : yes_supply_matches_sum s)
    (s' : State)
    (hstep : transfer_noTransition s signer from_idx to_idx amount = some s') :
    yes_supply_matches_sum s' := by
  unfold transfer_noTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold yes_supply_matches_sum at h ⊢
    simp only [Map.set]
    have step1 := sum_update_proj_eq (β := Account) (γ := Nat)
      s.accounts from_idx
      { (s.accounts from_idx) with no_balance := (s.accounts from_idx).no_balance - amount }
      (fun a => a.yes_balance)
      (by rfl)
    simp at step1
    have step2 := sum_update_proj_eq (β := Account) (γ := Nat)
      (Function.update s.accounts from_idx
        { (s.accounts from_idx) with no_balance := (s.accounts from_idx).no_balance - amount })
      to_idx
      { (Function.update s.accounts from_idx
          { (s.accounts from_idx) with no_balance := (s.accounts from_idx).no_balance - amount }
          to_idx) with
        no_balance := (Function.update s.accounts from_idx
          { (s.accounts from_idx) with no_balance := (s.accounts from_idx).no_balance - amount }
          to_idx).no_balance + amount }
      (fun a => a.yes_balance)
      (by rfl)
    simp at step2
    rw [step2, step1]
    exact h
  case isFalse => simp at hstep

-- ----------------------------------------------------------------------
-- no_supply_matches_sum (symmetric)
-- ----------------------------------------------------------------------

theorem no_supply_matches_sum_preserved_by_mint_no
    (s : State) (signer : Pubkey) (i : AccountIdx) (amount : Nat)
    (h : no_supply_matches_sum s)
    (s' : State)
    (hstep : mint_noTransition s signer i amount = some s') :
    no_supply_matches_sum s' := by
  unfold mint_noTransition at hstep
  injection hstep with hs'; subst hs'
  unfold no_supply_matches_sum at h ⊢
  simp only [Map.set]
  have key := sum_update_proj_bilinear (β := Account) (γ := Nat)
    s.accounts i
    { (s.accounts i) with no_balance := (s.accounts i).no_balance + amount }
    (fun a => a.no_balance)
  simp at key
  rw [← h] at key
  linarith

theorem no_supply_matches_sum_preserved_by_mint_yes
    (s : State) (signer : Pubkey) (i : AccountIdx) (amount : Nat)
    (h : no_supply_matches_sum s)
    (s' : State)
    (hstep : mint_yesTransition s signer i amount = some s') :
    no_supply_matches_sum s' := by
  unfold mint_yesTransition at hstep
  injection hstep with hs'; subst hs'
  unfold no_supply_matches_sum at h ⊢
  simp only [Map.set]
  have key := sum_update_proj_eq (β := Account) (γ := Nat)
    s.accounts i
    { (s.accounts i) with yes_balance := (s.accounts i).yes_balance + amount }
    (fun a => a.no_balance)
    (by rfl)
  simp at key
  rw [key]
  exact h

theorem no_supply_matches_sum_preserved_by_burn_no
    (s : State) (signer : Pubkey) (i : AccountIdx) (amount : Nat)
    (h : no_supply_matches_sum s)
    (s' : State)
    (hstep : burn_noTransition s signer i amount = some s') :
    no_supply_matches_sum s' := by
  unfold burn_noTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    unfold no_supply_matches_sum at h ⊢
    simp only [Map.set]
    -- TODO: same as burn_yes_preserved_by — finite-sum + Nat-sub omega
    -- normalization issue. Resolution path identical.
    have := sum_update_proj_bilinear s.accounts i ( { yes_balance := ( s.accounts i ).yes_balance, no_balance := ( s.accounts i ).no_balance - amount } ) ( fun x => x.no_balance );
    grind
  case isFalse => simp at hstep

theorem no_supply_matches_sum_preserved_by_burn_yes
    (s : State) (signer : Pubkey) (i : AccountIdx) (amount : Nat)
    (h : no_supply_matches_sum s)
    (s' : State)
    (hstep : burn_yesTransition s signer i amount = some s') :
    no_supply_matches_sum s' := by
  unfold burn_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold no_supply_matches_sum at h ⊢
    simp only [Map.set]
    have key := sum_update_proj_eq (β := Account) (γ := Nat)
      s.accounts i
      { (s.accounts i) with yes_balance := (s.accounts i).yes_balance - amount }
      (fun a => a.no_balance)
      (by rfl)
    simp at key
    rw [key]
    exact h
  case isFalse => simp at hstep

theorem no_supply_matches_sum_preserved_by_transfer_no
    (s : State) (signer : Pubkey)
    (from_idx to_idx : AccountIdx) (amount : Nat)
    (h : no_supply_matches_sum s)
    (s' : State)
    (hstep : transfer_noTransition s signer from_idx to_idx amount = some s') :
    no_supply_matches_sum s' := by
  unfold transfer_noTransition at hstep
  split at hstep
  case isTrue hg =>
    injection hstep with hs'; subst hs'
    unfold no_supply_matches_sum at h ⊢
    simp only [Map.set]
    by_cases heq : from_idx = to_idx
    · subst heq
      grind +splitImp
    · have key_from := sum_update_proj_bilinear (β := Account) (γ := Nat)
        s.accounts from_idx
        { (s.accounts from_idx) with no_balance := (s.accounts from_idx).no_balance - amount }
        (fun a => a.no_balance)
      simp at key_from
      have key_to := sum_update_proj_bilinear (β := Account) (γ := Nat)
        (Function.update s.accounts from_idx
          { (s.accounts from_idx) with no_balance := (s.accounts from_idx).no_balance - amount })
        to_idx
        { (Function.update s.accounts from_idx
            { (s.accounts from_idx) with no_balance := (s.accounts from_idx).no_balance - amount }
            to_idx) with
          no_balance := (Function.update s.accounts from_idx
            { (s.accounts from_idx) with no_balance := (s.accounts from_idx).no_balance - amount }
            to_idx).no_balance + amount }
        (fun a => a.no_balance)
      -- TODO: two bilinear applications cancel; same omega-on-sums issue.
      grind
  case isFalse => simp at hstep

theorem no_supply_matches_sum_preserved_by_transfer_yes
    (s : State) (signer : Pubkey)
    (from_idx to_idx : AccountIdx) (amount : Nat)
    (h : no_supply_matches_sum s)
    (s' : State)
    (hstep : transfer_yesTransition s signer from_idx to_idx amount = some s') :
    no_supply_matches_sum s' := by
  unfold transfer_yesTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'; subst hs'
    unfold no_supply_matches_sum at h ⊢
    simp only [Map.set]
    have step1 := sum_update_proj_eq (β := Account) (γ := Nat)
      s.accounts from_idx
      { (s.accounts from_idx) with yes_balance := (s.accounts from_idx).yes_balance - amount }
      (fun a => a.no_balance)
      (by rfl)
    simp at step1
    have step2 := sum_update_proj_eq (β := Account) (γ := Nat)
      (Function.update s.accounts from_idx
        { (s.accounts from_idx) with yes_balance := (s.accounts from_idx).yes_balance - amount })
      to_idx
      { (Function.update s.accounts from_idx
          { (s.accounts from_idx) with yes_balance := (s.accounts from_idx).yes_balance - amount }
          to_idx) with
        yes_balance := (Function.update s.accounts from_idx
          { (s.accounts from_idx) with yes_balance := (s.accounts from_idx).yes_balance - amount }
          to_idx).yes_balance + amount }
      (fun a => a.no_balance)
      (by rfl)
    simp at step2
    rw [step2, step1]
    exact h
  case isFalse => simp at hstep

end AccountLayer