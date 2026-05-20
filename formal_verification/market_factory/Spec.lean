import QEDGen.Solana.Account

namespace MarketFactory

open QEDGen.Solana.Account
abbrev Pubkey := QEDGen.Solana.Account.Pubkey

structure State where
  initialized : Nat
  deadline_slot : Nat
  created_at_slot : Nat
  deriving Repr, DecidableEq, BEq

instance : Inhabited State := ⟨{
  initialized := 0,
  deadline_slot := 0,
  created_at_slot := 0,
}⟩

def registerTransition (s : State) (signer : Pubkey) (deadline_slot : Nat) (current_slot : Nat) : Option State :=
  if (s.initialized = 0) then
    some { s with initialized := 1, deadline_slot := deadline_slot, created_at_slot := current_slot }
  else none

inductive Operation where
  | register (deadline_slot : Nat) (current_slot : Nat)

def applyOp (s : State) (signer : Pubkey) : Operation → Option State
  | .register deadline_slot current_slot => registerTransition s signer deadline_slot current_slot

/-- Property: registration_terminal. -/
def registration_terminal (s : State) : Prop :=
  s.initialized = 0 ∨ s.initialized = 1

end MarketFactory
