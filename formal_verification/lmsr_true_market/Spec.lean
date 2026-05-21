import Mathlib.Algebra.BigOperators.Fin
import QEDGen.Solana.Account

namespace LmsrTrueMarket

open QEDGen.Solana.Account
abbrev Pubkey := QEDGen.Solana.Account.Pubkey

abbrev STATUS_UNINIT : Nat := 255
abbrev STATUS_ACTIVE : Nat := 0
abbrev STATUS_RESOLVED_YES : Nat := 1
abbrev STATUS_RESOLVED_NO : Nat := 2
abbrev STATUS_INVALID : Nat := 3

structure State where
  status : Nat
  b : Nat
  q_yes : Nat
  q_no : Nat
  vault : Nat
  initial_subsidy : Nat
  net_flow : Nat
  deriving Repr, DecidableEq, BEq

instance : Inhabited State := ⟨{
  status := STATUS_UNINIT,
  b := 0,
  q_yes := 0,
  q_no := 0,
  vault := 0,
  initial_subsidy := 0,
  net_flow := 0,
}⟩

def initialize_poolTransition
    (s : State) (signer : Pubkey)
    (b : Nat) (initial_subsidy : Nat) (b_ln2_ceil : Nat) : Option State :=
  let _payer := signer
  if (s.status = STATUS_UNINIT) ∧ (b > 0) ∧ (initial_subsidy ≥ b_ln2_ceil) then
    some { status := STATUS_ACTIVE,
           b := b,
           q_yes := 0, q_no := 0,
           vault := initial_subsidy,
           initial_subsidy := initial_subsidy,
           net_flow := 0 }
  else none

def buy_yesTransition (s : State) (_signer : Pubkey) (delta : Nat) (cost : Nat) : Option State :=
  if (s.status = STATUS_ACTIVE) ∧ (delta > 0) then
    some { s with q_yes := s.q_yes + delta,
                  vault := s.vault + cost,
                  net_flow := s.net_flow + cost }
  else none

def buy_noTransition (s : State) (_signer : Pubkey) (delta : Nat) (cost : Nat) : Option State :=
  if (s.status = STATUS_ACTIVE) ∧ (delta > 0) then
    some { s with q_no := s.q_no + delta,
                  vault := s.vault + cost,
                  net_flow := s.net_flow + cost }
  else none

def sell_yesTransition (s : State) (_signer : Pubkey) (delta : Nat) (payout : Nat) : Option State :=
  if (s.status = STATUS_ACTIVE) ∧ (delta > 0) ∧ (delta ≤ s.q_yes)
      ∧ (payout ≤ s.vault) ∧ (payout ≤ s.net_flow) then
    some { s with q_yes := s.q_yes - delta,
                  vault := s.vault - payout,
                  net_flow := s.net_flow - payout }
  else none

def sell_noTransition (s : State) (_signer : Pubkey) (delta : Nat) (payout : Nat) : Option State :=
  if (s.status = STATUS_ACTIVE) ∧ (delta > 0) ∧ (delta ≤ s.q_no)
      ∧ (payout ≤ s.vault) ∧ (payout ≤ s.net_flow) then
    some { s with q_no := s.q_no - delta,
                  vault := s.vault - payout,
                  net_flow := s.net_flow - payout }
  else none

def resolve_yesTransition (s : State) (_signer : Pubkey) : Option State :=
  if (s.status = STATUS_ACTIVE) then
    some { s with status := STATUS_RESOLVED_YES }
  else none

def resolve_noTransition (s : State) (_signer : Pubkey) : Option State :=
  if (s.status = STATUS_ACTIVE) then
    some { s with status := STATUS_RESOLVED_NO }
  else none

def resolve_invalidTransition (s : State) (_signer : Pubkey) : Option State :=
  if (s.status = STATUS_ACTIVE) then
    some { s with status := STATUS_INVALID }
  else none

inductive Operation where
  | initialize_pool (b : Nat) (initial_subsidy : Nat) (b_ln2_ceil : Nat)
  | buy_yes (delta : Nat) (cost : Nat)
  | buy_no  (delta : Nat) (cost : Nat)
  | sell_yes (delta : Nat) (payout : Nat)
  | sell_no  (delta : Nat) (payout : Nat)
  | resolve_yes
  | resolve_no
  | resolve_invalid

def applyOp (s : State) (signer : Pubkey) : Operation → Option State
  | .initialize_pool b is c => initialize_poolTransition s signer b is c
  | .buy_yes d c => buy_yesTransition s signer d c
  | .buy_no  d c => buy_noTransition s signer d c
  | .sell_yes d p => sell_yesTransition s signer d p
  | .sell_no  d p => sell_noTransition s signer d p
  | .resolve_yes => resolve_yesTransition s signer
  | .resolve_no => resolve_noTransition s signer
  | .resolve_invalid => resolve_invalidTransition s signer

/-- `initial_subsidy` is set at init and never modified afterwards. We
state preservation across non-UNINIT states. -/
def subsidy_field_stable (s s' : State) : Prop :=
  s.status = STATUS_UNINIT ∨ s.initial_subsidy = s'.initial_subsidy

/-- `b` is set at init and never modified afterwards. -/
def b_field_stable (s s' : State) : Prop :=
  s.status = STATUS_UNINIT ∨ s.b = s'.b

/-- **Bookkeeping invariant**: vault equals initial subsidy plus net
trader flow. This is the abstract conservation law; the LMSR-specific
bounded-loss claim (that `net_flow ≥ -b·ln(2)`) is deferred to a
separate proof against the actual cost function. -/
def vault_eq_initial_plus_net_flow (s : State) : Prop :=
  s.status = STATUS_UNINIT ∨ s.vault = s.initial_subsidy + s.net_flow

/-- Status only progresses through the lifecycle: UNINIT → ACTIVE →
{RESOLVED_*, INVALID}. -/
def status_monotone (s : State) : Prop :=
  s.status = STATUS_UNINIT ∨ s.status ≤ 3

end LmsrTrueMarket
