// Mean-reverter: sells the side whose price is far from 0.5, expecting
// it to drift back. Tunable target; defaults to 0.5. Behaves the
// opposite of MomentumStrategy.

import type { Strategy } from "../strategy.js";
import type { Action, AgentContext } from "../types.js";

export class MeanReverterStrategy implements Strategy {
  readonly name = "mean-reverter";
  private target: number;
  private threshold: number;

  constructor(opts: { target?: number; threshold?: number } = {}) {
    this.target = opts.target ?? 0.5;
    // Lowered from 0.15 → 0.03 so fresh markets at exactly 0.5 still
    // get small probes; the strategy was over-conservative before.
    this.threshold = opts.threshold ?? 0.03;
  }

  decide(ctx: AgentContext): Action {
    const active = ctx.markets.filter((m) => m.status === 0);
    let best: { m: (typeof active)[number]; gap: number } | null = null;
    for (const m of active) {
      const gap = m.yesPrice - this.target;
      if (best == null || Math.abs(gap) > Math.abs(best.gap)) {
        best = { m, gap };
      }
    }
    if (best == null) return { kind: "noop" };
    if (Math.abs(best.gap) < this.threshold)
      return { kind: "noop", reason: "near target" };

    // YES too high → bet NO (price will revert down). And vice versa.
    const side: "yes" | "no" = best.gap > 0 ? "no" : "yes";

    if (ctx.collateral < 100_000n) return { kind: "noop", reason: "broke" };

    const m = best.m;
    const delta = 50_000n;
    if (m.poolType === "true-lmsr") {
      return { kind: "buy", market: m.market, side, delta, maxCollateralIn: 200_000n };
    }

    // CPMM
    const pos = ctx.positions.find((p) => p.market.equals(m.market));
    if (!pos || (pos.yesBalance === 0n && pos.noBalance === 0n)) {
      return { kind: "split", market: m.market, amount: 500_000n };
    }
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
