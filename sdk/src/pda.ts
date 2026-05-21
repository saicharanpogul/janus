import { PublicKey } from "@solana/web3.js";

import {
  CONDITIONAL_TOKENS_PROGRAM_ID,
  LMSR_MARKET_PROGRAM_ID,
  LMSR_TRUE_MARKET_PROGRAM_ID,
  MARKET_FACTORY_PROGRAM_ID,
  MARKET_SEED,
  NO_MINT_SEED,
  POOL_NO_VAULT_SEED,
  POOL_SEED,
  POOL_YES_VAULT_SEED,
  PYTH_RESOLVER_SEED,
  PYTH_PRICE_RESOLVER_PROGRAM_ID,
  REGISTRATION_SEED,
  SLOT_HEIGHT_RESOLVER_PROGRAM_ID,
  SLOT_RESOLVER_SEED,
  TRUE_POOL_COLLATERAL_VAULT_SEED,
  VAULT_SEED,
  YES_MINT_SEED,
} from "./constants.js";

const enc = (s: string) => new TextEncoder().encode(s);

const u64le = (n: bigint): Uint8Array => {
  const buf = new Uint8Array(8);
  const view = new DataView(buf.buffer);
  view.setBigUint64(0, n, true);
  return buf;
};

// ---- Conditional-tokens PDAs ----

export function deriveMarketPda(
  collateralMint: PublicKey,
  resolverState: PublicKey,
  deadlineSlot: bigint,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      enc(MARKET_SEED),
      collateralMint.toBytes(),
      resolverState.toBytes(),
      u64le(deadlineSlot),
    ],
    CONDITIONAL_TOKENS_PROGRAM_ID,
  );
}

export function deriveYesMintPda(market: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(YES_MINT_SEED), market.toBytes()],
    CONDITIONAL_TOKENS_PROGRAM_ID,
  );
}

export function deriveNoMintPda(market: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(NO_MINT_SEED), market.toBytes()],
    CONDITIONAL_TOKENS_PROGRAM_ID,
  );
}

export function deriveVaultPda(market: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(VAULT_SEED), market.toBytes()],
    CONDITIONAL_TOKENS_PROGRAM_ID,
  );
}

// ---- LMSR-market PDAs ----

export function derivePoolPda(market: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(POOL_SEED), market.toBytes()],
    LMSR_MARKET_PROGRAM_ID,
  );
}

export function derivePoolYesVaultPda(
  pool: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(POOL_YES_VAULT_SEED), pool.toBytes()],
    LMSR_MARKET_PROGRAM_ID,
  );
}

export function derivePoolNoVaultPda(
  pool: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(POOL_NO_VAULT_SEED), pool.toBytes()],
    LMSR_MARKET_PROGRAM_ID,
  );
}

// ---- True-LMSR PDAs ----
//
// The true-LMSR pool is keyed by the resolver state (so the same
// collateral + resolver pair gives a deterministic pool address)
// and the collateral vault is keyed by the pool.

export function deriveTruePoolPda(
  resolverState: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(POOL_SEED), resolverState.toBytes()],
    LMSR_TRUE_MARKET_PROGRAM_ID,
  );
}

export function deriveTruePoolCollateralVaultPda(
  pool: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(TRUE_POOL_COLLATERAL_VAULT_SEED), pool.toBytes()],
    LMSR_TRUE_MARKET_PROGRAM_ID,
  );
}

// ---- Resolver PDAs ----

/**
 * Slot-height resolver state PDA.
 *
 * `seedKey` is any caller-chosen 32-byte tag (often the predicted market
 * pubkey, so one authority can run many independent resolver instances).
 */
export function deriveSlotResolverStatePda(
  seedKey: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(SLOT_RESOLVER_SEED), seedKey.toBytes()],
    SLOT_HEIGHT_RESOLVER_PROGRAM_ID,
  );
}

export function derivePythResolverStatePda(
  seedKey: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(PYTH_RESOLVER_SEED), seedKey.toBytes()],
    PYTH_PRICE_RESOLVER_PROGRAM_ID,
  );
}

// ---- Market-factory PDA ----

export function deriveRegistrationPda(market: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [enc(REGISTRATION_SEED), market.toBytes()],
    MARKET_FACTORY_PROGRAM_ID,
  );
}
