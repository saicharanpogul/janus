import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";

import {
  MARKET_FACTORY_IX,
  MARKET_FACTORY_PROGRAM_ID,
} from "../constants.js";
import { deriveRegistrationPda } from "../pda.js";

export interface RegisterMarketParams {
  payer: PublicKey;
  market: PublicKey;
  pool: PublicKey;
  /** Optional 32-byte hash of the off-chain question text. */
  questionHash?: Uint8Array;
}

export interface RegisterMarketResult {
  ix: TransactionInstruction;
  registration: PublicKey;
}

export function registerMarketIx(
  params: RegisterMarketParams,
): RegisterMarketResult {
  const [registration, bump] = deriveRegistrationPda(params.market);

  // data: tag(1) + [bump:u8, pad:7, question_hash:32] (40 bytes)
  const data = new Uint8Array(1 + 40);
  data[0] = MARKET_FACTORY_IX.Register;
  data[1] = bump;
  if (params.questionHash) {
    if (params.questionHash.length !== 32) {
      throw new Error("questionHash must be 32 bytes");
    }
    data.set(params.questionHash, 9);
  }

  const ix = new TransactionInstruction({
    programId: MARKET_FACTORY_PROGRAM_ID,
    keys: [
      { pubkey: params.payer, isSigner: true, isWritable: true },
      { pubkey: registration, isSigner: false, isWritable: true },
      { pubkey: params.market, isSigner: false, isWritable: false },
      { pubkey: params.pool, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });

  return { ix, registration };
}
