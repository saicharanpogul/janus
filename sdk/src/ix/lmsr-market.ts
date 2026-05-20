import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

import { LMSR_MARKET_IX, LMSR_MARKET_PROGRAM_ID } from "../constants.js";
import {
  derivePoolNoVaultPda,
  derivePoolPda,
  derivePoolYesVaultPda,
} from "../pda.js";

// ----- InitializePool -----

export interface InitializePoolParams {
  payer: PublicKey;
  market: PublicKey;
  yesMint: PublicKey;
  noMint: PublicKey;
  creatorYes: PublicKey;
  creatorNo: PublicKey;
  subsidyYes: bigint;
  subsidyNo: bigint;
  feeBps: number;
}

export interface InitializePoolResult {
  ix: TransactionInstruction;
  pool: PublicKey;
  yesVault: PublicKey;
  noVault: PublicKey;
}

export function initializePoolIx(
  params: InitializePoolParams,
): InitializePoolResult {
  const [pool, poolBump] = derivePoolPda(params.market);
  const [yesVault, yesVaultBump] = derivePoolYesVaultPda(pool);
  const [noVault, noVaultBump] = derivePoolNoVaultPda(pool);

  // data: tag(1) + InitializePoolData (24 bytes)
  const data = new Uint8Array(25);
  data[0] = LMSR_MARKET_IX.InitializePool;
  const view = new DataView(data.buffer, data.byteOffset + 1, 24);
  view.setBigUint64(0, params.subsidyYes, true);
  view.setBigUint64(8, params.subsidyNo, true);
  view.setUint16(16, params.feeBps, true);
  view.setUint8(18, poolBump);
  view.setUint8(19, yesVaultBump);
  view.setUint8(20, noVaultBump);

  const ix = new TransactionInstruction({
    programId: LMSR_MARKET_PROGRAM_ID,
    keys: [
      { pubkey: params.payer, isSigner: true, isWritable: true },
      { pubkey: pool, isSigner: false, isWritable: true },
      { pubkey: params.market, isSigner: false, isWritable: false },
      { pubkey: params.yesMint, isSigner: false, isWritable: false },
      { pubkey: params.noMint, isSigner: false, isWritable: false },
      { pubkey: yesVault, isSigner: false, isWritable: true },
      { pubkey: noVault, isSigner: false, isWritable: true },
      { pubkey: params.creatorYes, isSigner: false, isWritable: true },
      { pubkey: params.creatorNo, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });

  return { ix, pool, yesVault, noVault };
}

// ----- Swap -----

export type SwapDirection = "yesToNo" | "noToYes";

export interface SwapParams {
  user: PublicKey;
  pool: PublicKey;
  yesVault: PublicKey;
  noVault: PublicKey;
  userInToken: PublicKey;
  userOutToken: PublicKey;
  amountIn: bigint;
  minAmountOut: bigint;
  direction: SwapDirection;
}

export function swapIx(p: SwapParams): TransactionInstruction {
  const data = new Uint8Array(25);
  data[0] = LMSR_MARKET_IX.Swap;
  const view = new DataView(data.buffer, data.byteOffset + 1, 24);
  view.setBigUint64(0, p.amountIn, true);
  view.setBigUint64(8, p.minAmountOut, true);
  view.setUint8(16, p.direction === "yesToNo" ? 0 : 1);

  return new TransactionInstruction({
    programId: LMSR_MARKET_PROGRAM_ID,
    keys: [
      { pubkey: p.user, isSigner: true, isWritable: false },
      { pubkey: p.pool, isSigner: false, isWritable: true },
      { pubkey: p.yesVault, isSigner: false, isWritable: true },
      { pubkey: p.noVault, isSigner: false, isWritable: true },
      { pubkey: p.userInToken, isSigner: false, isWritable: true },
      { pubkey: p.userOutToken, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });
}
