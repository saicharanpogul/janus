import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

import {
  CONDITIONAL_TOKENS_IX,
  CONDITIONAL_TOKENS_PROGRAM_ID,
} from "../constants.js";
import {
  deriveMarketPda,
  deriveNoMintPda,
  deriveVaultPda,
  deriveYesMintPda,
} from "../pda.js";

const writeU64Le = (view: DataView, offset: number, n: bigint) =>
  view.setBigUint64(offset, n, true);

// ----- InitializeMarket -----

export interface InitializeMarketParams {
  payer: PublicKey;
  authority: PublicKey;
  collateralMint: PublicKey;
  resolverProgram: PublicKey;
  resolverState: PublicKey;
  deadlineSlot: bigint;
}

export interface InitializeMarketResult {
  ix: TransactionInstruction;
  market: PublicKey;
  yesMint: PublicKey;
  noMint: PublicKey;
  vault: PublicKey;
}

export function initializeMarketIx(
  params: InitializeMarketParams,
): InitializeMarketResult {
  const [market, marketBump] = deriveMarketPda(
    params.collateralMint,
    params.resolverState,
    params.deadlineSlot,
  );
  const [yesMint, yesBump] = deriveYesMintPda(market);
  const [noMint, noBump] = deriveNoMintPda(market);
  const [vault, vaultBump] = deriveVaultPda(market);

  // data: tag(1) + InitializeMarketData (16 bytes)
  const data = new Uint8Array(17);
  data[0] = CONDITIONAL_TOKENS_IX.InitializeMarket;
  const view = new DataView(data.buffer, data.byteOffset + 1, 16);
  writeU64Le(view, 0, params.deadlineSlot);
  view.setUint8(8, marketBump);
  view.setUint8(9, yesBump);
  view.setUint8(10, noBump);
  view.setUint8(11, vaultBump);

  const ix = new TransactionInstruction({
    programId: CONDITIONAL_TOKENS_PROGRAM_ID,
    keys: [
      { pubkey: params.payer, isSigner: true, isWritable: true },
      { pubkey: market, isSigner: false, isWritable: true },
      { pubkey: params.collateralMint, isSigner: false, isWritable: false },
      { pubkey: yesMint, isSigner: false, isWritable: true },
      { pubkey: noMint, isSigner: false, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: params.resolverProgram, isSigner: false, isWritable: false },
      { pubkey: params.resolverState, isSigner: false, isWritable: false },
      { pubkey: params.authority, isSigner: true, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });

  return { ix, market, yesMint, noMint, vault };
}

// ----- Split / Merge / Redeem -----

interface AmountParamsBase {
  user: PublicKey;
  market: PublicKey;
  userCollateral: PublicKey;
  vault: PublicKey;
  yesMint: PublicKey;
  noMint: PublicKey;
  userYes: PublicKey;
  userNo: PublicKey;
  amount: bigint;
}

function amountIx(
  tag: number,
  p: AmountParamsBase,
): TransactionInstruction {
  const data = new Uint8Array(9);
  data[0] = tag;
  new DataView(data.buffer, data.byteOffset + 1, 8).setBigUint64(0, p.amount, true);
  return new TransactionInstruction({
    programId: CONDITIONAL_TOKENS_PROGRAM_ID,
    keys: [
      { pubkey: p.user, isSigner: true, isWritable: false },
      { pubkey: p.market, isSigner: false, isWritable: false },
      { pubkey: p.userCollateral, isSigner: false, isWritable: true },
      { pubkey: p.vault, isSigner: false, isWritable: true },
      { pubkey: p.yesMint, isSigner: false, isWritable: true },
      { pubkey: p.noMint, isSigner: false, isWritable: true },
      { pubkey: p.userYes, isSigner: false, isWritable: true },
      { pubkey: p.userNo, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });
}

export const splitIx = (p: AmountParamsBase) =>
  amountIx(CONDITIONAL_TOKENS_IX.Split, p);

export const mergeIx = (p: AmountParamsBase) =>
  amountIx(CONDITIONAL_TOKENS_IX.Merge, p);

export interface RedeemParams {
  user: PublicKey;
  market: PublicKey;
  userCollateral: PublicKey;
  vault: PublicKey;
  winningMint: PublicKey;
  userWinning: PublicKey;
  amount: bigint;
}

export function redeemIx(p: RedeemParams): TransactionInstruction {
  const data = new Uint8Array(9);
  data[0] = CONDITIONAL_TOKENS_IX.Redeem;
  new DataView(data.buffer, data.byteOffset + 1, 8).setBigUint64(0, p.amount, true);
  return new TransactionInstruction({
    programId: CONDITIONAL_TOKENS_PROGRAM_ID,
    keys: [
      { pubkey: p.user, isSigner: true, isWritable: false },
      { pubkey: p.market, isSigner: false, isWritable: false },
      { pubkey: p.userCollateral, isSigner: false, isWritable: true },
      { pubkey: p.vault, isSigner: false, isWritable: true },
      { pubkey: p.winningMint, isSigner: false, isWritable: true },
      { pubkey: p.userWinning, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });
}

// ----- Resolve -----

export interface ResolveParams {
  caller: PublicKey;
  market: PublicKey;
  resolverProgram: PublicKey;
  resolverState: PublicKey;
  /** Additional accounts the resolver needs beyond its state account. */
  extraResolverAccounts?: PublicKey[];
}

export function resolveIx(p: ResolveParams): TransactionInstruction {
  const data = new Uint8Array([CONDITIONAL_TOKENS_IX.Resolve]);
  const keys = [
    { pubkey: p.caller, isSigner: true, isWritable: false },
    { pubkey: p.market, isSigner: false, isWritable: true },
    { pubkey: p.resolverProgram, isSigner: false, isWritable: false },
    { pubkey: p.resolverState, isSigner: false, isWritable: false },
  ];
  for (const extra of p.extraResolverAccounts ?? []) {
    keys.push({ pubkey: extra, isSigner: false, isWritable: false });
  }
  return new TransactionInstruction({
    programId: CONDITIONAL_TOKENS_PROGRAM_ID,
    keys,
    data: Buffer.from(data),
  });
}
