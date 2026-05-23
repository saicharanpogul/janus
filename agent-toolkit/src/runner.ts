// Swarm runner: orchestrates N agents on a tick loop. Each tick:
//   1. Refreshes shared market snapshot + per-agent balances.
//   2. Calls each agent's strategy.decide() with the current context.
//   3. Sends the resulting action through the TradeClient.
//   4. Emits telemetry.
//
// Concurrency: agents act sequentially per-tick to avoid hammering the
// devnet RPC with parallel signature requests. A real distributed swarm
// would run each agent in its own process; this single-process version
// is plenty for hundreds-of-tx-per-hour experiments.

import { Connection, PublicKey } from "@solana/web3.js";
import {
  AccountLayout,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";

import { snapshotAllMarkets } from "./discovery.js";
import { Telemetry } from "./telemetry.js";
import { TradeClient } from "./trade-client.js";
import type { Strategy } from "./strategy.js";
import type {
  Action,
  ActionResult,
  AgentContext,
  AgentPosition,
  TelemetryEvent,
} from "./types.js";
import type { SwarmIdentity, SwarmSession } from "./keypair-manager.js";

export interface RunnerOpts {
  connection: Connection;
  swarm: SwarmSession;
  /** Strategy factory — one Strategy instance per agent (so strategies that
   *  carry state stay isolated). */
  strategyFor: (agent: SwarmIdentity) => Strategy;
  /** ms between ticks. */
  tickIntervalMs: number;
  /** Number of ticks to run. Infinity = run until interrupted. */
  maxTicks: number;
  telemetry: Telemetry;
  /** Print per-tick summary to stdout. */
  verbose?: boolean;
}

export class SwarmRunner {
  private opts: RunnerOpts;
  private strategies: Map<string, Strategy> = new Map();
  private client: TradeClient;
  private tick = 0;
  private stopFlag = false;

  constructor(opts: RunnerOpts) {
    this.opts = opts;
    for (const agent of opts.swarm.agents) {
      this.strategies.set(agent.agentId, opts.strategyFor(agent));
    }
    this.client = new TradeClient({
      connection: opts.connection,
      collateralMint: opts.swarm.collateralMint.publicKey,
    });
  }

  stop(): void {
    this.stopFlag = true;
  }

  async run(): Promise<void> {
    while (this.tick < this.opts.maxTicks && !this.stopFlag) {
      const tickStart = Date.now();
      try {
        await this.runTick();
      } catch (e) {
        console.error(`[runner] tick ${this.tick} failed:`, e);
      }
      const elapsed = Date.now() - tickStart;
      const sleep = Math.max(0, this.opts.tickIntervalMs - elapsed);
      if (sleep > 0) await new Promise((r) => setTimeout(r, sleep));
      this.tick++;
    }
  }

  private async runTick(): Promise<void> {
    const slot = BigInt(await this.opts.connection.getSlot("confirmed"));
    const allMarkets = await snapshotAllMarkets(this.opts.connection);
    // Restrict to markets that use the swarm's shared collateral mint —
    // otherwise the swarm tries to act on foreign markets it can't pay
    // into, producing "invalid account data" errors at simulation.
    const swarmMint = this.opts.swarm.collateralMint.publicKey;
    const markets = allMarkets.filter((m) => m.collateralMint.equals(swarmMint));

    const summary = {
      tick: this.tick,
      slot: slot.toString(),
      markets: markets.length,
      activeMarkets: markets.filter((m) => m.status === 0).length,
      actions: 0,
      errors: 0,
    };

    for (const agent of this.opts.swarm.agents) {
      if (this.stopFlag) break;
      const strategy = this.strategies.get(agent.agentId);
      if (!strategy) continue;

      // Build the agent's context.
      const collateral = await this.getCollateralBalance(agent.keypair.publicKey);
      const positions = await this.getPositions(agent.keypair.publicKey, markets);
      const ctx: AgentContext = {
        agentId: agent.agentId,
        pubkey: agent.keypair.publicKey,
        collateral,
        slot,
        markets,
        positions,
        tick: this.tick,
      };

      let action: Action;
      try {
        action = await strategy.decide(ctx);
      } catch (e: any) {
        action = { kind: "noop", reason: `decide threw: ${e?.message}` };
      }

      let result: ActionResult;
      if (action.kind === "noop") {
        result = { ok: true };
      } else {
        result = await this.client.execute(agent.keypair, action, markets);
        summary.actions++;
        if (!result.ok) summary.errors++;
      }

      strategy.observe?.(ctx, action, result);

      const event: TelemetryEvent = {
        ts: Date.now(),
        tick: this.tick,
        agentId: agent.agentId,
        action,
        result,
      };
      this.opts.telemetry.emit(event);
    }

    if (this.opts.verbose) {
      console.log(
        `[tick ${summary.tick}] slot=${summary.slot} markets=${summary.markets} active=${summary.activeMarkets} actions=${summary.actions} errors=${summary.errors}`,
      );
    }

    // Periodic snapshot for the dashboard (every 5 ticks).
    if (this.tick % 5 === 0) {
      const snapshot = {
        tick: this.tick,
        ts: Date.now(),
        slot: slot.toString(),
        markets: markets.length,
        activeMarkets: markets.filter((m) => m.status === 0).length,
      };
      this.opts.telemetry.writeSnapshot(snapshot);
    }
  }

  private async getCollateralBalance(owner: PublicKey): Promise<bigint> {
    const ata = getAssociatedTokenAddressSync(
      this.opts.swarm.collateralMint.publicKey,
      owner,
      true,
    );
    const info = await this.opts.connection.getAccountInfo(ata, "confirmed");
    if (!info) return 0n;
    try {
      return BigInt(AccountLayout.decode(info.data).amount.toString());
    } catch {
      return 0n;
    }
  }

  private async getPositions(
    owner: PublicKey,
    markets: import("./types.js").MarketSnapshot[],
  ): Promise<AgentPosition[]> {
    // For each market, derive the user's YES + NO ATAs and fetch in one
    // batched getMultipleAccountsInfo call. Without this, ticks scale O(N×M)
    // RPCs which devnet hates.
    if (markets.length === 0) return [];
    const headers = await this.opts.connection.getMultipleAccountsInfo(
      markets.map((m) => m.market),
      "confirmed",
    );
    const mints: PublicKey[] = [];
    const ataKeys: { idx: number; side: "yes" | "no" }[] = [];
    headers.forEach((info, i) => {
      if (!info || info.data.length !== 248) return;
      const yesMint = new PublicKey(info.data.subarray(40, 72));
      const noMint = new PublicKey(info.data.subarray(72, 104));
      mints.push(
        getAssociatedTokenAddressSync(yesMint, owner, true),
        getAssociatedTokenAddressSync(noMint, owner, true),
      );
      ataKeys.push(
        { idx: i, side: "yes" },
        { idx: i, side: "no" },
      );
    });
    if (mints.length === 0) return [];
    const ataInfos = await this.opts.connection.getMultipleAccountsInfo(
      mints,
      "confirmed",
    );
    const positions: AgentPosition[] = markets.map((m) => ({
      market: m.market,
      yesBalance: 0n,
      noBalance: 0n,
    }));
    ataInfos.forEach((info, k) => {
      if (!info) return;
      try {
        const amt = BigInt(AccountLayout.decode(info.data).amount.toString());
        const { idx, side } = ataKeys[k];
        if (side === "yes") positions[idx].yesBalance = amt;
        else positions[idx].noBalance = amt;
      } catch {
        /* not a token account */
      }
    });
    return positions.filter(
      (p) => p.yesBalance > 0n || p.noBalance > 0n,
    );
  }
}
