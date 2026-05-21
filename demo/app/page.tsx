"use client";

import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import dynamic from "next/dynamic";
import Link from "next/link";
import { useEffect, useState } from "react";

import { fetchUserPositions, type Position } from "@/lib/positions";
import {
  formatPct,
  formatPubkey,
  questionHashToHex,
  statusLabel,
} from "@/lib/janus";

const WalletMultiButton = dynamic(
  async () =>
    (await import("@solana/wallet-adapter-react-ui")).WalletMultiButton,
  { ssr: false },
);

export default function PortfolioPage() {
  const { connection } = useConnection();
  const { publicKey } = useWallet();
  const [positions, setPositions] = useState<Position[]>([]);
  const [totalValue, setTotalValue] = useState(0);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (!publicKey) {
      setPositions([]);
      setTotalValue(0);
      return;
    }
    setLoading(true);
    fetchUserPositions(connection, publicKey)
      .then(({ positions, totalValue }) => {
        if (cancelled) return;
        setPositions(positions);
        setTotalValue(totalValue);
      })
      .catch(console.error)
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [connection, publicKey]);

  if (!publicKey) {
    return <ConnectPrompt />;
  }

  // Aggregate stats (Bloomberg hero panel)
  const totalCost = positions.length * 0; // unknown — we'd need a trade log
  const dayGain = 0; // ditto
  const totalGain = totalValue - totalCost;
  const totalGainPct = totalCost === 0 ? 0 : totalGain / totalCost;

  return (
    <div className="max-w-7xl mx-auto px-6 py-8">
      {/* Section tabs (Bloomberg-style underlines) */}
      <div className="flex items-center gap-6 border-b border-line mb-6 -mt-2">
        <div className="text-sm font-semibold pb-3 border-b-2 border-black">
          Portfolio
        </div>
        <Link
          href="/markets"
          className="text-sm text-muted hover:text-black pb-3"
        >
          All Markets
        </Link>
      </div>

      {/* Hero stat row */}
      <section className="grid grid-cols-12 gap-6 items-end mb-8">
        <div className="col-span-12 md:col-span-7 pl-4 relative">
          <span className="absolute left-0 top-1 bottom-1 w-1 bg-gain-soft" />
          <div className="stat-label mb-2">TOTAL VALUE</div>
          <div className="text-hero tabular-nums leading-none">
            <span>${totalValue.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</span>
            <span className="text-xl ml-2 align-bottom text-muted">USD</span>
          </div>
          <div className="mt-2 text-sm">
            <span className="bg-gain-soft px-2 py-0.5 mr-2 font-semibold text-ink">▲ {formatPct(totalGainPct, 2)}</span>
            <span className="text-muted">{positions.length} positions • {loading ? "syncing…" : "live"}</span>
          </div>
        </div>

        <StatCell label="DAY GAIN" value={`$${dayGain.toFixed(2)}`} positive={dayGain >= 0} />
        <StatCell label="POSITIONS" value={positions.length.toString()} />
        <StatCell label="UNREALIZED" value={`$${totalGain.toFixed(2)}`} positive={totalGain >= 0} />

        <div className="col-span-12 md:col-span-1 flex md:flex-col gap-2 md:items-end pl-2">
          <Link href="/markets" className="btn-outline w-full md:w-32 flex items-center justify-center">
            Trade
          </Link>
        </div>
      </section>

      {/* Positions table */}
      <section>
        {positions.length === 0 ? (
          <EmptyState loading={loading} />
        ) : (
          <PositionsTable positions={positions} />
        )}
      </section>
    </div>
  );
}

function ConnectPrompt() {
  return (
    <div className="max-w-7xl mx-auto px-6 py-24">
      <div className="max-w-2xl">
        <div className="text-xs uppercase tracking-widest text-accent font-medium mb-3">
          Devnet · Read-only until connected
        </div>
        <h1 className="text-5xl font-extrabold tracking-tight mb-4 leading-tight">
          Onchain binary markets on Solana.
        </h1>
        <p className="text-lg text-muted mb-8 max-w-xl">
          Janus is a permissionless primitive for YES/NO markets. Pool
          mints conditional tokens; LMSR sets prices; resolvers are
          pluggable. Mechanized at every layer — 105+ Lean theorems, 107
          BMC harnesses, full Mollusk integration coverage.
        </p>
        <div className="flex gap-3">
          <WalletMultiButton />
          <Link href="/markets" className="btn-outline flex items-center">
            Browse markets
          </Link>
        </div>
      </div>
      <div className="mt-12 grid grid-cols-1 md:grid-cols-3 gap-px bg-line border border-line">
        <FeatureCard label="Bounded loss" value="b · log(2)" sub="Subsidizer max exposure, formally proven over ℝ in Lean" />
        <FeatureCard label="CU per swap" value="23.5K" sub="True-LMSR Q32.32 pricing, well under 50K budget" />
        <FeatureCard label="Mechanized" value="105+" sub="Lean theorems across 6 programs, axiom-free" />
      </div>
    </div>
  );
}

function FeatureCard({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <div className="bg-white px-6 py-6">
      <div className="text-[10px] uppercase tracking-wider text-muted font-medium">{label}</div>
      <div className="text-3xl font-extrabold tracking-tight mt-1 tabular-nums">{value}</div>
      <div className="text-xs text-muted mt-2">{sub}</div>
    </div>
  );
}

function StatCell({ label, value, positive }: { label: string; value: string; positive?: boolean }) {
  return (
    <div className="col-span-6 md:col-span-1 pl-3 relative">
      <span className={`absolute left-0 top-1 bottom-1 w-0.5 ${positive === undefined ? "bg-line-strong" : positive ? "bg-gain-soft" : "bg-loss-soft"}`} />
      <div className="stat-label">{label}</div>
      <div className="text-2xl font-bold mt-1 tabular-nums">{value}</div>
    </div>
  );
}

function PositionsTable({ positions }: { positions: Position[] }) {
  return (
    <div className="border-t border-b border-line-strong overflow-x-auto">
      <table className="w-full text-sm">
        <thead className="border-b border-line">
          <tr>
            <th className="table-header text-left">▲ SYMBOL</th>
            <th className="table-header text-right">YES BAL</th>
            <th className="table-header text-right">NO BAL</th>
            <th className="table-header text-right">YES PRICE</th>
            <th className="table-header text-right">VALUE</th>
            <th className="table-header text-left">STATUS</th>
            <th className="table-header text-right">DEADLINE</th>
            <th className="table-header text-right"></th>
          </tr>
        </thead>
        <tbody>
          {positions.map((p) => (
            <PositionRow key={p.market.market.toBase58()} p={p} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

function PositionRow({ p }: { p: Position }) {
  const sym = questionHashToHex(p.registration.questionHash).slice(0, 10) || formatPubkey(p.market.market, 4);
  const yesPct = p.yesPrice;
  return (
    <tr className="border-b border-line hover:bg-canvas-alt">
      <td className="table-cell font-semibold">
        <div className="font-mono">{sym.toUpperCase()}</div>
        <div className="text-[10px] text-muted">{formatPubkey(p.market.market, 6)}</div>
      </td>
      <td className="table-cell text-right">
        {p.yesBalance === 0n ? "—" : Number(p.yesBalance).toLocaleString()}
      </td>
      <td className="table-cell text-right">
        {p.noBalance === 0n ? "—" : Number(p.noBalance).toLocaleString()}
      </td>
      <td className="table-cell text-right">
        <span className="font-semibold">{formatPct(yesPct, 1)}</span>
        <div className="text-[10px] text-muted">NO {formatPct(1 - yesPct, 1)}</div>
      </td>
      <td className="table-cell text-right font-semibold">${p.value.toFixed(4)}</td>
      <td className="table-cell">
        <StatusBadge status={p.market.status} />
      </td>
      <td className="table-cell text-right tabular-nums">
        slot {p.market.deadlineSlot.toString()}
      </td>
      <td className="table-cell text-right">
        <Link href={`/markets/${p.market.market.toBase58()}`} className="text-xs font-semibold hover:underline">
          OPEN →
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

function EmptyState({ loading }: { loading: boolean }) {
  return (
    <div className="border border-line bg-canvas-alt py-16 text-center">
      <div className="text-sm text-muted">
        {loading ? "Scanning chain for your positions…" : "No positions yet."}
      </div>
      {!loading && (
        <div className="mt-4 flex justify-center gap-3">
          <Link href="/markets" className="btn-primary inline-flex items-center">
            Browse markets
          </Link>
          <Link href="/create" className="btn-outline inline-flex items-center">
            Create market
          </Link>
        </div>
      )}
    </div>
  );
}
