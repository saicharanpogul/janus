// Market discovery: scan the market-factory's registration PDAs and decode
// market + pool state for both CPMM and true-LMSR pools.

import { Connection, PublicKey } from "@solana/web3.js";
import {
  MARKET_FACTORY_PROGRAM_ID,
  LMSR_MARKET_PROGRAM_ID,
  LMSR_TRUE_MARKET_PROGRAM_ID,
} from "@janus/sdk";
import { priceYes as truePriceYes } from "@janus/sdk";

import type { MarketSnapshot, PoolType } from "./types.js";

const COLLATERAL_DECIMALS = 6;

// MarketRegistration = 216 bytes (factory state)
function decodeRegistration(pubkey: PublicKey, data: Buffer) {
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

// CPMM Pool = 224 bytes
function decodeCpmmPool(
  pubkey: PublicKey,
  data: Buffer,
): {
  yesReserves: bigint;
  noReserves: bigint;
  feeBps: number;
} | null {
  if (data.length !== 224) return null;
  return {
    yesReserves: data.readBigUInt64LE(200),
    noReserves: data.readBigUInt64LE(208),
    feeBps: data.readUInt16LE(216),
  };
}

// True-LMSR Pool = 264 bytes
function decodeTruePool(
  pubkey: PublicKey,
  data: Buffer,
): {
  bLiquidity: bigint;
  qYes: bigint;
  qNo: bigint;
  collateralVault: PublicKey;
  initialSubsidy: bigint;
} | null {
  if (data.length !== 264) return null;
  return {
    collateralVault: new PublicKey(data.subarray(136, 168)),
    bLiquidity: data.readBigUInt64LE(232),
    qYes: data.readBigUInt64LE(240),
    qNo: data.readBigUInt64LE(248),
    initialSubsidy: data.readBigUInt64LE(256),
  };
}

// CPMM implied YES probability.
function cpmmYesPrice(yes: bigint, no: bigint): number {
  const total = yes + no;
  if (total === 0n) return 0.5;
  return Number(no) / Number(total);
}

/**
 * Fetch every market the agent could potentially trade on. Decodes both
 * CPMM and true-LMSR pools and unifies them under a single MarketSnapshot.
 */
export async function snapshotAllMarkets(
  connection: Connection,
): Promise<MarketSnapshot[]> {
  const accounts = await connection.getProgramAccounts(
    MARKET_FACTORY_PROGRAM_ID,
    {
      filters: [{ dataSize: 216 }],
      commitment: "confirmed",
    },
  );
  const registrations = accounts
    .map((a) => decodeRegistration(a.pubkey, a.account.data))
    .filter((r): r is NonNullable<ReturnType<typeof decodeRegistration>> => r !== null);

  if (registrations.length === 0) return [];

  const marketKeys = registrations.map((r) => r.market);
  const poolKeys = registrations.map((r) => r.pool);
  const [marketInfos, poolInfos] = await Promise.all([
    connection.getMultipleAccountsInfo(marketKeys, "confirmed"),
    connection.getMultipleAccountsInfo(poolKeys, "confirmed"),
  ]);

  // For true-LMSR pools, we also need to fetch the collateral vault balance.
  const vaultAddrsByIdx: { idx: number; vault: PublicKey }[] = [];
  poolInfos.forEach((info, i) => {
    if (!info) return;
    const owner = info.owner.toBase58();
    if (owner !== LMSR_TRUE_MARKET_PROGRAM_ID.toBase58()) return;
    const decoded = decodeTruePool(poolKeys[i], info.data);
    if (decoded) vaultAddrsByIdx.push({ idx: i, vault: decoded.collateralVault });
  });
  const vaultInfos =
    vaultAddrsByIdx.length > 0
      ? await connection.getMultipleAccountsInfo(
          vaultAddrsByIdx.map((v) => v.vault),
          "confirmed",
        )
      : [];
  const vaultBalances = new Map<number, bigint>();
  vaultAddrsByIdx.forEach((entry, j) => {
    const info = vaultInfos[j];
    if (!info) return;
    // SPL token account amount is at offset 64 (u64 LE).
    const amt = info.data.readBigUInt64LE(64);
    vaultBalances.set(entry.idx, amt);
  });

  const snapshots: MarketSnapshot[] = [];
  registrations.forEach((reg, i) => {
    const mInfo = marketInfos[i];
    const pInfo = poolInfos[i];
    if (!mInfo || !pInfo || mInfo.data.length < 248) return;

    const status = mInfo.data[1];
    const poolOwner = pInfo.owner.toBase58();

    let poolType: PoolType;
    let yesReserves = 0n;
    let noReserves = 0n;
    let yesPrice = 0.5;
    let feeBps = 0;
    let bLiquidity: bigint | undefined;
    let collateralVaultBalance: bigint | undefined;
    let liquidity = 0;

    if (poolOwner === LMSR_MARKET_PROGRAM_ID.toBase58()) {
      const pool = decodeCpmmPool(poolKeys[i], pInfo.data);
      if (!pool) return;
      poolType = "cpmm";
      yesReserves = pool.yesReserves;
      noReserves = pool.noReserves;
      feeBps = pool.feeBps;
      yesPrice = cpmmYesPrice(yesReserves, noReserves);
      liquidity =
        (Number(yesReserves) + Number(noReserves)) / 10 ** COLLATERAL_DECIMALS;
    } else if (poolOwner === LMSR_TRUE_MARKET_PROGRAM_ID.toBase58()) {
      const pool = decodeTruePool(poolKeys[i], pInfo.data);
      if (!pool) return;
      poolType = "true-lmsr";
      yesReserves = pool.qYes;
      noReserves = pool.qNo;
      bLiquidity = pool.bLiquidity;
      collateralVaultBalance = vaultBalances.get(i) ?? 0n;
      yesPrice = truePriceYes(
        Number(bLiquidity),
        Number(yesReserves),
        Number(noReserves),
      );
      liquidity = Number(collateralVaultBalance) / 10 ** COLLATERAL_DECIMALS;
    } else {
      // Unknown pool owner — skip.
      return;
    }

    snapshots.push({
      market: reg.market,
      pool: reg.pool,
      poolType,
      status,
      creator: reg.creator,
      questionHash: reg.questionHash,
      deadlineSlot: reg.deadlineSlot,
      createdAtSlot: reg.createdAtSlot,
      yesPrice,
      liquidity,
      feeBps,
      yesReserves,
      noReserves,
      bLiquidity,
      collateralVaultBalance,
    });
  });

  return snapshots;
}
