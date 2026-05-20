import Mathlib.Algebra.BigOperators.Fin
import QEDGen.Solana.Account
import QEDGenMathlib.IndexedState

namespace LmsrMarket

open QEDGen.Solana
open QEDGen.Solana.IndexedState

abbrev MAX_FEE_BPS : Nat := 1000

abbrev AccountIdx : Type := Fin MAX_FEE_BPS

structure State where
  initialized : Nat
  status : Nat
  yes_reserves : Nat
  no_reserves : Nat
  fee_bps : Nat
  deriving Repr, DecidableEq, BEq

instance : Inhabited State := ⟨{
  initialized := 0,
  status := 0,
  yes_reserves := 0,
  no_reserves := 0,
  fee_bps := 0,
}⟩

structure State where

def initialize_poolTransition (s : State) (signer : Pubkey) (subsidy_yes : Nat) (subsidy_no : Nat) (fee_bps : Nat) : Option State :=
  let payer := signer
  if (s.initialized = 0) ∧ (subsidy_yes > 0) ∧ (subsidy_no > 0) ∧ (fee_bps ≤ 1000) then
    some { s with initialized := 1, status := 0, yes_reserves := subsidy_yes, no_reserves := subsidy_no, fee_bps := fee_bps }
  else none

def swap_yes_for_noTransition (s : State) (signer : Pubkey) (amount_in : Nat) (amount_out : Nat) (min_amount_out : Nat) : Option State :=
  if (s.status = 0) ∧ (amount_in > 0) ∧ (amount_out ≥ min_amount_out) ∧ (amount_out < s.no_reserves) then
    some { s with yes_reserves := s.yes_reserves + amount_in, no_reserves := s.no_reserves - amount_out }
  else none

def swap_no_for_yesTransition (s : State) (signer : Pubkey) (amount_in : Nat) (amount_out : Nat) (min_amount_out : Nat) : Option State :=
  if (s.status = 0) ∧ (amount_in > 0) ∧ (amount_out ≥ min_amount_out) ∧ (amount_out < s.yes_reserves) then
    some { s with no_reserves := s.no_reserves + amount_in, yes_reserves := s.yes_reserves - amount_out }
  else none

def mark_resolved_yesTransition (s : State) (signer : Pubkey) : Option State :=
  if (s.status = 0) then
    some { s with status := 1 }
  else none

def mark_resolved_noTransition (s : State) (signer : Pubkey) : Option State :=
  if (s.status = 0) then
    some { s with status := 2 }
  else none

def withdraw_yes_after_resolutionTransition (s : State) (signer : Pubkey) (amount : Nat) : Option State :=
  let payer := signer
  if (s.status = 1) ∧ (amount > 0) ∧ (amount ≤ s.yes_reserves) then
    some { s with yes_reserves := s.yes_reserves - amount }
  else none

def withdraw_no_after_resolutionTransition (s : State) (signer : Pubkey) (amount : Nat) : Option State :=
  let payer := signer
  if (s.status = 2) ∧ (amount > 0) ∧ (amount ≤ s.no_reserves) then
    some { s with no_reserves := s.no_reserves - amount }
  else none

inductive Operation where
  | initialize_pool (subsidy_yes : Nat) (subsidy_no : Nat) (fee_bps : Nat)
  | swap_yes_for_no (amount_in : Nat) (amount_out : Nat) (min_amount_out : Nat)
  | swap_no_for_yes (amount_in : Nat) (amount_out : Nat) (min_amount_out : Nat)
  | mark_resolved_yes
  | mark_resolved_no
  | withdraw_yes_after_resolution (amount : Nat)
  | withdraw_no_after_resolution (amount : Nat)

def applyOp (s : State) (signer : Pubkey) : Operation → Option State
  | .initialize_pool subsidy_yes subsidy_no fee_bps => initialize_poolTransition s signer subsidy_yes subsidy_no fee_bps
  | .swap_yes_for_no amount_in amount_out min_amount_out => swap_yes_for_noTransition s signer amount_in amount_out min_amount_out
  | .swap_no_for_yes amount_in amount_out min_amount_out => swap_no_for_yesTransition s signer amount_in amount_out min_amount_out
  | .mark_resolved_yes => mark_resolved_yesTransition s signer
  | .mark_resolved_no => mark_resolved_noTransition s signer
  | .withdraw_yes_after_resolution amount => withdraw_yes_after_resolutionTransition s signer amount
  | .withdraw_no_after_resolution amount => withdraw_no_after_resolutionTransition s signer amount

/-- Property: swap_no_drain. -/
def swap_no_drain (s : State) : Prop :=
  s.status ≠ 0 ∨ s.yes_reserves > 0 ∨ s.no_reserves > 0

/-- Property: fee_bps_bounded. -/
def fee_bps_bounded (s : State) : Prop :=
  s.fee_bps ≤ 1000

/-- Property: status_monotone. -/
def status_monotone (s : State) : Prop :=
  s.status ≤ 2

end LmsrMarket
