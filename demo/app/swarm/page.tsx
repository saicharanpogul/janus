"use client";

import { useConnection } from "@solana/wallet-adapter-react";
import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { AccountLayout } from "@solana/spl-token";

import {
  fetchAllRegistrations,
  formatPct,
  formatPubkey,
  questionHashToHex,
  statusLabel,
  type MarketRegistration,
} from "@/lib/janus";

interface MarketWithState extends MarketRegistration {
  yesPrice: number;
  liquidity: number;
  poolType: "cpmm" | "true-lmsr" | "unknown";
  status: number;
  vaultBalance: number; // collateral USDC
}

const CPMM_PROGRAM = "GUwcYfYGqR6WPduoB6gEEZoPG6vdAAK7gK1xP6eTJ3JK";
const TRUE_LMSR_PROGRAM = "HrFV8Nfncv2gekc9jZPC6rXxnVVaUQi75BmwVFzd5fjQ";

export default function SwarmPage() {
  const { connection } = useConnection();
  const [markets, setMarkets] = useState<MarketWithState[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    (async () => {
      const regs = await fetchAllRegistrations(connection);
      if (regs.length === 0) {
        if (!cancelled) {
          setMarkets([]);
          setLoading(false);
        }
        return;
      }
      const [marketInfos, poolInfos] = await Promise.all([
        connection.getMultipleAccountsInfo(regs.map((r) => r.market), "confirmed"),
        connection.getMultipleAccountsInfo(regs.map((r) => r.pool), "confirmed"),
      ]);

      // For each market we need the conditional-tokens vault balance.
      const vaultKeys: PublicKey[] = [];
      regs.forEach((_, i) => {
        const info = marketInfos[i];
        if (!info || info.data.length !== 248) {
          vaultKeys.push(PublicKey.default);
          return;
        }
        vaultKeys.push(new PublicKey(info.data.subarray(104, 136)));
      });
      const vaultInfos = await connection.getMultipleAccountsInfo(
        vaultKeys.map((k) => (k.equals(PublicKey.default) ? regs[0].market : k)),
        "confirmed",
      );

      const results: MarketWithState[] = regs.map((reg, i) => {
        const mInfo = marketInfos[i];
        const pInfo = poolInfos[i];
        const vInfo = vaultInfos[i];
        let yesPrice = 0.5;
        let liquidity = 0;
        let poolType: "cpmm" | "true-lmsr" | "unknown" = "unknown";
        let status = -1;
        let vaultBalance = 0;
        if (mInfo && mInfo.data.length === 248) {
          status = mInfo.data[1];
        }
        if (vInfo && vInfo.data.length >= 72) {
          try {
            vaultBalance =
              Number(AccountLayout.decode(vInfo.data).amount.toString()) / 1e6;
          } catch {
            /* not a token account */
          }
        }
        if (pInfo) {
          const owner = pInfo.owner.toBase58();
          if (owner === CPMM_PROGRAM && pInfo.data.length === 224) {
            poolType = "cpmm";
            const yes = pInfo.data.readBigUInt64LE(200);
            const no = pInfo.data.readBigUInt64LE(208);
            const total = yes + no;
            yesPrice = total === 0n ? 0.5 : Number(no) / Number(total);
            liquidity = (Number(yes) + Number(no)) / 1e6;
          } else if (owner === TRUE_LMSR_PROGRAM && pInfo.data.length === 264) {
            poolType = "true-lmsr";
            const b = Number(pInfo.data.readBigUInt64LE(232));
            const qYes = Number(pInfo.data.readBigUInt64LE(240));
            const qNo = Number(pInfo.data.readBigUInt64LE(248));
            // YES price via log-sum-exp.
            const a = qYes / b;
            const c = qNo / b;
            const m = Math.max(a, c);
            const ea = Math.exp(a - m);
            const ec = Math.exp(c - m);
            yesPrice = b > 0 ? ea / (ea + ec) : 0.5;
            liquidity = vaultBalance;
          }
        }
        return { ...reg, yesPrice, liquidity, poolType, status, vaultBalance };
      });

      if (!cancelled) {
        setMarkets(results);
        setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [connection, refreshKey]);

  // Aggregate stats
  const stats = useMemo(() => {
    const creators = new Map<string, MarketWithState[]>();
    for (const m of markets) {
      const k = m.creator.toBase58();
      if (!creators.has(k)) creators.set(k, []);
      creators.get(k)!.push(m);
    }
    const active = markets.filter((m) => m.status === 0).length;
    const resolved = markets.filter((m) => m.status > 0 && m.status < 4).length;
    const totalLiquidity = markets.reduce((s, m) => s + m.liquidity, 0);
    const cpmmCount = markets.filter((m) => m.poolType === "cpmm").length;
    const lmsrCount = markets.filter((m) => m.poolType === "true-lmsr").length;
    return {
      totalMarkets: markets.length,
      activeMarkets: active,
      resolvedMarkets: resolved,
      uniqueCreators: creators.size,
      totalLiquidity,
      cpmmCount,
      lmsrCount,
      creators,
    };
  }, [markets]);

  // Per-creator leaderboard (sorted by markets created desc)
  const leaderboard = useMemo(() => {
    return Array.from(stats.creators.entries())
      .map(([creator, ms]) => ({
        creator,
        marketCount: ms.length,
        totalLiquidity: ms.reduce((s, m) => s + m.liquidity, 0),
        activeMarkets: ms.filter((m) => m.status === 0).length,
        avgYesPrice: ms.reduce((s, m) => s + m.yesPrice, 0) / ms.length,
        lastActiveSlot: ms.reduce(
          (s, m) => (m.createdAtSlot > s ? m.createdAtSlot : s),
          0n,
        ),
      }))
      .sort((a, b) => b.marketCount - a.marketCount);
  }, [stats]);

  return (
    <div className="max-w-7xl mx-auto px-6 py-8">
      <div className="flex items-center gap-6 border-b border-line mb-6 -mt-2">
        <Link href="/" className="text-sm text-muted hover:text-black pb-3">Portfolio</Link>
        <Link href="/markets" className="text-sm text-muted hover:text-black pb-3">All Markets</Link>
        <Link href="/create" className="text-sm text-muted hover:text-black pb-3">Create</Link>
        <div className="text-sm font-semibold pb-3 border-b-2 border-black">Swarm</div>
      </div>

      <div className="flex items-baseline justify-between mb-8">
        <div>
          <div className="text-xs uppercase tracking-widest text-accent font-medium mb-2">
            Devnet · live chain state
          </div>
          <h1 className="text-3xl font-extrabold tracking-tight">Swarm dashboard</h1>
          <p className="text-sm text-muted mt-1">
            Aggregate view of every market and every creator on the protocol.
            Refresh to repaint from chain.
          </p>
        </div>
        <button
          onClick={() => setRefreshKey((k) => k + 1)}
          className="btn-outline"
        >
          {loading ? "Loading…" : "Refresh"}
        </button>
      </div>

      {/* Stat grid */}
      <section className="grid grid-cols-2 md:grid-cols-6 gap-px bg-line border border-line mb-8">
        <StatTile label="MARKETS" value={stats.totalMarkets.toString()} />
        <StatTile label="ACTIVE" value={stats.activeMarkets.toString()} highlight="gain" />
        <StatTile label="RESOLVED" value={stats.resolvedMarkets.toString()} highlight="muted" />
        <StatTile label="CREATORS" value={stats.uniqueCreators.toString()} />
        <StatTile label="TVL" value={`$${stats.totalLiquidity.toFixed(0)}`} />
        <StatTile label="CPMM / LMSR" value={`${stats.cpmmCount} / ${stats.lmsrCount}`} />
      </section>

      {/* Leaderboard */}
      <section>
        <h2 className="text-xs uppercase tracking-wider text-muted font-medium mb-3">
          Creator leaderboard
        </h2>
        <div className="border-t border-b border-line-strong overflow-x-auto mb-8">
          <table className="w-full text-sm">
            <thead className="border-b border-line">
              <tr>
                <th className="table-header text-left">RANK</th>
                <th className="table-header text-left">CREATOR</th>
                <th className="table-header text-right">MARKETS</th>
                <th className="table-header text-right">ACTIVE</th>
                <th className="table-header text-right">LIQUIDITY</th>
                <th className="table-header text-right">AVG YES PRICE</th>
                <th className="table-header text-right">LAST ACTIVE</th>
              </tr>
            </thead>
            <tbody>
              {leaderboard.map((c, i) => (
                <tr key={c.creator} className="border-b border-line hover:bg-canvas-alt">
                  <td className="table-cell font-bold tabular-nums">{i + 1}</td>
                  <td className="table-cell font-mono text-xs">
                    {formatPubkey(c.creator, 6)}
                  </td>
                  <td className="table-cell text-right tabular-nums font-semibold">
                    {c.marketCount}
                  </td>
                  <td className="table-cell text-right tabular-nums">
                    {c.activeMarkets}
                  </td>
                  <td className="table-cell text-right tabular-nums">
                    ${c.totalLiquidity.toFixed(2)}
                  </td>
                  <td className="table-cell text-right tabular-nums">
                    {formatPct(c.avgYesPrice, 1)}
                  </td>
                  <td className="table-cell text-right tabular-nums text-muted text-xs">
                    slot {c.lastActiveSlot.toString()}
                  </td>
                </tr>
              ))}
              {leaderboard.length === 0 && !loading && (
                <tr>
                  <td colSpan={7} className="text-center py-12 text-muted text-sm">
                    No markets yet. Run the agent toolkit's spawn-swarm example.
                  </td>
                </tr>
              )}
              {loading && (
                <tr>
                  <td colSpan={7} className="text-center py-12 text-muted text-sm">
                    Scanning chain…
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>

      {/* Recent markets */}
      <section>
        <h2 className="text-xs uppercase tracking-wider text-muted font-medium mb-3">
          Recent markets ({markets.length})
        </h2>
        <div className="border-t border-b border-line-strong overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="border-b border-line">
              <tr>
                <th className="table-header text-left">MARKET</th>
                <th className="table-header text-left">POOL</th>
                <th className="table-header text-right">YES PRICE</th>
                <th className="table-header text-right">LIQUIDITY</th>
                <th className="table-header text-left">STATUS</th>
                <th className="table-header text-right">CREATOR</th>
                <th className="table-header text-right">DEADLINE</th>
              </tr>
            </thead>
            <tbody>
              {markets
                .sort((a, b) => Number(b.createdAtSlot - a.createdAtSlot))
                .slice(0, 30)
                .map((m) => (
                  <tr
                    key={m.market.toBase58()}
                    className="border-b border-line hover:bg-canvas-alt"
                  >
                    <td className="table-cell">
                      <Link
                        href={`/markets/${m.market.toBase58()}`}
                        className="block font-mono text-xs"
                      >
                        {formatPubkey(m.market, 6)}
                      </Link>
                      <div className="text-[10px] text-muted">
                        {questionHashToHex(m.questionHash).slice(0, 10) || "—"}
                      </div>
                    </td>
                    <td className="table-cell">
                      <PoolBadge type={m.poolType} />
                    </td>
                    <td className="table-cell text-right tabular-nums">
                      {formatPct(m.yesPrice, 1)}
                    </td>
                    <td className="table-cell text-right tabular-nums">
                      ${m.liquidity.toFixed(2)}
                    </td>
                    <td className="table-cell">
                      <StatusBadge status={m.status} />
                    </td>
                    <td className="table-cell text-right font-mono text-xs text-muted">
                      {formatPubkey(m.creator, 4)}
                    </td>
                    <td className="table-cell text-right text-xs text-muted">
                      {m.deadlineSlot.toString()}
                    </td>
                  </tr>
                ))}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}

function StatTile({
  label,
  value,
  highlight,
}: {
  label: string;
  value: string;
  highlight?: "gain" | "muted";
}) {
  const bgClass =
    highlight === "gain"
      ? "bg-gain-soft"
      : highlight === "muted"
      ? "bg-canvas-alt"
      : "bg-white";
  return (
    <div className={`px-4 py-4 ${bgClass}`}>
      <div className="stat-label">{label}</div>
      <div className="text-2xl font-bold mt-1 tabular-nums">{value}</div>
    </div>
  );
}

function PoolBadge({ type }: { type: "cpmm" | "true-lmsr" | "unknown" }) {
  const cls =
    type === "true-lmsr"
      ? "bg-gain-soft text-ink"
      : type === "cpmm"
      ? "bg-canvas-alt text-ink border-line-strong"
      : "bg-line text-muted";
  return (
    <span className={`text-[10px] uppercase tracking-wider font-semibold px-2 py-0.5 border ${cls}`}>
      {type === "true-lmsr" ? "LMSR" : type === "cpmm" ? "CPMM" : "?"}
    </span>
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
