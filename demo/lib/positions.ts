// User-position scanner: given a user's pubkey, find all YES/NO token
// balances across all known markets and pair them with implied prices.

import { Connection, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, AccountLayout } from "@solana/spl-token";

import {
  fetchAllRegistrations,
  fetchMarket,
  fetchPool,
  impliedYesProbability,
  type MarketRegistration,
  type MarketState,
  type PoolState,
} from "./janus";

export interface Position {
  registration: MarketRegistration;
  market: MarketState;
  pool: PoolState;
  yesBalance: bigint;
  noBalance: bigint;
  /** Implied YES probability in [0, 1]. */
  yesPrice: number;
  /** Mark-to-market value in collateral units. */
  value: number;
}

export async function fetchUserPositions(
  conn: Connection,
  user: PublicKey,
): Promise<{
  positions: Position[];
  totalValue: number;
  // We don't track cost on-chain — value is the only stat we can derive
  // without an off-chain trade log. Cost is left to the UI to overlay
  // from local storage.
}> {
  const regs = await fetchAllRegistrations(conn);
  if (regs.length === 0) return { positions: [], totalValue: 0 };

  // Fetch all markets + pools in parallel.
  const marketAddrs = regs.map((r) => r.market);
  const poolAddrs = regs.map((r) => r.pool);
  const [marketInfos, poolInfos] = await Promise.all([
    conn.getMultipleAccountsInfo(marketAddrs, "confirmed"),
    conn.getMultipleAccountsInfo(poolAddrs, "confirmed"),
  ]);

  // Compute user ATAs for each market's YES + NO mints.
  // First decode each market to get mints.
  const markets: (MarketState | null)[] = await Promise.all(
    marketAddrs.map((m, i) =>
      marketInfos[i]
        ? Promise.resolve(decodeMarketBuf(m, marketInfos[i]!.data))
        : Promise.resolve(null),
    ),
  );
  const pools: (PoolState | null)[] = poolInfos.map((info, i) =>
    info ? decodePoolBuf(poolAddrs[i], info.data) : null,
  );

  // Collect user ATAs.
  const ataList: PublicKey[] = [];
  const idxMap: { mIdx: number; side: "yes" | "no" }[] = [];
  markets.forEach((m, i) => {
    if (!m) return;
    const ataYes = getAssociatedTokenAddressSync(m.yesMint, user, true);
    const ataNo = getAssociatedTokenAddressSync(m.noMint, user, true);
    ataList.push(ataYes, ataNo);
    idxMap.push({ mIdx: i, side: "yes" }, { mIdx: i, side: "no" });
  });

  const ataInfos = await conn.getMultipleAccountsInfo(ataList, "confirmed");

  const balances: { yes: bigint; no: bigint }[] = markets.map(() => ({
    yes: 0n,
    no: 0n,
  }));
  ataInfos.forEach((info, k) => {
    if (!info) return;
    try {
      const parsed = AccountLayout.decode(info.data);
      const amount = BigInt(parsed.amount.toString());
      const { mIdx, side } = idxMap[k];
      balances[mIdx][side] = amount;
    } catch {
      /* not a token account */
    }
  });

  const positions: Position[] = [];
  let totalValue = 0;
  regs.forEach((reg, i) => {
    const m = markets[i];
    const p = pools[i];
    if (!m || !p) return;
    const yesBal = balances[i].yes;
    const noBal = balances[i].no;
    if (yesBal === 0n && noBal === 0n) return;
    const yesPrice = impliedYesProbability(p);
    // Value: YES position * yesPrice + NO position * (1 - yesPrice).
    // Collateral has 6 decimals (USDC-style) so scale accordingly.
    const COLLATERAL_DECIMALS = 6;
    const value =
      (Number(yesBal) * yesPrice + Number(noBal) * (1 - yesPrice)) /
      10 ** COLLATERAL_DECIMALS;
    totalValue += value;
    positions.push({
      registration: reg,
      market: m,
      pool: p,
      yesBalance: yesBal,
      noBalance: noBal,
      yesPrice,
      value,
    });
  });

  return { positions, totalValue };
}

// Local decode helpers (avoid circular imports with janus.ts).

function decodeMarketBuf(pubkey: PublicKey, data: Buffer): MarketState | null {
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

function decodePoolBuf(pubkey: PublicKey, data: Buffer): PoolState | null {
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

// Re-export for type imports.
export type { MarketState, PoolState, MarketRegistration };
