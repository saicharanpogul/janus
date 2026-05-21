// Market maker: creates markets when there aren't enough active ones,
// and provides liquidity by splitting collateral when their inventory
// runs low. Does NOT swap directionally — passively earns fees from
// CPMM trades (and the bounded-loss bet on true-LMSR markets).

import type { Strategy } from "../strategy.js";
import type { Action, AgentContext } from "../types.js";

export class MarketMakerStrategy implements Strategy {
  readonly name = "market-maker";
  private targetActiveMarkets: number;
  private subsidyPerSide: bigint;

  constructor(opts: {
    targetActiveMarkets?: number;
    subsidyPerSide?: bigint;
  } = {}) {
    this.targetActiveMarkets = opts.targetActiveMarkets ?? 3;
    this.subsidyPerSide = opts.subsidyPerSide ?? 5_000_000n; // 5 USDC
  }

  decide(ctx: AgentContext): Action {
    const active = ctx.markets.filter((m) => m.status === 0);

    // Spawn new markets if the floor is below target.
    const minSubsidy = this.subsidyPerSide * 2n + 1_000_000n;
    if (
      active.length < this.targetActiveMarkets &&
      ctx.collateral >= minSubsidy
    ) {
      return {
        kind: "create-market",
        pool: "cpmm",
        resolver: "slot",
        // Random outcome — the MM doesn't know who'll win, just makes the market.
        outcomeAtOrAfter: (Math.floor(Math.random() * 2) + 1) as 1 | 2,
        deadlineSlot: ctx.slot + 60n * 60n * 2n,
        subsidy: this.subsidyPerSide,
        feeBps: 100,
      };
    }

    // For each market we created where inventory got drained, top up by
    // splitting more collateral.
    for (const m of active) {
      if (!m.creator.equals(ctx.pubkey)) continue;
      if (m.poolType !== "cpmm") continue;
      if (m.yesReserves < 200_000n || m.noReserves < 200_000n) {
        if (ctx.collateral >= 1_000_000n) {
          return {
            kind: "split",
            market: m.market,
            amount: 1_000_000n,
          };
        }
      }
    }

    return { kind: "noop", reason: "MM idle" };
  }
}
