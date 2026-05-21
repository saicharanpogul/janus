/-
Spec.lean — hand-written state machine for AccountLayer.

Models multi-user holdings of YES/NO outcome tokens as a fixed-size
`Map MAX_ACCOUNTS Account`, with conservation invariants between the
per-user balances and the cached `total_*_supply` fields.

This is the "if every user is a separate SPL Token account, the
program's `yes_supply` field equals the sum of all user balances"
invariant, formalised so Janus can compose against the SPL Token
program's semantics with confidence.

We skip the qedgen-generated Lean — its `Map[N]` lowering had several
known bugs in v2.15.1 (literal `Map[N]` syntax, duplicate `structure
State where`, sorry'd transition bodies). The state machine is small
enough that hand-writing it is faster than patching the codegen.
-/

import Mathlib.Algebra.BigOperators.Fin
import QEDGen.Solana.Account
import QEDGenMathlib.IndexedState

namespace AccountLayer

open QEDGen.Solana.Account
open QEDGen.Solana.IndexedState

abbrev Pubkey := QEDGen.Solana.Account.Pubkey
abbrev MAX_ACCOUNTS : Nat := 8
abbrev AccountIdx : Type := Fin MAX_ACCOUNTS

structure Account where
  yes_balance : Nat
  no_balance  : Nat
  deriving Repr, DecidableEq

instance : Inhabited Account := ⟨{ yes_balance := 0, no_balance := 0 }⟩

structure State where
  total_yes_supply : Nat
  total_no_supply  : Nat
  accounts         : Map MAX_ACCOUNTS Account
  deriving Inhabited

-- ----------------------------------------------------------------------
-- Operations
-- ----------------------------------------------------------------------

def mint_yesTransition (s : State) (_signer : Pubkey)
    (i : AccountIdx) (amount : Nat) : Option State :=
  some {
    s with
      accounts := Map.set s.accounts i
        { (s.accounts i) with yes_balance := (s.accounts i).yes_balance + amount },
      total_yes_supply := s.total_yes_supply + amount
  }

def mint_noTransition (s : State) (_signer : Pubkey)
    (i : AccountIdx) (amount : Nat) : Option State :=
  some {
    s with
      accounts := Map.set s.accounts i
        { (s.accounts i) with no_balance := (s.accounts i).no_balance + amount },
      total_no_supply := s.total_no_supply + amount
  }

def burn_yesTransition (s : State) (_signer : Pubkey)
    (i : AccountIdx) (amount : Nat) : Option State :=
  if (s.accounts i).yes_balance ≥ amount ∧ s.total_yes_supply ≥ amount then
    some {
      s with
        accounts := Map.set s.accounts i
          { (s.accounts i) with yes_balance := (s.accounts i).yes_balance - amount },
        total_yes_supply := s.total_yes_supply - amount
    }
  else none

def burn_noTransition (s : State) (_signer : Pubkey)
    (i : AccountIdx) (amount : Nat) : Option State :=
  if (s.accounts i).no_balance ≥ amount ∧ s.total_no_supply ≥ amount then
    some {
      s with
        accounts := Map.set s.accounts i
          { (s.accounts i) with no_balance := (s.accounts i).no_balance - amount },
        total_no_supply := s.total_no_supply - amount
    }
  else none

def transfer_yesTransition (s : State) (_signer : Pubkey)
    (from_idx to_idx : AccountIdx) (amount : Nat) : Option State :=
  if (s.accounts from_idx).yes_balance ≥ amount then
    let after_from := Map.set s.accounts from_idx
      { (s.accounts from_idx) with yes_balance := (s.accounts from_idx).yes_balance - amount }
    let after_to := Map.set after_from to_idx
      { (after_from to_idx) with yes_balance := (after_from to_idx).yes_balance + amount }
    some { s with accounts := after_to }
  else none

def transfer_noTransition (s : State) (_signer : Pubkey)
    (from_idx to_idx : AccountIdx) (amount : Nat) : Option State :=
  if (s.accounts from_idx).no_balance ≥ amount then
    let after_from := Map.set s.accounts from_idx
      { (s.accounts from_idx) with no_balance := (s.accounts from_idx).no_balance - amount }
    let after_to := Map.set after_from to_idx
      { (after_from to_idx) with no_balance := (after_from to_idx).no_balance + amount }
    some { s with accounts := after_to }
  else none

-- ----------------------------------------------------------------------
-- Properties
-- ----------------------------------------------------------------------

/-- The total YES supply equals the sum of every user's YES balance. -/
def yes_supply_matches_sum (s : State) : Prop :=
  s.total_yes_supply = ∑ i : AccountIdx, (s.accounts i).yes_balance

/-- The total NO supply equals the sum of every user's NO balance. -/
def no_supply_matches_sum (s : State) : Prop :=
  s.total_no_supply = ∑ i : AccountIdx, (s.accounts i).no_balance

end AccountLayer
