import QEDGen.Solana.Account

namespace SlotHeightResolver

open QEDGen.Solana.Account

abbrev Pubkey := QEDGen.Solana.Account.Pubkey

abbrev OUTCOME_UNRESOLVED : Nat := 0
abbrev OUTCOME_YES : Nat := 1
abbrev OUTCOME_NO : Nat := 2
abbrev OUTCOME_INVALID : Nat := 3

abbrev AccountIdx : Type := Fin 1024

structure State where
  initialized : Nat
  target_slot : Nat
  outcome_at_or_after : Nat
  last_returned : Nat
  deriving Repr, DecidableEq, BEq

instance : Inhabited State := ⟨{
  initialized := 0,
  target_slot := 0,
  outcome_at_or_after := 0,
  last_returned := 0,
}⟩

def initializeTransition (s : State) (signer : Pubkey) (target_slot : Nat) (outcome : Nat) : Option State :=
  if (s.initialized = 0) ∧ (outcome ≥ 1) ∧ (outcome ≤ 3) then
    some { s with initialized := 1, target_slot := target_slot, outcome_at_or_after := outcome, last_returned := 0 }
  else none

def resolve_case_0Transition (s : State) (signer : Pubkey) (clock_slot : Nat) : Option State :=
  if (clock_slot < s.target_slot) then
    some { s with last_returned := 0 }
  else none

def resolve_otherwiseTransition (s : State) (signer : Pubkey) (clock_slot : Nat) : Option State :=
  if (¬(clock_slot < s.target_slot)) then
    some { s with last_returned := s.outcome_at_or_after }
  else none

inductive Operation where
  | «initialize» (target_slot : Nat) (outcome : Nat)
  | resolve_case_0 (clock_slot : Nat)
  | resolve_otherwise (clock_slot : Nat)

def applyOp (s : State) (signer : Pubkey) : Operation → Option State
  | .«initialize» target_slot outcome => initializeTransition s signer target_slot outcome
  | .resolve_case_0 clock_slot => resolve_case_0Transition s signer clock_slot
  | .resolve_otherwise clock_slot => resolve_otherwiseTransition s signer clock_slot

/-- Property: returned_value_bounded. -/
def returned_value_bounded (s : State) : Prop :=
  s.last_returned = 0 ∨ s.last_returned = s.outcome_at_or_after

/-- Property: outcome_terminal_after_init. -/
def outcome_terminal_after_init (s : State) : Prop :=
  s.initialized = 0 ∨ (s.outcome_at_or_after ≥ 1 ∧ s.outcome_at_or_after ≤ 3)

end SlotHeightResolver
