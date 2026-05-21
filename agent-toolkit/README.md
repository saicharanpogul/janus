# @janus/agent-toolkit

Build autonomous agents that create and trade on Janus binary markets.

The toolkit gives you:

| Module | What it does |
|---|---|
| `keypair-manager` | Generate N persistent agent wallets, airdrop SOL, seed shared test collateral |
| `discovery` | Snapshot every market on chain (CPMM + true-LMSR) into a unified `MarketSnapshot[]` |
| `trade-client` | High-level `execute(agent, action, markets)` — handles split/swap/buy/sell/redeem and create-market |
| `strategy` | Abstract `Strategy { decide(ctx) → Action; observe?(ctx, action, result) }` interface |
| `runner` | `SwarmRunner` orchestrates N agents on a tick loop with batched RPC reads |
| `telemetry` | Append-only JSONL log of every action + result + periodic snapshot |

Plus four example strategies in `strategies/`:
- **`RandomStrategy`** — control group; arbitrary action picker.
- **`MomentumStrategy`** — buys the side whose price is moving up (EWMA-based).
- **`MeanReverterStrategy`** — bets the price drifts back toward a target.
- **`MarketMakerStrategy`** — creates markets when there aren't enough and tops up its own pools.

## Quickstart — spawn a 10-agent swarm on devnet

```bash
cd agent-toolkit
pnpm install
node --import=tsx examples/spawn-swarm.ts
```

Default config: 10 agents, 1000 ticks @ 4s each (~1 hour). Override via env:

```bash
SWARM_SIZE=25 SWARM_TICKS=10000 SWARM_TICK_MS=2000 \
  JANUS_RPC=https://api.devnet.solana.com \
  node --import=tsx examples/spawn-swarm.ts
```

The first run:
1. Generates 10 fresh keypairs → persists them to `./swarm.json` so subsequent runs reuse the same agents (positions accumulate).
2. Airdrops 0.5 SOL to each.
3. Creates a shared test collateral mint and seeds each agent with 100 USDC-equivalent.
4. Spawns the strategies in rotation (agent #0 = random, #1 = momentum, …).
5. Writes telemetry to `./telemetry/events-<sessionId>.jsonl`.

## Writing your own strategy

```ts
import { Strategy, Action, AgentContext } from "@janus/agent-toolkit";

export class MyStrategy implements Strategy {
  readonly name = "my-strategy";

  private memory = new Map<string, number>();

  decide(ctx: AgentContext): Action {
    // Pick a market. Inspect ctx.markets, ctx.positions, ctx.collateral.
    const m = ctx.markets.find((m) => m.poolType === "true-lmsr" && m.status === 0);
    if (!m) return { kind: "noop", reason: "no LMSR market" };

    // Bet a fraction of collateral on YES if its price < 0.4.
    if (m.yesPrice < 0.4 && ctx.collateral > 1_000_000n) {
      return {
        kind: "buy",
        market: m.market,
        side: "yes",
        delta: 50_000n,
        maxCollateralIn: 100_000n,
      };
    }
    return { kind: "noop" };
  }

  observe(ctx: AgentContext, action: Action, result: ActionResult): void {
    // Update internal state (Q-table, model weights, EWMA) based on the result.
  }
}
```

Then plug it into `spawn-swarm.ts`'s `strategyFor` factory.

## Self-learning agents

The `observe()` hook is the learning entry point. Track the result of each
action, update your strategy's state, and the next `decide()` call has the
updated knowledge. A simple Q-learning template:

```ts
class QLearner implements Strategy {
  readonly name = "q-learner";
  private Q = new Map<string, Map<string, number>>(); // state → action → value
  private epsilon = 0.1;

  private stateKey(ctx: AgentContext): string {
    // Discretize: e.g. round prices to 0.1 buckets, count active markets.
    return `markets=${ctx.markets.length}|collateral=${ctx.collateral / 10_000_000n}`;
  }

  decide(ctx: AgentContext): Action {
    const state = this.stateKey(ctx);
    // Epsilon-greedy: explore vs exploit.
    if (Math.random() < this.epsilon) return this.randomAction(ctx);
    return this.bestAction(state, ctx);
  }

  observe(ctx: AgentContext, action: Action, result: ActionResult): void {
    // Bellman update: Q(s,a) += alpha * (reward + gamma * max Q(s',·) - Q(s,a))
  }

  // ... bestAction / randomAction / etc.
}
```

## Telemetry analysis

Every action lands on disk as a JSON line. To compute final P&L per agent:

```ts
import { readFileSync } from "node:fs";
const events = readFileSync("./telemetry/events-<sessionId>.jsonl", "utf-8")
  .split("\n")
  .filter(Boolean)
  .map((l) => JSON.parse(l));
const byAgent = new Map<string, number>();
for (const e of events) {
  const a = byAgent.get(e.agentId) ?? 0;
  // Heuristic: count successful actions as +1, errors as 0
  byAgent.set(e.agentId, a + (e.result.ok ? 1 : 0));
}
console.table(Array.from(byAgent.entries()));
```

A richer dashboard at `/swarm` is available in the demo app.

## Rate-limiting + reliability

- The runner serializes actions per-tick to avoid hammering devnet's RPC.
- Each agent's keypair persists across runs (`swarm.json`); positions accumulate.
- Idempotent ATA creation in `TradeClient` means re-runs don't blow up on existing accounts.
- Devnet airdrop is rate-limited; `fundAgentsWithSol` skips already-funded
  agents and uses exponential backoff on rate-limit errors.

## Architecture

```
SwarmRunner (tick loop)
   │
   ├─ snapshot markets (1 RPC batch via discovery)
   │
   ├─ for each agent:
   │     │
   │     ├─ build AgentContext (collateral + positions)
   │     ├─ strategy.decide(ctx) → Action
   │     ├─ TradeClient.execute(agent, action, markets)
   │     ├─ strategy.observe(ctx, action, result)
   │     └─ telemetry.emit(event)
   │
   └─ every 5 ticks: telemetry.writeSnapshot(stats)
```
