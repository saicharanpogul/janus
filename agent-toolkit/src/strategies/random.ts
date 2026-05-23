// Random-walk strategy: picks an arbitrary market and an arbitrary
// action. Useful for stressing the protocol and as a control group.

import type { Strategy } from "../strategy.js";
import type { Action, AgentContext } from "../types.js";

export class RandomStrategy implements Strategy {
  readonly name = "random";

  decide(ctx: AgentContext): Action {
    const r = Math.random();
    // 10% chance to create a market (only when we have collateral budget).
    if (r < 0.1 && ctx.collateral >= 10_000_000n) {
      return {
        kind: "create-market",
        pool: "cpmm",
        resolver: "slot",
        outcomeAtOrAfter: Math.random() < 0.5 ? 1 : 2,
        deadlineSlot: ctx.slot + 60n * 60n * 2n, // ~1h on a 2-slot-per-sec chain
        subsidy: 5_000_000n, // 5 USDC each side
        feeBps: 100,
      };
    }

    const active = ctx.markets.filter((m) => m.status === 0);
    if (active.length === 0) return { kind: "noop", reason: "no active markets" };
    const m = active[Math.floor(Math.random() * active.length)];

    // For CPMM: trade if we have a position; split first if we don't.
    if (m.poolType === "cpmm") {
      const myPos = ctx.positions.find((p) => p.market.equals(m.market));
      const hasPosition = myPos && (myPos.yesBalance > 0n || myPos.noBalance > 0n);

      if (!hasPosition) {
        // Always split to get a position when we don't have one and can afford it.
        if (ctx.collateral >= 200_000n) {
          return { kind: "split", market: m.market, amount: 200_000n };
        }
        return { kind: "noop", reason: "no position and broke" };
      }

      // Randomly swap or split-more.
      if (Math.random() < 0.2 && ctx.collateral >= 200_000n) {
        return { kind: "split", market: m.market, amount: 200_000n };
      }
      // Pick the bigger side to swap from.
      const fromYes =
        myPos!.yesBalance > myPos!.noBalance ? true : myPos!.noBalance === 0n;
      const sourceBal = fromYes ? myPos!.yesBalance : myPos!.noBalance;
      if (sourceBal < 100n) return { kind: "noop", reason: "dust position" };
      const amt = sourceBal / 4n + 1n;
      return {
        kind: "swap",
        market: m.market,
        direction: fromYes ? "yesToNo" : "noToYes",
        amountIn: amt,
        minAmountOut: 0n,
      };
    }

    // For true-LMSR: buy or sell.
    if (m.poolType === "true-lmsr") {
      const side = Math.random() < 0.5 ? "yes" : "no";
      const delta = 50_000n;
      if (Math.random() < 0.6 && ctx.collateral >= 100_000n) {
        return {
          kind: "buy",
          market: m.market,
          side,
          delta,
          maxCollateralIn: 200_000n,
        };
      }
      const pos = ctx.positions.find((p) => p.market.equals(m.market));
      const balance = pos
        ? side === "yes"
          ? pos.yesBalance
          : pos.noBalance
        : 0n;
      if (balance >= delta) {
        return {
          kind: "sell",
          market: m.market,
          side,
          delta,
          minCollateralOut: 0n,
        };
      }
    }

    return { kind: "noop", reason: "fall-through" };
  }
}
