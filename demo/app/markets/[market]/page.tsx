"use client";

import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import {
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountIdempotentInstruction,
  AccountLayout,
} from "@solana/spl-token";

import {
  fetchMarket,
  fetchPool,
  formatPct,
  formatPubkey,
  impliedYesProbability,
  quoteSwap,
  statusLabel,
  type MarketState,
  type PoolState,
} from "@/lib/janus";
import {
  swapIx,
  splitIx,
  type SwapDirection,
} from "@janus/sdk";

const COLLATERAL_DECIMALS = 6; // USDC-style

export default function MarketDetail() {
  const { connection } = useConnection();
  const wallet = useWallet();
  const params = useParams<{ market: string }>();
  const marketAddr = params.market;

  const [market, setMarket] = useState<MarketState | null>(null);
  const [pool, setPool] = useState<PoolState | null>(null);
  const [userYes, setUserYes] = useState<bigint>(0n);
  const [userNo, setUserNo] = useState<bigint>(0n);
  const [userCollateral, setUserCollateral] = useState<bigint>(0n);
  const [refreshKey, setRefreshKey] = useState(0);

  const reload = useCallback(() => setRefreshKey((k) => k + 1), []);

  useEffect(() => {
    let cancelled = false;
    if (!marketAddr) return;
    (async () => {
      const m = await fetchMarket(connection, new PublicKey(marketAddr));
      if (cancelled || !m) return;
      setMarket(m);
      // Derive pool from market and load it.
      const [poolPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("pool"), m.market.toBuffer()],
        new PublicKey("GUwcYfYGqR6WPduoB6gEEZoPG6vdAAK7gK1xP6eTJ3JK"),
      );
      const p = await fetchPool(connection, poolPda);
      if (cancelled) return;
      setPool(p);

      // Load user token balances.
      if (wallet.publicKey) {
        const [ataYes, ataNo, ataColl] = [m.yesMint, m.noMint, m.collateralMint].map((mint) =>
          getAssociatedTokenAddressSync(mint, wallet.publicKey!, true),
        );
        const infos = await connection.getMultipleAccountsInfo([ataYes, ataNo, ataColl]);
        if (cancelled) return;
        const decode = (i: number): bigint => {
          if (!infos[i]) return 0n;
          try {
            return BigInt(AccountLayout.decode(infos[i]!.data).amount.toString());
          } catch {
            return 0n;
          }
        };
        setUserYes(decode(0));
        setUserNo(decode(1));
        setUserCollateral(decode(2));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [connection, marketAddr, wallet.publicKey, refreshKey]);

  if (!market || !pool) {
    return (
      <div className="max-w-7xl mx-auto px-6 py-24 text-center text-muted">
        Loading market…
      </div>
    );
  }

  const yesPrice = impliedYesProbability(pool);
  const liquidity = (Number(pool.yesReserves) + Number(pool.noReserves)) / 1e6;

  return (
    <div className="max-w-7xl mx-auto px-6 py-8">
      <Link href="/markets" className="text-xs text-muted hover:text-black">← All markets</Link>

      {/* Hero */}
      <div className="grid grid-cols-12 gap-6 mt-4 mb-8 items-end">
        <div className="col-span-12 md:col-span-8 pl-4 relative">
          <span className="absolute left-0 top-1 bottom-1 w-1 bg-gain-soft" />
          <div className="stat-label mb-2">YES PROBABILITY</div>
          <div className="text-hero tabular-nums leading-none">
            {formatPct(yesPrice, 2)}
          </div>
          <div className="text-sm mt-2 text-muted">
            <span className="font-mono">{market.market.toBase58()}</span> ·{" "}
            <StatusBadge status={market.status} />
          </div>
        </div>
        <StatCell label="NO PRICE" value={formatPct(1 - yesPrice, 2)} />
        <StatCell label="LIQUIDITY" value={`$${liquidity.toFixed(2)}`} />
        <StatCell label="FEE" value={`${(pool.feeBps / 100).toFixed(2)}%`} />
      </div>

      {/* Two-column: orderbook-ish info + trade form */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-px bg-line border border-line">
        <DetailsPanel market={market} pool={pool} />
        <UserPositionPanel
          userYes={userYes}
          userNo={userNo}
          userCollateral={userCollateral}
          yesPrice={yesPrice}
        />
        <TradePanel
          market={market}
          pool={pool}
          userYes={userYes}
          userNo={userNo}
          userCollateral={userCollateral}
          onSuccess={reload}
        />
      </div>
    </div>
  );
}

function DetailsPanel({ market, pool }: { market: MarketState; pool: PoolState }) {
  const rows: [string, string][] = [
    ["Status", statusLabel(market.status)],
    ["Deadline slot", market.deadlineSlot.toString()],
    ["Created at slot", market.createdAtSlot.toString()],
    ["Collateral mint", formatPubkey(market.collateralMint, 6)],
    ["YES mint", formatPubkey(market.yesMint, 6)],
    ["NO mint", formatPubkey(market.noMint, 6)],
    ["Authority", formatPubkey(market.authority, 6)],
    ["Resolver program", formatPubkey(market.resolverProgram, 6)],
    ["Resolver state", formatPubkey(market.resolverState, 6)],
    ["Pool YES reserves", (Number(pool.yesReserves) / 1e6).toFixed(2)],
    ["Pool NO reserves", (Number(pool.noReserves) / 1e6).toFixed(2)],
  ];
  return (
    <div className="bg-white p-6">
      <div className="stat-label mb-4">MARKET DETAILS</div>
      <div className="space-y-2">
        {rows.map(([k, v]) => (
          <div key={k} className="flex justify-between text-xs">
            <span className="text-muted">{k}</span>
            <span className="font-mono tabular-nums">{v}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function UserPositionPanel({
  userYes,
  userNo,
  userCollateral,
  yesPrice,
}: {
  userYes: bigint;
  userNo: bigint;
  userCollateral: bigint;
  yesPrice: number;
}) {
  const wallet = useWallet();
  if (!wallet.publicKey) {
    return (
      <div className="bg-white p-6 flex flex-col items-center justify-center text-sm text-muted">
        Connect your wallet to see balances.
      </div>
    );
  }
  const value =
    (Number(userYes) * yesPrice + Number(userNo) * (1 - yesPrice)) / 1e6;
  return (
    <div className="bg-white p-6">
      <div className="stat-label mb-4">YOUR POSITION</div>
      <div className="space-y-3 text-sm">
        <RowPair k="Collateral" v={(Number(userCollateral) / 1e6).toFixed(2)} />
        <RowPair k="YES balance" v={Number(userYes).toLocaleString()} />
        <RowPair k="NO balance" v={Number(userNo).toLocaleString()} />
        <hr className="border-line" />
        <RowPair k="Mark value" v={`$${value.toFixed(4)}`} bold />
      </div>
    </div>
  );
}

function RowPair({ k, v, bold }: { k: string; v: string; bold?: boolean }) {
  return (
    <div className="flex justify-between">
      <span className="text-muted">{k}</span>
      <span className={`font-mono tabular-nums ${bold ? "font-semibold" : ""}`}>{v}</span>
    </div>
  );
}

type TradeAction = "split" | "swap_y2n" | "swap_n2y";

function TradePanel({
  market,
  pool,
  userYes,
  userNo,
  userCollateral,
  onSuccess,
}: {
  market: MarketState;
  pool: PoolState;
  userYes: bigint;
  userNo: bigint;
  userCollateral: bigint;
  onSuccess: () => void;
}) {
  const { connection } = useConnection();
  const wallet = useWallet();
  const [action, setAction] = useState<TradeAction>("split");
  const [amountStr, setAmountStr] = useState("");
  const [slippageBps, setSlippageBps] = useState(100); // 1%
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const amount = parseAmount(amountStr);
  const isActive = market.status === 0;

  const quote =
    action === "split"
      ? null
      : action === "swap_y2n"
      ? quoteSwap(pool, "yesToNo", amount)
      : quoteSwap(pool, "noToYes", amount);

  const minOut =
    quote === null
      ? 0n
      : (quote.amountOut * BigInt(10000 - slippageBps)) / 10000n;

  const submit = useCallback(async () => {
    if (!wallet.publicKey || !wallet.signTransaction) return;
    if (amount === 0n) {
      setError("Enter a non-zero amount");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const ixs: TransactionInstruction[] = [];
      const owner = wallet.publicKey;

      const ataYes = getAssociatedTokenAddressSync(market.yesMint, owner, true);
      const ataNo = getAssociatedTokenAddressSync(market.noMint, owner, true);
      const ataCol = getAssociatedTokenAddressSync(market.collateralMint, owner, true);

      // Ensure ATAs exist (idempotent).
      ixs.push(createAssociatedTokenAccountIdempotentInstruction(owner, ataYes, owner, market.yesMint));
      ixs.push(createAssociatedTokenAccountIdempotentInstruction(owner, ataNo, owner, market.noMint));

      if (action === "split") {
        if (amount > userCollateral) {
          setError("Insufficient collateral balance");
          setBusy(false);
          return;
        }
        ixs.push(
          splitIx({
            user: owner,
            market: market.market,
            userCollateral: ataCol,
            vault: market.vault,
            yesMint: market.yesMint,
            noMint: market.noMint,
            userYes: ataYes,
            userNo: ataNo,
            amount,
          }),
        );
      } else {
        const direction: SwapDirection =
          action === "swap_y2n" ? "yesToNo" : "noToYes";
        if (direction === "yesToNo" && amount > userYes) {
          setError("Insufficient YES balance");
          setBusy(false);
          return;
        }
        if (direction === "noToYes" && amount > userNo) {
          setError("Insufficient NO balance");
          setBusy(false);
          return;
        }
        const [yesVault] = PublicKey.findProgramAddressSync(
          [Buffer.from("yes-vault"), pool.pool.toBuffer()],
          new PublicKey("GUwcYfYGqR6WPduoB6gEEZoPG6vdAAK7gK1xP6eTJ3JK"),
        );
        const [noVault] = PublicKey.findProgramAddressSync(
          [Buffer.from("no-vault"), pool.pool.toBuffer()],
          new PublicKey("GUwcYfYGqR6WPduoB6gEEZoPG6vdAAK7gK1xP6eTJ3JK"),
        );
        const userIn = direction === "yesToNo" ? ataYes : ataNo;
        const userOut = direction === "yesToNo" ? ataNo : ataYes;
        ixs.push(
          swapIx({
            user: owner,
            pool: pool.pool,
            yesVault,
            noVault,
            userInToken: userIn,
            userOutToken: userOut,
            amountIn: amount,
            minAmountOut: minOut,
            direction,
          }),
        );
      }

      const tx = new Transaction().add(...ixs);
      tx.feePayer = owner;
      tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
      const signed = await wallet.signTransaction(tx);
      const sig = await connection.sendRawTransaction(signed.serialize(), {
        skipPreflight: false,
      });
      await connection.confirmTransaction(sig, "confirmed");
      setAmountStr("");
      onSuccess();
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }, [
    action,
    amount,
    minOut,
    wallet,
    connection,
    market,
    pool,
    userCollateral,
    userYes,
    userNo,
    onSuccess,
  ]);

  return (
    <div className="bg-white p-6">
      <div className="stat-label mb-4">TRADE</div>

      {!isActive && (
        <div className="bg-canvas-alt border border-line text-xs p-3 mb-3 text-muted">
          Market is {statusLabel(market.status).toLowerCase()}; trading is closed.
        </div>
      )}

      {/* Action tabs */}
      <div className="flex gap-px bg-line border border-line mb-4 text-xs">
        <ActionTab label="Split" active={action === "split"} onClick={() => setAction("split")} />
        <ActionTab label="Buy YES" active={action === "swap_n2y"} onClick={() => setAction("swap_n2y")} />
        <ActionTab label="Buy NO" active={action === "swap_y2n"} onClick={() => setAction("swap_y2n")} />
      </div>

      <div className="text-xs text-muted mb-1">
        {action === "split"
          ? "Split collateral 1:1 into YES + NO"
          : action === "swap_n2y"
          ? "Swap NO → YES along the LMSR curve"
          : "Swap YES → NO along the LMSR curve"}
      </div>
      <label className="block">
        <span className="stat-label">
          {action === "split" ? "COLLATERAL" : action === "swap_n2y" ? "NO IN" : "YES IN"}
        </span>
        <input
          className="input mt-1"
          type="number"
          inputMode="decimal"
          step={action === "split" ? "0.000001" : "1"}
          placeholder="0"
          value={amountStr}
          onChange={(e) => setAmountStr(e.target.value)}
          disabled={!isActive || busy}
        />
        <div className="text-[10px] text-muted mt-1">
          balance:{" "}
          {action === "split"
            ? `${(Number(userCollateral) / 1e6).toFixed(2)} collateral`
            : action === "swap_n2y"
            ? `${Number(userNo).toLocaleString()} NO`
            : `${Number(userYes).toLocaleString()} YES`}
        </div>
      </label>

      {quote !== null && amount > 0n && (
        <div className="mt-4 bg-canvas-alt border border-line p-3 text-xs space-y-1">
          <div className="flex justify-between">
            <span className="text-muted">Output (est)</span>
            <span className="font-mono">{Number(quote.amountOut).toLocaleString()}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted">Min received</span>
            <span className="font-mono">{Number(minOut).toLocaleString()}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted">Price impact</span>
            <span className={`font-mono ${quote.priceImpact > 0.05 ? "text-loss" : ""}`}>
              {formatPct(quote.priceImpact, 2)}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted">Slippage</span>
            <select
              className="bg-transparent text-xs font-mono"
              value={slippageBps}
              onChange={(e) => setSlippageBps(parseInt(e.target.value, 10))}
            >
              <option value={50}>0.5%</option>
              <option value={100}>1%</option>
              <option value={500}>5%</option>
            </select>
          </div>
        </div>
      )}

      {error && (
        <div className="mt-3 bg-loss-soft border border-loss text-xs p-2 break-all">
          {error}
        </div>
      )}

      <button
        className="btn-primary mt-4 w-full"
        disabled={!wallet.publicKey || !isActive || busy || amount === 0n}
        onClick={submit}
      >
        {busy ? "Sending…" : !wallet.publicKey ? "Connect wallet" : "Submit"}
      </button>
    </div>
  );
}

function ActionTab({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex-1 py-2 ${active ? "bg-black text-white" : "bg-white hover:bg-canvas-alt"}`}
    >
      {label}
    </button>
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

function StatCell({ label, value }: { label: string; value: string }) {
  return (
    <div className="col-span-4 md:col-span-1 lg:col-span-1 pl-3 relative">
      <span className="absolute left-0 top-1 bottom-1 w-0.5 bg-line-strong" />
      <div className="stat-label">{label}</div>
      <div className="text-2xl font-bold mt-1 tabular-nums">{value}</div>
    </div>
  );
}

function parseAmount(s: string): bigint {
  const n = parseFloat(s);
  if (!isFinite(n) || n <= 0) return 0n;
  return BigInt(Math.floor(n * 10 ** COLLATERAL_DECIMALS));
}
