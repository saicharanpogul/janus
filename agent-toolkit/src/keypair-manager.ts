// Keypair + funding manager for swarm agents.
//
// Generates N keypairs deterministically (so re-runs reuse the same
// agents and their positions persist), persists them to disk, and
// airdrops SOL on devnet.

import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  Transaction,
  SystemProgram,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  AccountLayout,
  TOKEN_PROGRAM_ID,
  MINT_SIZE,
  createInitializeMintInstruction,
  createMintToInstruction,
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
  getMinimumBalanceForRentExemptMint,
} from "@solana/spl-token";
import { readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

export interface SwarmIdentity {
  /** Stable index in the swarm (0..N-1). */
  index: number;
  /** Stable string id for logs. */
  agentId: string;
  keypair: Keypair;
}

export interface SwarmSession {
  agents: SwarmIdentity[];
  /** Shared test collateral mint used by every agent in this session. */
  collateralMint: Keypair;
  collateralDecimals: number;
}

interface PersistedSession {
  collateralMintSecret: number[];
  collateralDecimals: number;
  agents: {
    index: number;
    agentId: string;
    secret: number[];
  }[];
}

/** Load or create a swarm session, persisting keypairs to disk. */
export function loadOrCreateSwarm(opts: {
  size: number;
  storagePath: string;
  collateralDecimals?: number;
}): SwarmSession {
  const decimals = opts.collateralDecimals ?? 6;
  const path = resolve(opts.storagePath);
  if (existsSync(path)) {
    const persisted: PersistedSession = JSON.parse(
      readFileSync(path, "utf-8"),
    );
    return {
      collateralMint: Keypair.fromSecretKey(
        Uint8Array.from(persisted.collateralMintSecret),
      ),
      collateralDecimals: persisted.collateralDecimals,
      agents: persisted.agents.map((a) => ({
        index: a.index,
        agentId: a.agentId,
        keypair: Keypair.fromSecretKey(Uint8Array.from(a.secret)),
      })),
    };
  }
  // Fresh session.
  const collateralMint = Keypair.generate();
  const agents = Array.from({ length: opts.size }, (_, i) => ({
    index: i,
    agentId: `agent-${i.toString().padStart(3, "0")}`,
    keypair: Keypair.generate(),
  }));
  const persisted: PersistedSession = {
    collateralMintSecret: Array.from(collateralMint.secretKey),
    collateralDecimals: decimals,
    agents: agents.map((a) => ({
      index: a.index,
      agentId: a.agentId,
      secret: Array.from(a.keypair.secretKey),
    })),
  };
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, JSON.stringify(persisted, null, 2));
  return { agents, collateralMint, collateralDecimals: decimals };
}

/**
 * Fund agents with SOL. Tries `requestAirdrop` first (works on local
 * validators and lightly-loaded devnets); falls back to a transfer
 * from `authority` when the faucet 429s — the public devnet faucet is
 * rate-limited heavily and unreliable for >1 wallet per minute.
 *
 * Throws if neither path can fund an agent below `minSol`.
 */
export async function fundAgentsWithSol(opts: {
  connection: Connection;
  swarm: SwarmSession;
  /** Authority keypair used as a fallback funding source (recommended on devnet). */
  authority?: Keypair;
  amountSol: number;
  minSol: number;
  /** How many faucet attempts before falling back to authority transfer. */
  faucetAttempts?: number;
}): Promise<void> {
  const faucetAttempts = opts.faucetAttempts ?? 1;
  for (const agent of opts.swarm.agents) {
    const balance = await opts.connection.getBalance(agent.keypair.publicKey);
    if (balance >= opts.minSol * LAMPORTS_PER_SOL) {
      console.log(
        `[fund] ${agent.agentId} already has ${(balance / LAMPORTS_PER_SOL).toFixed(3)} SOL — skipping`,
      );
      continue;
    }

    let funded = false;

    // Path 1: try the faucet briefly.
    for (let attempt = 0; attempt < faucetAttempts; attempt++) {
      try {
        const sig = await opts.connection.requestAirdrop(
          agent.keypair.publicKey,
          opts.amountSol * LAMPORTS_PER_SOL,
        );
        await opts.connection.confirmTransaction(sig, "confirmed");
        console.log(`[fund] ${agent.agentId} +${opts.amountSol} SOL via faucet`);
        funded = true;
        break;
      } catch {
        // fall through
      }
    }

    if (funded) continue;

    // Path 2: transfer from authority.
    if (!opts.authority) {
      throw new Error(
        `[fund] ${agent.agentId} faucet failed and no authority provided. Pass authority: Keypair to fundAgentsWithSol so it can transfer SOL.`,
      );
    }
    const tx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: opts.authority.publicKey,
        toPubkey: agent.keypair.publicKey,
        lamports: Math.floor(opts.amountSol * LAMPORTS_PER_SOL),
      }),
    );
    await sendAndConfirmTransaction(opts.connection, tx, [opts.authority]);
    console.log(
      `[fund] ${agent.agentId} +${opts.amountSol} SOL via authority transfer`,
    );
  }
}

/**
 * Set up the shared test collateral mint and seed each agent with `amount`
 * test USDC. Idempotent: agents who already have a sufficient balance are
 * skipped.
 */
export async function setupCollateralAndSeed(opts: {
  connection: Connection;
  swarm: SwarmSession;
  authority: Keypair; // Pays for setup; becomes mint authority
  amountPerAgent: bigint;
}): Promise<void> {
  const { connection, swarm } = opts;
  const mintPubkey = swarm.collateralMint.publicKey;

  // 1. Create the mint if it doesn't exist.
  const mintInfo = await connection.getAccountInfo(mintPubkey);
  if (!mintInfo) {
    console.log(`[setup] creating collateral mint ${mintPubkey.toBase58()}`);
    const rent = await getMinimumBalanceForRentExemptMint(connection);
    const tx = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: opts.authority.publicKey,
        newAccountPubkey: mintPubkey,
        space: MINT_SIZE,
        lamports: rent,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        mintPubkey,
        swarm.collateralDecimals,
        opts.authority.publicKey,
        null,
        TOKEN_PROGRAM_ID,
      ),
    );
    await sendAndConfirmTransaction(connection, tx, [opts.authority, swarm.collateralMint]);
  }

  // 2. Mint to each agent's ATA if they're short.
  for (const agent of swarm.agents) {
    const ata = getAssociatedTokenAddressSync(
      mintPubkey,
      agent.keypair.publicKey,
      true,
    );
    const info = await connection.getAccountInfo(ata);
    const current = info
      ? BigInt(AccountLayout.decode(info.data).amount.toString())
      : 0n;
    if (current >= opts.amountPerAgent) continue;
    const toMint = opts.amountPerAgent - current;
    const tx = new Transaction().add(
      createAssociatedTokenAccountIdempotentInstruction(
        opts.authority.publicKey,
        ata,
        agent.keypair.publicKey,
        mintPubkey,
      ),
      createMintToInstruction(mintPubkey, ata, opts.authority.publicKey, toMint),
    );
    await sendAndConfirmTransaction(connection, tx, [opts.authority]);
    console.log(`[setup] ${agent.agentId} seeded with ${toMint}`);
  }
}

/** Read an agent's current collateral ATA balance. */
export async function getCollateralBalance(
  connection: Connection,
  agent: SwarmIdentity,
  collateralMint: PublicKey,
): Promise<bigint> {
  const ata = getAssociatedTokenAddressSync(
    collateralMint,
    agent.keypair.publicKey,
    true,
  );
  const info = await connection.getAccountInfo(ata);
  if (!info) return 0n;
  return BigInt(AccountLayout.decode(info.data).amount.toString());
}
