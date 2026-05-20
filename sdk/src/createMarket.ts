import {
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";

import {
  initializeMarketIx,
  splitIx,
} from "./ix/conditional-tokens.js";
import { initializePoolIx } from "./ix/lmsr-market.js";
import { registerMarketIx } from "./ix/market-factory.js";
import { initializeSlotHeightResolverIx } from "./ix/resolvers.js";

/**
 * Compose the full instruction set for creating a Janus market backed by
 * a slot-height resolver — the simplest end-to-end demo path.
 *
 * Returns the instructions in execution order. The caller is responsible
 * for assembling them into one or more transactions. With ALTs this whole
 * sequence fits comfortably in a single transaction.
 *
 * Flow:
 *   1. Initialize slot-height resolver state
 *   2. Initialize conditional-tokens market (creates outcome mints + vault)
 *   3. Create creator's YES + NO ATAs (idempotent)
 *   4. Split N collateral into N YES + N NO for the creator
 *   5. Initialize LMSR pool (pulls N YES + N NO from creator as subsidy)
 *   6. Register market in the factory
 */
export interface CreateMarketWithSlotResolverParams {
  payer: PublicKey;
  authority: PublicKey;
  collateralMint: PublicKey;
  /** Creator's existing token account for the collateral mint. */
  creatorCollateral: PublicKey;
  /** Slot at which the market is allowed to resolve. */
  deadlineSlot: bigint;
  /** Outcome the resolver reports once the slot is reached. */
  resolutionOutcome: 1 | 2 | 3;
  /** Initial subsidy amount (in collateral units) to seed the AMM pool with. */
  subsidy: bigint;
  /** Swap fee in basis points (max 1000). */
  feeBps: number;
  /** Optional 32-byte off-chain question text hash for the registry. */
  questionHash?: Uint8Array;
  /**
   * Caller-chosen 32-byte tag for the resolver state PDA. To keep things
   * simple, default to the authority pubkey — works as long as one
   * authority creates at most one slot-height resolver instance per
   * combination of (authority, target slot, collateral). Pass an explicit
   * seed key when you need multiple resolvers from the same authority.
   */
  resolverSeedKey?: PublicKey;
}

export interface CreateMarketResult {
  instructions: TransactionInstruction[];
  market: PublicKey;
  pool: PublicKey;
  yesMint: PublicKey;
  noMint: PublicKey;
  vault: PublicKey;
  yesVault: PublicKey;
  noVault: PublicKey;
  resolverState: PublicKey;
  registration: PublicKey;
}

export function createMarketWithSlotResolver(
  params: CreateMarketWithSlotResolverParams,
): CreateMarketResult {
  const seedKey = params.resolverSeedKey ?? params.authority;

  // (1) resolver init
  const resolverInit = initializeSlotHeightResolverIx({
    payer: params.payer,
    authority: params.authority,
    seedKey,
    outcomeAtOrAfter: params.resolutionOutcome,
    targetSlot: params.deadlineSlot,
  });

  // (2) market init
  const marketInit = initializeMarketIx({
    payer: params.payer,
    authority: params.authority,
    collateralMint: params.collateralMint,
    resolverProgram: resolverInit.ix.programId,
    resolverState: resolverInit.state,
    deadlineSlot: params.deadlineSlot,
  });

  // (3) ATA creation for creator's YES + NO holdings
  const creatorYes = getAssociatedTokenAddressSync(
    marketInit.yesMint,
    params.authority,
    true,
  );
  const creatorNo = getAssociatedTokenAddressSync(
    marketInit.noMint,
    params.authority,
    true,
  );
  const createYesAta = createAssociatedTokenAccountIdempotentInstruction(
    params.payer,
    creatorYes,
    params.authority,
    marketInit.yesMint,
  );
  const createNoAta = createAssociatedTokenAccountIdempotentInstruction(
    params.payer,
    creatorNo,
    params.authority,
    marketInit.noMint,
  );

  // (4) split: pull `subsidy` of collateral from creator, mint matching YES + NO
  const split = splitIx({
    user: params.authority,
    market: marketInit.market,
    userCollateral: params.creatorCollateral,
    vault: marketInit.vault,
    yesMint: marketInit.yesMint,
    noMint: marketInit.noMint,
    userYes: creatorYes,
    userNo: creatorNo,
    amount: params.subsidy,
  });

  // (5) pool init: take the freshly-minted YES + NO as the pool's subsidy
  const poolInit = initializePoolIx({
    payer: params.payer,
    market: marketInit.market,
    yesMint: marketInit.yesMint,
    noMint: marketInit.noMint,
    creatorYes,
    creatorNo,
    subsidyYes: params.subsidy,
    subsidyNo: params.subsidy,
    feeBps: params.feeBps,
  });

  // (6) register in factory
  const register = registerMarketIx({
    payer: params.payer,
    market: marketInit.market,
    pool: poolInit.pool,
    questionHash: params.questionHash,
  });

  return {
    instructions: [
      resolverInit.ix,
      marketInit.ix,
      createYesAta,
      createNoAta,
      split,
      poolInit.ix,
      register.ix,
    ],
    market: marketInit.market,
    pool: poolInit.pool,
    yesMint: marketInit.yesMint,
    noMint: marketInit.noMint,
    vault: marketInit.vault,
    yesVault: poolInit.yesVault,
    noVault: poolInit.noVault,
    resolverState: resolverInit.state,
    registration: register.registration,
  };
}
