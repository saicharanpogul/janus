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
  /** 32-byte feed identifier matching the on-chain `PriceUpdateV2.feed_id`. */
  feedId: Uint8Array;
  earliestSlot: bigint;
  /** Max slot drift tolerated between `posted_slot` and current slot. */
  maxStalenessSlots: bigint;
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
  if (params.feedId.length !== 32) {
    throw new Error("feedId must be 32 bytes");
  }
  const [state, bump] = derivePythResolverStatePda(params.seedKey);

  // data: tag(1) + 136 bytes:
  //   [0]      bump : u8
  //   [1]      comparison : u8 (0=gte, 1=lt)
  //   [2..8]   padding
  //   [8..40]  price_feed : Pubkey
  //   [40..72] feed_id : [u8;32]
  //   [72..80] earliest_slot : u64
  //   [80..88] max_staleness_slots : u64
  //   [88..96] threshold_price : i64
  //   [96..100] threshold_expo : i32
  //   [100..104] padding
  //   [104..136] seed_key : [u8;32]
  const data = new Uint8Array(1 + 136);
  data[0] = RESOLVER_IX.Initialize;
  data[1] = bump;
  data[2] = params.comparison === "gte" ? 0 : 1;
  data.set(params.priceFeed.toBytes(), 9);
  data.set(params.feedId, 41);
  const view = new DataView(data.buffer, data.byteOffset + 73, 28);
  view.setBigUint64(0, params.earliestSlot, true);
  view.setBigUint64(8, params.maxStalenessSlots, true);
  view.setBigInt64(16, params.thresholdPrice, true);
  view.setInt32(24, params.thresholdExpo, true);
  data.set(params.seedKey.toBytes(), 105);

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
