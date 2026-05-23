// Momentum / trend-follower: buys the side whose price is moving up,
// sells when it stalls. Tracks the EWMA of each market's YES price.

import type { Strategy } from "../strategy.js";
import type { Action, AgentContext } from "../types.js";

export class MomentumStrategy implements Strategy {
  readonly name = "momentum";
  private ewma = new Map<string, number>();
  private alpha: number;
  private threshold: number;

  constructor(opts: { alpha?: number; threshold?: number } = {}) {
    this.alpha = opts.alpha ?? 0.2;
    // 0.005 lets it act on small (~0.5%) deltas, which lets the swarm
    // explore. Bump back up for noisier markets.
    this.threshold = opts.threshold ?? 0.005;
  }

  decide(ctx: AgentContext): Action {
    const active = ctx.markets.filter((m) => m.status === 0);
    if (active.length === 0) return { kind: "noop" };

    // Update EWMA + find the market with the strongest deviation from trend.
    let best: { m: (typeof active)[number]; delta: number } | null = null;
    for (const m of active) {
      const key = m.market.toBase58();
      const prev = this.ewma.get(key) ?? m.yesPrice;
      const next = this.alpha * m.yesPrice + (1 - this.alpha) * prev;
      this.ewma.set(key, next);
      const delta = m.yesPrice - prev;
      if (best == null || Math.abs(delta) > Math.abs(best.delta)) {
        best = { m, delta };
      }
    }
    if (best == null) return { kind: "noop" };
    if (Math.abs(best.delta) < this.threshold)
      return { kind: "noop", reason: "no strong trend" };

    const m = best.m;
    const side: "yes" | "no" = best.delta > 0 ? "yes" : "no";

    // Need at least 0.1 USDC of budget.
    if (ctx.collateral < 100_000n) return { kind: "noop", reason: "broke" };

    const delta = 50_000n;
    const maxIn = 100_000n;

    if (m.poolType === "true-lmsr") {
      return { kind: "buy", market: m.market, side, delta, maxCollateralIn: maxIn };
    }
    // CPMM: buying a side means swapping the OTHER side INTO the pool.
    // We need outcome tokens to swap with — split first if we have none.
    const pos = ctx.positions.find((p) => p.market.equals(m.market));
    if (!pos || (pos.yesBalance === 0n && pos.noBalance === 0n)) {
      return { kind: "split", market: m.market, amount: 1_000_000n };
    }
    // Buy YES = swap NO into pool. Buy NO = swap YES into pool.
    if (side === "yes" && pos.noBalance > 0n) {
      return {
        kind: "swap",
        market: m.market,
        direction: "noToYes",
        amountIn: pos.noBalance / 4n + 1n,
        minAmountOut: 0n,
      };
    }
    if (side === "no" && pos.yesBalance > 0n) {
      return {
        kind: "swap",
        market: m.market,
        direction: "yesToNo",
        amountIn: pos.yesBalance / 4n + 1n,
        minAmountOut: 0n,
      };
    }
    return { kind: "noop" };
  }
}
