import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";

import {
  PYTH_PRICE_RESOLVER_PROGRAM_ID,
  RESOLVER_IX,
  SLOT_HEIGHT_RESOLVER_PROGRAM_ID,
} from "../constants.js";
import {
  derivePythResolverStatePda,
  deriveSlotResolverStatePda,
} from "../pda.js";

// ============================================================
// Slot-height resolver
// ============================================================

export interface SlotHeightResolverInitParams {
  payer: PublicKey;
  authority: PublicKey;
  /** Any caller-chosen 32-byte tag (typically the predicted market pubkey). */
  seedKey: PublicKey;
  /** ResolutionOutcome to return at/after the target slot: 1 (Yes), 2 (No), 3 (Invalid). */
  outcomeAtOrAfter: 1 | 2 | 3;
  targetSlot: bigint;
}

export interface SlotHeightResolverInitResult {
  ix: TransactionInstruction;
  state: PublicKey;
}

export function initializeSlotHeightResolverIx(
  params: SlotHeightResolverInitParams,
): SlotHeightResolverInitResult {
  const [state, bump] = deriveSlotResolverStatePda(params.seedKey);

  // data: tag(1) + [outcome:u8, bump:u8, pad:6, target_slot:u64, seed_key:32] (48 bytes)
  const data = new Uint8Array(1 + 48);
  data[0] = RESOLVER_IX.Initialize;
  data[1] = params.outcomeAtOrAfter;
  data[2] = bump;
  // bytes [3..9] padding
  const view = new DataView(data.buffer, data.byteOffset + 9, 8);
  view.setBigUint64(0, params.targetSlot, true);
  data.set(params.seedKey.toBytes(), 17);

  const ix = new TransactionInstruction({
    programId: SLOT_HEIGHT_RESOLVER_PROGRAM_ID,
    keys: [
      { pubkey: params.payer, isSigner: true, isWritable: true },
      { pubkey: state, isSigner: false, isWritable: true },
      { pubkey: params.authority, isSigner: true, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });

  return { ix, state };
}

// ============================================================
// Pyth-price resolver
// ============================================================

export type PythComparison = "gte" | "lt";

export interface PythPriceResolverInitParams {
  payer: PublicKey;
  authority: PublicKey;
  seedKey: PublicKey;
  priceFeed: PublicKey;
  earliestSlot: bigint;
  thresholdPrice: bigint;
  thresholdExpo: number;
  comparison: PythComparison;
}

export interface PythPriceResolverInitResult {
  ix: TransactionInstruction;
  state: PublicKey;
}

export function initializePythPriceResolverIx(
  params: PythPriceResolverInitParams,
): PythPriceResolverInitResult {
  const [state, bump] = derivePythResolverStatePda(params.seedKey);

  // data: tag(1) + [bump:u8, comparison:u8, pad:6, price_feed:32, earliest:u64,
  //                 threshold_price:i64, threshold_expo:i32, pad:4, seed_key:32] (96 bytes)
  const data = new Uint8Array(1 + 96);
  data[0] = RESOLVER_IX.Initialize;
  data[1] = bump;
  data[2] = params.comparison === "gte" ? 0 : 1;
  // bytes [3..9] padding
  data.set(params.priceFeed.toBytes(), 9);
  const view = new DataView(data.buffer, data.byteOffset + 41, 24);
  view.setBigUint64(0, params.earliestSlot, true);
  view.setBigInt64(8, params.thresholdPrice, true);
  view.setInt32(16, params.thresholdExpo, true);
  data.set(params.seedKey.toBytes(), 65);

  const ix = new TransactionInstruction({
    programId: PYTH_PRICE_RESOLVER_PROGRAM_ID,
    keys: [
      { pubkey: params.payer, isSigner: true, isWritable: true },
      { pubkey: state, isSigner: false, isWritable: true },
      { pubkey: params.authority, isSigner: true, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });

  return { ix, state };
}
