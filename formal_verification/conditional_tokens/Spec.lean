import Mathlib.Algebra.BigOperators.Fin
import QEDGen.Solana.Account
import QEDGenMathlib.IndexedState

namespace ConditionalTokens

open QEDGen.Solana
open QEDGen.Solana.IndexedState

abbrev STATUS_ACTIVE : Nat := 0
abbrev STATUS_RESOLVED_YES : Nat := 1
abbrev STATUS_RESOLVED_NO : Nat := 2
abbrev STATUS_RESOLVED_INVALID : Nat := 3

abbrev AccountIdx : Type := Fin 1024

structure State where
  initialized : Nat
  status : Nat
  collateral_balance : Nat
  yes_supply : Nat
  no_supply : Nat
  vault : Nat
  deriving Repr, DecidableEq, BEq

instance : Inhabited State := ⟨{
  initialized := 0,
  status := 0,
  collateral_balance := 0,
  yes_supply := 0,
  no_supply := 0,
  vault := 0,
}⟩

structure State where

def initialize_marketTransition (s : State) (signer : Pubkey) (initial_collateral : Nat) : Option State :=
  if (s.initialized = 0) then
    some { s with initialized := 1, status := 0, collateral_balance := initial_collateral, yes_supply := 0, no_supply := 0, vault := 0 }
  else none

def splitTransition (s : State) (signer : Pubkey) (amount : Nat) : Option State :=
  if (s.initialized = 1) ∧ (s.status = 0) ∧ (amount > 0) ∧ (s.collateral_balance ≥ amount) then
    some { s with collateral_balance := s.collateral_balance - amount, vault := s.vault + amount, yes_supply := s.yes_supply + amount, no_supply := s.no_supply + amount }
  else none

def mergeTransition (s : State) (signer : Pubkey) (amount : Nat) : Option State :=
  if (s.initialized = 1) ∧ (s.status = 0) ∧ (amount > 0) ∧ (s.yes_supply ≥ amount) ∧ (s.no_supply ≥ amount) ∧ (s.vault ≥ amount) then
    some { s with yes_supply := s.yes_supply - amount, no_supply := s.no_supply - amount, vault := s.vault - amount, collateral_balance := s.collateral_balance + amount }
  else none

def resolve_yesTransition (s : State) (signer : Pubkey) : Option State :=
  if (s.initialized = 1) ∧ (s.status = 0) then
    some { s with status := 1 }
  else none

def resolve_noTransition (s : State) (signer : Pubkey) : Option State :=
  if (s.initialized = 1) ∧ (s.status = 0) then
    some { s with status := 2 }
  else none

def redeem_yesTransition (s : State) (signer : Pubkey) (amount : Nat) : Option State :=
  if (s.status = 1) ∧ (amount > 0) ∧ (s.yes_supply ≥ amount) ∧ (s.vault ≥ amount) then
    some { s with yes_supply := s.yes_supply - amount, vault := s.vault - amount, collateral_balance := s.collateral_balance + amount }
  else none

def redeem_noTransition (s : State) (signer : Pubkey) (amount : Nat) : Option State :=
  if (s.status = 2) ∧ (amount > 0) ∧ (s.no_supply ≥ amount) ∧ (s.vault ≥ amount) then
    some { s with no_supply := s.no_supply - amount, vault := s.vault - amount, collateral_balance := s.collateral_balance + amount }
  else none

inductive Operation where
  | initialize_market (initial_collateral : Nat)
  | split (amount : Nat)
  | merge (amount : Nat)
  | resolve_yes
  | resolve_no
  | redeem_yes (amount : Nat)
  | redeem_no (amount : Nat)

def applyOp (s : State) (signer : Pubkey) : Operation → Option State
  | .initialize_market initial_collateral => initialize_marketTransition s signer initial_collateral
  | .split amount => splitTransition s signer amount
  | .merge amount => mergeTransition s signer amount
  | .resolve_yes => resolve_yesTransition s signer
  | .resolve_no => resolve_noTransition s signer
  | .redeem_yes amount => redeem_yesTransition s signer amount
  | .redeem_no amount => redeem_noTransition s signer amount

/-- Property: yes_no_paired_while_active. -/
def yes_no_paired_while_active (s : State) : Prop :=
  s.status ≠ 0 ∨ s.yes_supply = s.no_supply

/-- Property: vault_tracks_yes. -/
def vault_tracks_yes (s : State) : Prop :=
  s.status = 2 ∨ s.vault = s.yes_supply

/-- Property: vault_tracks_no. -/
def vault_tracks_no (s : State) : Prop :=
  s.status = 1 ∨ s.vault = s.no_supply

/-- Property: status_monotone. -/
def status_monotone (s : State) : Prop :=
  s.status ≤ 3

end ConditionalTokens
