import { PublicKey } from "@solana/web3.js";

// ---- Program IDs (placeholders; replace before deployment) ----
//
// These mirror the `declare_id!` macros in each program's lib.rs. Keep them
// in sync when you generate real keypairs with `solana-keygen new`.

export const CONDITIONAL_TOKENS_PROGRAM_ID = new PublicKey(
  "61MLdp3EEExnhh6W9BYT8Jj52ZoXYoQn6PHKmxtrsc7y",
);

export const LMSR_MARKET_PROGRAM_ID = new PublicKey(
  "61MLdp3QZMZ8knfBtZvi7CGhB2SgX1Usq6wVaL89SHD1",
);

export const SLOT_HEIGHT_RESOLVER_PROGRAM_ID = new PublicKey(
  "61MLdp3RWCNin5N3CPGTdCHoSA4EyZYLqegkaDus9nFZ",
);

export const PYTH_PRICE_RESOLVER_PROGRAM_ID = new PublicKey(
  "61MLdp3R75WtTNjW4MfvbCDU43uAB2uUNZhD53kX9hyq",
);

export const MARKET_FACTORY_PROGRAM_ID = new PublicKey(
  "61MLdp3PjxAhoiPRbx2kJqoq4cri8vB5FovmHYq7AKuh",
);

// ---- PDA seed constants (must match Rust) ----

export const MARKET_SEED = "market";
export const YES_MINT_SEED = "yes";
export const NO_MINT_SEED = "no";
export const VAULT_SEED = "vault";

export const POOL_SEED = "pool";
export const POOL_YES_VAULT_SEED = "yes-vault";
export const POOL_NO_VAULT_SEED = "no-vault";

export const SLOT_RESOLVER_SEED = "slot-resolver";
export const PYTH_RESOLVER_SEED = "pyth-resolver";

export const REGISTRATION_SEED = "registration";

// ---- Instruction discriminators (must match Rust `InstructionTag`) ----

export const CONDITIONAL_TOKENS_IX = {
  InitializeMarket: 0,
  Split: 1,
  Merge: 2,
  Redeem: 3,
  Resolve: 4,
} as const;

export const LMSR_MARKET_IX = {
  InitializePool: 0,
  Swap: 1,
  WithdrawPoolTokens: 2,
} as const;

export const RESOLVER_IX = {
  /** Standardized across all resolver programs. */
  Resolve: 0,
  Initialize: 1,
} as const;

export const MARKET_FACTORY_IX = {
  Register: 0,
} as const;

/** Resolution outcome bytes returned by resolvers via set_return_data. */
export const RESOLUTION_OUTCOME = {
  Unresolved: 0,
  Yes: 1,
  No: 2,
  Invalid: 3,
} as const;
