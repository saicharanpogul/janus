/-
Proofs.lean — hand-written preservation proof for MarketFactory.

Closed by Claude (Anthropic) directly. The single property is trivially
inductive: `register` sets `initialized := 1`, so the right disjunct
closes.
-/
import Spec

namespace MarketFactory

theorem registration_terminal_preserved_by_register
    (s : State) (signer : Pubkey) (deadline_slot current_slot : Nat)
    (_h : registration_terminal s)
    (s' : State)
    (hstep : registerTransition s signer deadline_slot current_slot = some s') :
    registration_terminal s' := by
  unfold registerTransition at hstep
  split at hstep
  case isTrue =>
    injection hstep with hs'
    subst hs'
    -- post-state has initialized = 1, the second disjunct.
    exact Or.inr rfl
  case isFalse =>
    cases hstep

end MarketFactory
