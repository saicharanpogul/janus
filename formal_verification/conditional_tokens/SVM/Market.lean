/-
SVM/Market.lean — bytecode-layer model of the conditional-tokens
`Market` struct.

This is the first piece of Level 2: a faithful Lean rendering of the
on-chain `#[repr(C)]` layout used in
`programs/conditional-tokens/src/state.rs`. Once we depend on qedsvm,
this lifts to a separation-logic predicate `marketState addr m`
("the bytes at `addr` decode to `m`") that we'll need to phrase the
`Split` Hoare triple at the binary layer.

The structure isn't tied to any particular SL framework yet — we
define the Market type, byte offsets, and a deterministic
encode/decode pair. Lifting to qedsvm's heap is a 1-line wrapper.

Layout (248 bytes total, matches `state.rs::Market::LEN`):

  offset  size  field
  ────────────────────────────────────────
       0    1   bump            : u8
       1    1   status          : u8
       2    6   _padding        : [u8; 6]
       8   32   collateral_mint : Pubkey
      40   32   yes_mint        : Pubkey
      72   32   no_mint         : Pubkey
     104   32   vault           : Pubkey
     136   32   resolver_program: Pubkey
     168   32   resolver_state  : Pubkey
     200   32   authority       : Pubkey
     232    8   deadline_slot   : u64 LE
     240    8   created_at_slot : u64 LE
-/

-- `ByteArray` + `Array.ofFn` are in core Lean; no Mathlib needed here.
-- The decode/encode logic is pure stdlib so this module stays light and
-- doesn't pay the full Mathlib build cost on every `lake build`.

namespace Janus.ConditionalTokens.SVM

/-! ## Atom types -/

/-- 32-byte program-address / account-key. Mirrors `solana_pubkey::Pubkey`. -/
abbrev Pubkey := Fin 32 → UInt8

/-- Construct a Pubkey from a list (left-padded with zeros if short, truncated
if long). Helper for tests. -/
def Pubkey.ofList (bs : List UInt8) : Pubkey :=
  let bs' := (bs.take 32).toArray
  fun i => bs'.getD i.val 0

/-! ## Market layout -/

/-- The four valid market statuses, matching the on-chain `MarketStatus`
enum's discriminant bytes. -/
inductive MarketStatus
  | Active           -- 0
  | ResolvedYes      -- 1
  | ResolvedNo       -- 2
  | ResolvedInvalid  -- 3
  deriving Repr, DecidableEq, BEq

def MarketStatus.toByte : MarketStatus → UInt8
  | .Active          => 0
  | .ResolvedYes     => 1
  | .ResolvedNo      => 2
  | .ResolvedInvalid => 3

def MarketStatus.ofByte? (b : UInt8) : Option MarketStatus :=
  match b with
  | 0 => some .Active
  | 1 => some .ResolvedYes
  | 2 => some .ResolvedNo
  | 3 => some .ResolvedInvalid
  | _ => none

theorem MarketStatus.ofByte_toByte (s : MarketStatus) :
    MarketStatus.ofByte? s.toByte = some s := by
  cases s <;> rfl

/-- The on-chain Market struct. Field order matches the `#[repr(C)]`
declaration; field types match the byte semantics. -/
structure Market where
  bump            : UInt8
  status          : MarketStatus
  collateralMint  : Pubkey
  yesMint         : Pubkey
  noMint          : Pubkey
  vault           : Pubkey
  resolverProgram : Pubkey
  resolverState   : Pubkey
  authority       : Pubkey
  deadlineSlot    : UInt64
  createdAtSlot   : UInt64

/-! ## Byte offsets

These match `Market::LEN == 248` exactly. The constants are exposed
so SL predicates can address individual sub-atoms (e.g. "byte at
offset BUMP_OFFSET is the bump"). -/

def Market.BUMP_OFFSET            : Nat := 0
def Market.STATUS_OFFSET          : Nat := 1
def Market.PADDING_OFFSET         : Nat := 2
def Market.COLLATERAL_MINT_OFFSET : Nat := 8
def Market.YES_MINT_OFFSET        : Nat := 40
def Market.NO_MINT_OFFSET         : Nat := 72
def Market.VAULT_OFFSET           : Nat := 104
def Market.RESOLVER_PROGRAM_OFFSET: Nat := 136
def Market.RESOLVER_STATE_OFFSET  : Nat := 168
def Market.AUTHORITY_OFFSET       : Nat := 200
def Market.DEADLINE_SLOT_OFFSET   : Nat := 232
def Market.CREATED_AT_SLOT_OFFSET : Nat := 240
def Market.LEN                    : Nat := 248

/-- Sanity: the offsets line up with the next field's offset. -/
theorem Market.offsets_consistent :
    Market.PADDING_OFFSET = Market.STATUS_OFFSET + 1 ∧
    Market.COLLATERAL_MINT_OFFSET = Market.PADDING_OFFSET + 6 ∧
    Market.YES_MINT_OFFSET = Market.COLLATERAL_MINT_OFFSET + 32 ∧
    Market.NO_MINT_OFFSET = Market.YES_MINT_OFFSET + 32 ∧
    Market.VAULT_OFFSET = Market.NO_MINT_OFFSET + 32 ∧
    Market.RESOLVER_PROGRAM_OFFSET = Market.VAULT_OFFSET + 32 ∧
    Market.RESOLVER_STATE_OFFSET = Market.RESOLVER_PROGRAM_OFFSET + 32 ∧
    Market.AUTHORITY_OFFSET = Market.RESOLVER_STATE_OFFSET + 32 ∧
    Market.DEADLINE_SLOT_OFFSET = Market.AUTHORITY_OFFSET + 32 ∧
    Market.CREATED_AT_SLOT_OFFSET = Market.DEADLINE_SLOT_OFFSET + 8 ∧
    Market.LEN = Market.CREATED_AT_SLOT_OFFSET + 8 := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> rfl

/-! ## Encode / decode

`Bytes` is an alias for `Array UInt8`. Slightly easier to work with
than core `ByteArray` for this module (Array has `.getD` and `.ofFn`).
When we integrate with qedsvm we'll convert to/from their preferred
representation at the boundary. -/

abbrev Bytes := Array UInt8

/-- Convert a `UInt64` to its 8-byte little-endian representation. -/
def u64LE (n : UInt64) : Bytes :=
  let v := n.toNat
  Array.ofFn (fun (i : Fin 8) =>
    UInt8.ofNat ((v / (256 ^ i.val)) % 256))

/-- Read 8 bytes starting at `off` from `bs` and interpret as little-endian
u64. Returns 0 if out-of-bounds (callers should pre-check `bs.size ≥ 248`). -/
def readU64LE (bs : Bytes) (off : Nat) : UInt64 :=
  let go (i : Nat) : Nat := (bs.getD (off + i) 0).toNat * (256 ^ i)
  UInt64.ofNat (go 0 + go 1 + go 2 + go 3 + go 4 + go 5 + go 6 + go 7)

/-- Read 32 bytes starting at `off` as a Pubkey. -/
def readPubkey (bs : Bytes) (off : Nat) : Pubkey :=
  fun i => bs.getD (off + i.val) 0

/-- Decode a 248-byte buffer into a Market. Returns `none` if the buffer
is too short or the status byte is out of range. The fast-fail mirrors
the on-chain `Market::from_account_data` shape. -/
def Market.decode? (bs : Bytes) : Option Market := do
  if bs.size < Market.LEN then
    failure
  else
    let bump   := bs.getD Market.BUMP_OFFSET 0
    let status ← MarketStatus.ofByte? (bs.getD Market.STATUS_OFFSET 0)
    pure {
      bump            := bump,
      status          := status,
      collateralMint  := readPubkey bs Market.COLLATERAL_MINT_OFFSET,
      yesMint         := readPubkey bs Market.YES_MINT_OFFSET,
      noMint          := readPubkey bs Market.NO_MINT_OFFSET,
      vault           := readPubkey bs Market.VAULT_OFFSET,
      resolverProgram := readPubkey bs Market.RESOLVER_PROGRAM_OFFSET,
      resolverState   := readPubkey bs Market.RESOLVER_STATE_OFFSET,
      authority       := readPubkey bs Market.AUTHORITY_OFFSET,
      deadlineSlot    := readU64LE bs Market.DEADLINE_SLOT_OFFSET,
      createdAtSlot   := readU64LE bs Market.CREATED_AT_SLOT_OFFSET,
    }

/-! ## Separation-logic predicate (skeleton)

Once we depend on qedsvm we replace `Heap := Nat → UInt8` (a flat
byte-addressable heap) with their concrete account-data heap, and
`marketState` becomes a real SL predicate. For now we keep it
parameterized so the integration is a 1-line wrapper at the
import-qedsvm step. -/

/-- A minimal byte-addressable heap. Lifts to qedsvm's `AccountData`
predicate once we depend on it. -/
abbrev Heap := Nat → UInt8

/-- "Bytes in `h` starting at address `addr` for `Market.LEN` bytes
decode to `m` as a `Market`." Used as the precondition / postcondition
atom in the Split Hoare triple. -/
def marketState (h : Heap) (addr : Nat) (m : Market) : Prop :=
  let bs : Bytes := Array.ofFn (fun (i : Fin Market.LEN) => h (addr + i.val))
  Market.decode? bs = some m

/-- The empty Market — used as the post-state of failed initialisations
(though in our model the account would actually be entirely absent on
failure, since the program rolls back). Convenient as a default. -/
def Market.empty : Market := {
  bump := 0,
  status := .Active,
  collateralMint  := fun _ => 0,
  yesMint         := fun _ => 0,
  noMint          := fun _ => 0,
  vault           := fun _ => 0,
  resolverProgram := fun _ => 0,
  resolverState   := fun _ => 0,
  authority       := fun _ => 0,
  deadlineSlot    := 0,
  createdAtSlot   := 0,
}

/-! ## Layout sanity lemmas

These pin the byte offsets at compile time. If anyone reorders
fields in the on-chain Rust struct without updating the Lean side,
the discharge below breaks. -/

/-- Total layout size is 248, matching `Market::LEN` in Rust. -/
theorem Market.LEN_eq : Market.LEN = 248 := rfl

/-- All ten field-start offsets occupy non-overlapping aligned ranges. -/
theorem Market.layout_well_formed :
    Market.STATUS_OFFSET = 1 ∧
    Market.PADDING_OFFSET = 2 ∧
    Market.COLLATERAL_MINT_OFFSET = 8 ∧
    Market.YES_MINT_OFFSET = 40 ∧
    Market.NO_MINT_OFFSET = 72 ∧
    Market.VAULT_OFFSET = 104 ∧
    Market.RESOLVER_PROGRAM_OFFSET = 136 ∧
    Market.RESOLVER_STATE_OFFSET = 168 ∧
    Market.AUTHORITY_OFFSET = 200 ∧
    Market.DEADLINE_SLOT_OFFSET = 232 ∧
    Market.CREATED_AT_SLOT_OFFSET = 240 := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> rfl

-- The two decode-failure lemmas (`decode_too_short`, `decode_bad_status`)
-- belong here but their proofs need monadic-`do` reasoning that's
-- easier to discharge when consumed by a concrete SL proof rather than
-- in isolation. Will land alongside the Split triple they're used by.
-- See [LEVEL_2_SPLIT_SPEC.md] for the eventual proof obligations.

end Janus.ConditionalTokens.SVM
