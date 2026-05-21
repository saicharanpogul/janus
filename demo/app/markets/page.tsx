"use client";

import { useConnection } from "@solana/wallet-adapter-react";
import Link from "next/link";
import { useEffect, useState, useMemo } from "react";
import { PublicKey } from "@solana/web3.js";

import {
  fetchAllRegistrations,
  fetchPool,
  fetchMarket,
  formatPubkey,
  formatPct,
  impliedYesProbability,
  questionHashToHex,
  statusLabel,
  type MarketRegistration,
  type MarketState,
  type PoolState,
} from "@/lib/janus";

interface MarketRow {
  reg: MarketRegistration;
  market: MarketState;
  pool: PoolState;
}

export default function MarketsPage() {
  const { connection } = useConnection();
  const [rows, setRows] = useState<MarketRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<"all" | "active" | "resolved">("all");
  const [sortBy, setSortBy] = useState<"created" | "deadline" | "yes_price">("created");

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    (async () => {
      const regs = await fetchAllRegistrations(connection);
      const marketAddrs = regs.map((r) => r.market);
      const poolAddrs = regs.map((r) => r.pool);
      const [marketInfos, poolInfos] = await Promise.all([
        connection.getMultipleAccountsInfo(marketAddrs, "confirmed"),
        connection.getMultipleAccountsInfo(poolAddrs, "confirmed"),
      ]);
      const rows: MarketRow[] = [];
      regs.forEach((reg, i) => {
        const mi = marketInfos[i];
        const pi = poolInfos[i];
        if (!mi || !pi) return;
        const market = decodeMarket(marketAddrs[i], mi.data);
        const pool = decodePool(poolAddrs[i], pi.data);
        if (!market || !pool) return;
        rows.push({ reg, market, pool });
      });
      if (!cancelled) setRows(rows);
      setLoading(false);
    })();
    return () => {
      cancelled = true;
    };
  }, [connection]);

  const filtered = useMemo(() => {
    let r = rows;
    if (statusFilter === "active") r = r.filter((x) => x.market.status === 0);
    if (statusFilter === "resolved") r = r.filter((x) => x.market.status > 0);
    if (query.trim()) {
      const q = query.toLowerCase();
      r = r.filter((x) =>
        [x.market.market.toBase58(), x.reg.creator.toBase58(), questionHashToHex(x.reg.questionHash)]
          .join(" ")
          .toLowerCase()
          .includes(q),
      );
    }
    if (sortBy === "created") {
      r = [...r].sort((a, b) => Number(b.reg.createdAtSlot - a.reg.createdAtSlot));
    } else if (sortBy === "deadline") {
      r = [...r].sort((a, b) => Number(a.market.deadlineSlot - b.market.deadlineSlot));
    } else if (sortBy === "yes_price") {
      r = [...r].sort((a, b) => impliedYesProbability(b.pool) - impliedYesProbability(a.pool));
    }
    return r;
  }, [rows, query, statusFilter, sortBy]);

  return (
    <div className="max-w-7xl mx-auto px-6 py-8">
      <div className="flex items-center gap-6 border-b border-line mb-6 -mt-2">
        <Link href="/" className="text-sm text-muted hover:text-black pb-3">Portfolio</Link>
        <div className="text-sm font-semibold pb-3 border-b-2 border-black">All Markets</div>
      </div>

      {/* Toolbar */}
      <div className="flex flex-wrap gap-3 items-center justify-between mb-6">
        <div className="flex gap-3 items-center flex-1 max-w-2xl">
          <input
            className="input flex-1 max-w-md"
            placeholder="Search by market address, creator, or question hash…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <select
            className="input w-32"
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value as "all" | "active" | "resolved")}
          >
            <option value="all">All</option>
            <option value="active">Active</option>
            <option value="resolved">Resolved</option>
          </select>
          <select
            className="input w-40"
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as "created" | "deadline" | "yes_price")}
          >
            <option value="created">Newest first</option>
            <option value="deadline">Deadline ↑</option>
            <option value="yes_price">YES price ↓</option>
          </select>
        </div>
        <Link href="/create" className="btn-primary inline-flex items-center">
          + Create market
        </Link>
      </div>

      {/* Table */}
      <div className="border-t border-b border-line-strong overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="border-b border-line">
            <tr>
              <th className="table-header text-left">▲ MARKET</th>
              <th className="table-header text-right">YES PRICE</th>
              <th className="table-header text-right">LIQUIDITY</th>
              <th className="table-header text-right">FEE</th>
              <th className="table-header text-left">STATUS</th>
              <th className="table-header text-right">DEADLINE</th>
              <th className="table-header text-right">CREATOR</th>
              <th className="table-header text-right"></th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((r) => (
              <MarketRowComp key={r.market.market.toBase58()} r={r} />
            ))}
            {!loading && filtered.length === 0 && (
              <tr>
                <td colSpan={8} className="text-center py-16 text-muted text-sm">
                  No markets {query ? "match your filter" : "yet. Be the first to create one."}
                </td>
              </tr>
            )}
            {loading && (
              <tr>
                <td colSpan={8} className="text-center py-16 text-muted text-sm">
                  Scanning chain…
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="mt-4 text-xs text-muted">
        {filtered.length} {filtered.length === 1 ? "market" : "markets"} shown
      </div>
    </div>
  );
}

function MarketRowComp({ r }: { r: MarketRow }) {
  const yesPrice = impliedYesProbability(r.pool);
  const liquidity = (Number(r.pool.yesReserves) + Number(r.pool.noReserves)) / 1e6;
  return (
    <tr className="border-b border-line hover:bg-canvas-alt cursor-pointer">
      <td className="table-cell">
        <Link href={`/markets/${r.market.market.toBase58()}`} className="block">
          <div className="font-mono font-semibold">{formatPubkey(r.market.market, 6).toUpperCase()}</div>
          <div className="text-[10px] text-muted">
            {questionHashToHex(r.reg.questionHash) ? `Q ${questionHashToHex(r.reg.questionHash).slice(0, 10)}…` : "no question hash"}
          </div>
        </Link>
      </td>
      <td className="table-cell text-right">
        <div className="font-semibold tabular-nums">{formatPct(yesPrice, 1)}</div>
        <div className="text-[10px] text-muted tabular-nums">NO {formatPct(1 - yesPrice, 1)}</div>
      </td>
      <td className="table-cell text-right tabular-nums">{liquidity.toFixed(2)}</td>
      <td className="table-cell text-right tabular-nums">{(r.pool.feeBps / 100).toFixed(2)}%</td>
      <td className="table-cell">
        <StatusBadge status={r.market.status} />
      </td>
      <td className="table-cell text-right tabular-nums text-muted">
        {r.market.deadlineSlot.toString()}
      </td>
      <td className="table-cell text-right font-mono text-muted text-xs">
        {formatPubkey(r.reg.creator, 4)}
      </td>
      <td className="table-cell text-right">
        <Link href={`/markets/${r.market.market.toBase58()}`} className="text-xs font-semibold hover:underline">
          TRADE →
        </Link>
      </td>
    </tr>
  );
}

function StatusBadge({ status }: { status: number }) {
  const label = statusLabel(status);
  let cls = "bg-canvas-alt text-muted";
  if (status === 0) cls = "bg-canvas-alt text-ink border-line-strong";
  if (status === 1) cls = "bg-gain-soft text-ink";
  if (status === 2) cls = "bg-loss-soft text-ink";
  if (status === 3) cls = "bg-line text-ink";
  return (
    <span className={`text-[10px] uppercase tracking-wider font-semibold px-2 py-0.5 border ${cls}`}>
      {label}
    </span>
  );
}

// Local decoders (duplicated from janus.ts because client-side modules can't
// re-export them from the server boundary without a chain of "use client"s).

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
