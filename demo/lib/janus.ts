// Janus on-chain data layer.
//
// Wraps the SDK with read helpers that scan registrations + decode the
// Rust state structs at their byte offsets.

import { Connection, PublicKey } from "@solana/web3.js";
import {
  MARKET_FACTORY_PROGRAM_ID,
  CONDITIONAL_TOKENS_PROGRAM_ID,
  LMSR_MARKET_PROGRAM_ID,
} from "@janus/sdk";

// ---- On-chain layouts ----

export interface MarketRegistration {
  registration: PublicKey;
  market: PublicKey;
  pool: PublicKey;
  resolverProgram: PublicKey;
  resolverState: PublicKey;
  creator: PublicKey;
  deadlineSlot: bigint;
  createdAtSlot: bigint;
  questionHash: Uint8Array;
}

export interface MarketState {
  market: PublicKey;
  status: number; // 0=Active, 1=ResolvedYes, 2=ResolvedNo, 3=Invalid
  collateralMint: PublicKey;
  yesMint: PublicKey;
  noMint: PublicKey;
  vault: PublicKey;
  resolverProgram: PublicKey;
  resolverState: PublicKey;
  authority: PublicKey;
  deadlineSlot: bigint;
  createdAtSlot: bigint;
}

export interface PoolState {
  pool: PublicKey;
  market: PublicKey;
  yesMint: PublicKey;
  noMint: PublicKey;
  yesVault: PublicKey;
  noVault: PublicKey;
  authority: PublicKey;
  yesReserves: bigint;
  noReserves: bigint;
  feeBps: number;
}

// MarketRegistration = 216 bytes (market-factory state)
function decodeRegistration(
  pubkey: PublicKey,
  data: Buffer,
): MarketRegistration | null {
  if (data.length !== 216) return null;
  return {
    registration: pubkey,
    market: new PublicKey(data.subarray(8, 40)),
    pool: new PublicKey(data.subarray(40, 72)),
    resolverProgram: new PublicKey(data.subarray(72, 104)),
    resolverState: new PublicKey(data.subarray(104, 136)),
    creator: new PublicKey(data.subarray(136, 168)),
    deadlineSlot: data.readBigUInt64LE(168),
    createdAtSlot: data.readBigUInt64LE(176),
    questionHash: new Uint8Array(data.subarray(184, 216)),
  };
}

// Market = 248 bytes (conditional-tokens state)
function decodeMarket(pubkey: PublicKey, data: Buffer): MarketState | null {
  if (data.length !== 248) return null;
  return {
    market: pubkey,
    status: data[1],
    collateralMint: new PublicKey(data.subarray(8, 40)),
    yesMint: new PublicKey(data.subarray(40, 72)),
    noMint: new PublicKey(data.subarray(72, 104)),
    vault: new PublicKey(data.subarray(104, 136)),
    resolverProgram: new PublicKey(data.subarray(136, 168)),
    resolverState: new PublicKey(data.subarray(168, 200)),
    authority: new PublicKey(data.subarray(200, 232)),
    deadlineSlot: data.readBigUInt64LE(232),
    createdAtSlot: data.readBigUInt64LE(240),
  };
}

// Pool = 224 bytes (lmsr-market state)
function decodePool(pubkey: PublicKey, data: Buffer): PoolState | null {
  if (data.length !== 224) return null;
  return {
    pool: pubkey,
    market: new PublicKey(data.subarray(8, 40)),
    yesMint: new PublicKey(data.subarray(40, 72)),
    noMint: new PublicKey(data.subarray(72, 104)),
    yesVault: new PublicKey(data.subarray(104, 136)),
    noVault: new PublicKey(data.subarray(136, 168)),
    authority: new PublicKey(data.subarray(168, 200)),
    yesReserves: data.readBigUInt64LE(200),
    noReserves: data.readBigUInt64LE(208),
    feeBps: data.readUInt16LE(216),
  };
}

// ---- Fetchers ----

export async function fetchAllRegistrations(
  conn: Connection,
): Promise<MarketRegistration[]> {
  const accounts = await conn.getProgramAccounts(MARKET_FACTORY_PROGRAM_ID, {
    filters: [{ dataSize: 216 }],
    commitment: "confirmed",
  });
  return accounts
    .map((a) => decodeRegistration(a.pubkey, a.account.data))
    .filter((r): r is MarketRegistration => r !== null)
    .sort((a, b) => Number(b.createdAtSlot - a.createdAtSlot));
}

export async function fetchMarket(
  conn: Connection,
  market: PublicKey,
): Promise<MarketState | null> {
  const info = await conn.getAccountInfo(market, "confirmed");
  if (!info) return null;
  return decodeMarket(market, info.data);
}

export async function fetchPool(
  conn: Connection,
  pool: PublicKey,
): Promise<PoolState | null> {
  const info = await conn.getAccountInfo(pool, "confirmed");
  if (!info) return null;
  return decodePool(pool, info.data);
}

export async function fetchMarketAndPool(
  conn: Connection,
  market: PublicKey,
  pool: PublicKey,
): Promise<{ market: MarketState | null; pool: PoolState | null }> {
  const [m, p] = await Promise.all([
    fetchMarket(conn, market),
    fetchPool(conn, pool),
  ]);
  return { market: m, pool: p };
}

// ---- Pricing ----

/** YES probability implied by CPMM reserves: `no / (yes + no)`.
 * For balanced reserves this is 0.5; pushing q_yes up shifts probability
 * toward YES. */
export function impliedYesProbability(p: PoolState): number {
  const total = p.yesReserves + p.noReserves;
  if (total === 0n) return 0.5;
  // Higher YES reserves means more YES supply ⇒ less scarce ⇒ lower price.
  // p_yes = no / (yes + no).
  return Number(p.noReserves) / Number(total);
}

/** CPMM swap output: `amount_in_after_fee * out / (in + amount_in_after_fee)`. */
export function quoteSwap(
  pool: PoolState,
  direction: "yesToNo" | "noToYes",
  amountIn: bigint,
): { amountOut: bigint; priceImpact: number } {
  const fee = BigInt(pool.feeBps);
  const inAfterFee = (amountIn * (10000n - fee)) / 10000n;
  const [inR, outR] =
    direction === "yesToNo"
      ? [pool.yesReserves, pool.noReserves]
      : [pool.noReserves, pool.yesReserves];
  if (inR === 0n) return { amountOut: 0n, priceImpact: 0 };
  const num = outR * inAfterFee;
  const den = inR + inAfterFee;
  const amountOut = den === 0n ? 0n : num / den;
  // Spot price = outR / inR; effective = amountOut / amountIn.
  const spot = Number(outR) / Number(inR || 1n);
  const effective =
    amountIn === 0n ? spot : Number(amountOut) / Number(amountIn);
  const priceImpact = spot === 0 ? 0 : Math.abs(effective - spot) / spot;
  return { amountOut, priceImpact };
}

// ---- Display helpers ----

const STATUS_LABEL = ["Active", "Resolved YES", "Resolved NO", "Invalid"];

export function statusLabel(s: number): string {
  return STATUS_LABEL[s] ?? "Unknown";
}

export function formatPubkey(p: PublicKey | string, len = 4): string {
  const s = typeof p === "string" ? p : p.toBase58();
  return `${s.slice(0, len)}…${s.slice(-len)}`;
}

export function formatTokenAmount(
  amount: bigint | number,
  decimals: number,
  maxFrac = 4,
): string {
  const n = typeof amount === "bigint" ? Number(amount) : amount;
  const scale = 10 ** decimals;
  const v = n / scale;
  return v.toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: maxFrac,
  });
}

export function formatPct(x: number, frac = 2): string {
  return `${(x * 100).toLocaleString(undefined, {
    minimumFractionDigits: frac,
    maximumFractionDigits: frac,
  })}%`;
}

export function questionHashToHex(h: Uint8Array): string {
  if (!h || h.every((b) => b === 0)) return "";
  return Array.from(h)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export { CONDITIONAL_TOKENS_PROGRAM_ID, LMSR_MARKET_PROGRAM_ID, MARKET_FACTORY_PROGRAM_ID };
