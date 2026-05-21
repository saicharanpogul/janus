import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

import {
  LMSR_TRUE_MARKET_IX,
  LMSR_TRUE_MARKET_PROGRAM_ID,
  TRUE_SIDE,
} from "../constants.js";
import {
  deriveTruePoolCollateralVaultPda,
  deriveTruePoolPda,
} from "../pda.js";

export type TrueSide = "yes" | "no";

const sideByte = (s: TrueSide) => (s === "yes" ? TRUE_SIDE.Yes : TRUE_SIDE.No);

// ============================================================
// InitializePool
// ============================================================
//
// Creates the true-LMSR pool + collateral vault. The pool acts as the
// mint authority for YES + NO (which the caller is responsible for
// creating up-front with the pool PDA as mint_authority). The creator
// transfers `initialSubsidy` collateral into the vault; the on-chain
// program rejects subsidies below `ceil(b · ln(2))` for the bounded-
// loss invariant to hold.

export interface TrueInitializePoolParams {
  payer: PublicKey;
  resolverProgram: PublicKey;
  resolverState: PublicKey;
  collateralMint: PublicKey;
  /** Pre-created YES mint with the (to-be-derived) pool PDA as authority. */
  yesMint: PublicKey;
  /** Pre-created NO mint with the (to-be-derived) pool PDA as authority. */
  noMint: PublicKey;
  /** Payer's collateral token account, source of the initial subsidy. */
  payerCollateral: PublicKey;
  /** Liquidity parameter `b` in outcome-token base units (fits u32). */
  b: bigint;
  /** Initial subsidy in collateral base units; must be >= ceil(b · ln(2)). */
  initialSubsidy: bigint;
}

export interface TrueInitializePoolResult {
  ix: TransactionInstruction;
  pool: PublicKey;
  collateralVault: PublicKey;
}

export function trueInitializePoolIx(
  params: TrueInitializePoolParams,
): TrueInitializePoolResult {
  const [pool, poolBump] = deriveTruePoolPda(params.resolverState);
  const [collateralVault, vaultBump] = deriveTruePoolCollateralVaultPda(pool);

  // data: tag(1) + InitializePoolData (24 bytes)
  const data = new Uint8Array(25);
  data[0] = LMSR_TRUE_MARKET_IX.InitializePool;
  const view = new DataView(data.buffer, data.byteOffset + 1, 24);
  view.setBigUint64(0, params.b, true);
  view.setBigUint64(8, params.initialSubsidy, true);
  view.setUint8(16, poolBump);
  view.setUint8(17, vaultBump);

  const ix = new TransactionInstruction({
    programId: LMSR_TRUE_MARKET_PROGRAM_ID,
    keys: [
      { pubkey: params.payer, isSigner: true, isWritable: true },
      { pubkey: pool, isSigner: false, isWritable: true },
      { pubkey: params.resolverProgram, isSigner: false, isWritable: false },
      { pubkey: params.resolverState, isSigner: false, isWritable: false },
      { pubkey: params.collateralMint, isSigner: false, isWritable: false },
      { pubkey: collateralVault, isSigner: false, isWritable: true },
      { pubkey: params.yesMint, isSigner: false, isWritable: false },
      { pubkey: params.noMint, isSigner: false, isWritable: false },
      { pubkey: params.payerCollateral, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });

  return { ix, pool, collateralVault };
}

// ============================================================
// Buy (collateral in, outcome tokens out)
// ============================================================

export interface TrueBuyParams {
  user: PublicKey;
  pool: PublicKey;
  collateralVault: PublicKey;
  userCollateral: PublicKey;
  /** YES or NO mint depending on `side`. */
  outcomeMint: PublicKey;
  /** User's destination ATA for the freshly-minted outcome tokens. */
  userOutcome: PublicKey;
  /** Number of outcome tokens to receive. */
  delta: bigint;
  /** Max collateral the user will pay (slippage protection). */
  maxCollateralIn: bigint;
  side: TrueSide;
}

export function trueBuyIx(p: TrueBuyParams): TransactionInstruction {
  const data = new Uint8Array(25);
  data[0] = LMSR_TRUE_MARKET_IX.Buy;
  const view = new DataView(data.buffer, data.byteOffset + 1, 24);
  view.setBigUint64(0, p.delta, true);
  view.setBigUint64(8, p.maxCollateralIn, true);
  view.setUint8(16, sideByte(p.side));

  return new TransactionInstruction({
    programId: LMSR_TRUE_MARKET_PROGRAM_ID,
    keys: [
      { pubkey: p.user, isSigner: true, isWritable: false },
      { pubkey: p.pool, isSigner: false, isWritable: true },
      { pubkey: p.collateralVault, isSigner: false, isWritable: true },
      { pubkey: p.userCollateral, isSigner: false, isWritable: true },
      { pubkey: p.outcomeMint, isSigner: false, isWritable: true },
      { pubkey: p.userOutcome, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });
}

// ============================================================
// Sell (outcome tokens burned, collateral out)
// ============================================================

export interface TrueSellParams {
  user: PublicKey;
  pool: PublicKey;
  collateralVault: PublicKey;
  userCollateral: PublicKey;
  outcomeMint: PublicKey;
  /** User's source ATA holding the outcome tokens to burn. */
  userOutcome: PublicKey;
  delta: bigint;
  minCollateralOut: bigint;
  side: TrueSide;
}

export function trueSellIx(p: TrueSellParams): TransactionInstruction {
  const data = new Uint8Array(25);
  data[0] = LMSR_TRUE_MARKET_IX.Sell;
  const view = new DataView(data.buffer, data.byteOffset + 1, 24);
  view.setBigUint64(0, p.delta, true);
  view.setBigUint64(8, p.minCollateralOut, true);
  view.setUint8(16, sideByte(p.side));

  return new TransactionInstruction({
    programId: LMSR_TRUE_MARKET_PROGRAM_ID,
    keys: [
      { pubkey: p.user, isSigner: true, isWritable: false },
      { pubkey: p.pool, isSigner: false, isWritable: true },
      { pubkey: p.collateralVault, isSigner: false, isWritable: true },
      { pubkey: p.userCollateral, isSigner: false, isWritable: true },
      { pubkey: p.outcomeMint, isSigner: false, isWritable: true },
      { pubkey: p.userOutcome, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });
}

// ============================================================
// WithdrawResidual (post-resolution sweep)
// ============================================================

export interface TrueWithdrawResidualParams {
  authority: PublicKey;
  pool: PublicKey;
  collateralVault: PublicKey;
  /** Authority's destination collateral token account. */
  destination: PublicKey;
  amount: bigint;
}

export function trueWithdrawResidualIx(
  p: TrueWithdrawResidualParams,
): TransactionInstruction {
  const data = new Uint8Array(9);
  data[0] = LMSR_TRUE_MARKET_IX.WithdrawResidual;
  const view = new DataView(data.buffer, data.byteOffset + 1, 8);
  view.setBigUint64(0, p.amount, true);

  return new TransactionInstruction({
    programId: LMSR_TRUE_MARKET_PROGRAM_ID,
    keys: [
      { pubkey: p.authority, isSigner: true, isWritable: false },
      { pubkey: p.pool, isSigner: false, isWritable: true },
      { pubkey: p.collateralVault, isSigner: false, isWritable: true },
      { pubkey: p.destination, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });
}

// ============================================================
// Pricing helpers (off-chain mirror of the on-chain math)
// ============================================================
//
// All math operates on doubles; on-chain the program uses Q32.32 fixed-
// point with ~1e-4 relative error vs f64. For quoting purposes that's
// well within trader UX tolerance.

/** LMSR cost C(b, q_yes, q_no) = b · ln(exp(q_yes/b) + exp(q_no/b)). */
export function lmsrCost(b: number, qYes: number, qNo: number): number {
  if (b <= 0) return Number.NaN;
  // Log-sum-exp trick for stability.
  const a = qYes / b;
  const c = qNo / b;
  const m = Math.max(a, c);
  return b * (m + Math.log(Math.exp(a - m) + Math.exp(c - m)));
}

/** Cost to buy `delta` of YES at current reserves. */
export function buyYesCost(
  b: number,
  qYes: number,
  qNo: number,
  delta: number,
): number {
  return lmsrCost(b, qYes + delta, qNo) - lmsrCost(b, qYes, qNo);
}

/** Cost to buy `delta` of NO at current reserves. */
export function buyNoCost(
  b: number,
  qYes: number,
  qNo: number,
  delta: number,
): number {
  return lmsrCost(b, qYes, qNo + delta) - lmsrCost(b, qYes, qNo);
}

/** Instantaneous YES price: p_yes = exp(q_yes/b) / (exp(q_yes/b) + exp(q_no/b)). */
export function priceYes(b: number, qYes: number, qNo: number): number {
  if (b <= 0) return 0.5;
  const a = qYes / b;
  const c = qNo / b;
  const m = Math.max(a, c);
  const ea = Math.exp(a - m);
  const ec = Math.exp(c - m);
  return ea / (ea + ec);
}

/** Bounded-loss floor: `b · ln(2)` in collateral base units. */
export function bLn2(b: number): number {
  return b * Math.LN2;
}
