import QEDGen.Solana.Account

namespace PythPriceResolver

open QEDGen.Solana.Account
abbrev Pubkey := QEDGen.Solana.Account.Pubkey

abbrev COMPARISON_GTE : Nat := 0
abbrev COMPARISON_LT : Nat := 1

structure State where
  initialized : Nat
  earliest_slot : Nat
  threshold_price : Nat
  threshold_expo : Nat
  comparison : Nat
  last_returned : Nat
  deriving Repr, DecidableEq, BEq

instance : Inhabited State := ⟨{
  initialized := 0,
  earliest_slot := 0,
  threshold_price := 0,
  threshold_expo := 0,
  comparison := 0,
  last_returned := 0,
}⟩

def initializeTransition (s : State) (signer : Pubkey) (earliest_slot : Nat) (threshold_price : Nat) (threshold_expo : Nat) (comparison : Nat) : Option State :=
  if (s.initialized = 0) ∧ (comparison ≤ 1) then
    some { s with initialized := 1, earliest_slot := earliest_slot, threshold_price := threshold_price, threshold_expo := threshold_expo, comparison := comparison }
  else none

def resolve_case_0Transition (s : State) (signer : Pubkey) (clock_slot : Nat) (feed_price : Nat) (feed_expo : Nat) : Option State :=
  if (clock_slot < s.earliest_slot) then
    some { s with last_returned := 0 }
  else none

def resolve_case_1Transition (s : State) (signer : Pubkey) (clock_slot : Nat) (feed_price : Nat) (feed_expo : Nat) : Option State :=
  if (¬(clock_slot < s.earliest_slot)) ∧ (feed_expo ≠ s.threshold_expo) then
    some { s with last_returned := 3 }
  else none

def resolve_case_2Transition (s : State) (signer : Pubkey) (clock_slot : Nat) (feed_price : Nat) (feed_expo : Nat) : Option State :=
  if (¬(clock_slot < s.earliest_slot)) ∧ (¬(feed_expo ≠ s.threshold_expo)) ∧ (s.comparison = 0 ∧ feed_price ≥ s.threshold_price) then
    some { s with last_returned := 1 }
  else none

def resolve_case_3Transition (s : State) (signer : Pubkey) (clock_slot : Nat) (feed_price : Nat) (feed_expo : Nat) : Option State :=
  if (¬(clock_slot < s.earliest_slot)) ∧ (¬(feed_expo ≠ s.threshold_expo)) ∧ (¬(s.comparison = 0 ∧ feed_price ≥ s.threshold_price)) ∧ (s.comparison = 1 ∧ feed_price < s.threshold_price) then
    some { s with last_returned := 1 }
  else none

def resolve_otherwiseTransition (s : State) (signer : Pubkey) (clock_slot : Nat) (feed_price : Nat) (feed_expo : Nat) : Option State :=
  if (¬(clock_slot < s.earliest_slot)) ∧ (¬(feed_expo ≠ s.threshold_expo)) ∧ (¬(s.comparison = 0 ∧ feed_price ≥ s.threshold_price)) ∧ (¬(s.comparison = 1 ∧ feed_price < s.threshold_price)) then
    some { s with last_returned := 2 }
  else none

inductive Operation where
  | «initialize» (earliest_slot : Nat) (threshold_price : Nat) (threshold_expo : Nat) (comparison : Nat)
  | resolve_case_0 (clock_slot : Nat) (feed_price : Nat) (feed_expo : Nat)
  | resolve_case_1 (clock_slot : Nat) (feed_price : Nat) (feed_expo : Nat)
  | resolve_case_2 (clock_slot : Nat) (feed_price : Nat) (feed_expo : Nat)
  | resolve_case_3 (clock_slot : Nat) (feed_price : Nat) (feed_expo : Nat)
  | resolve_otherwise (clock_slot : Nat) (feed_price : Nat) (feed_expo : Nat)

def applyOp (s : State) (signer : Pubkey) : Operation → Option State
  | .«initialize» earliest_slot threshold_price threshold_expo comparison => initializeTransition s signer earliest_slot threshold_price threshold_expo comparison
  | .resolve_case_0 clock_slot feed_price feed_expo => resolve_case_0Transition s signer clock_slot feed_price feed_expo
  | .resolve_case_1 clock_slot feed_price feed_expo => resolve_case_1Transition s signer clock_slot feed_price feed_expo
  | .resolve_case_2 clock_slot feed_price feed_expo => resolve_case_2Transition s signer clock_slot feed_price feed_expo
  | .resolve_case_3 clock_slot feed_price feed_expo => resolve_case_3Transition s signer clock_slot feed_price feed_expo
  | .resolve_otherwise clock_slot feed_price feed_expo => resolve_otherwiseTransition s signer clock_slot feed_price feed_expo

/-- Property: outcome_in_canonical_range. -/
def outcome_in_canonical_range (s : State) : Prop :=
  s.last_returned ≤ 3

/-- Property: comparison_bounded. -/
def comparison_bounded (s : State) : Prop :=
  s.initialized = 0 ∨ (s.comparison = 0 ∨ s.comparison = 1)

end PythPriceResolver
