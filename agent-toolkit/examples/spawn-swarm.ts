// Example: spawn a 10-agent swarm on devnet for a few hours.
//
//   pnpm install
//   pnpm build
//   node --import=tsx examples/spawn-swarm.ts
//
// The example:
//   - Loads (or creates + persists) 10 agent keypairs at ./swarm.json
//   - Funds each with 0.5 SOL via the public devnet faucet
//   - Mints a shared test collateral and seeds each agent with 100 USDC-eq
//   - Assigns strategies in rotation: random / momentum / mean-revert / MM
//   - Runs 1000 ticks at 4-second intervals (~1h wall time)
//   - Writes telemetry to ./telemetry/

import { Connection, Keypair } from "@solana/web3.js";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { resolve } from "node:path";

import {
  loadOrCreateSwarm,
  fundAgentsWithSol,
  setupCollateralAndSeed,
  SwarmRunner,
  Telemetry,
  RandomStrategy,
  MomentumStrategy,
  MeanReverterStrategy,
  MarketMakerStrategy,
  type Strategy,
} from "../src/index.js";

const RPC = process.env.JANUS_RPC ?? "https://api.devnet.solana.com";
const SIZE = parseInt(process.env.SWARM_SIZE ?? "10", 10);
const TICKS = parseInt(process.env.SWARM_TICKS ?? "1000", 10);
const TICK_MS = parseInt(process.env.SWARM_TICK_MS ?? "4000", 10);

const conn = new Connection(RPC, "confirmed");

// Funding authority: defaults to the local solana CLI wallet.
const walletPath =
  process.env.JANUS_WALLET ?? resolve(homedir(), ".config/solana/id.json");
const authority = Keypair.fromSecretKey(
  Uint8Array.from(JSON.parse(readFileSync(walletPath, "utf-8"))),
);

console.log(`RPC: ${RPC}`);
console.log(`Authority: ${authority.publicKey.toBase58()}`);
console.log(`Swarm size: ${SIZE}, ticks: ${TICKS}, tick interval: ${TICK_MS}ms`);

const swarm = loadOrCreateSwarm({
  size: SIZE,
  storagePath: "./swarm.json",
});
console.log(`Loaded ${swarm.agents.length} agents.`);
console.log(`Collateral mint: ${swarm.collateralMint.publicKey.toBase58()}`);

console.log("Funding agents with SOL…");
await fundAgentsWithSol({
  connection: conn,
  swarm,
  authority,
  amountSol: 0.1, // 0.1 SOL covers ~1000 simple txs at devnet fees
  minSol: 0.02,
});

console.log("Seeding collateral…");
await setupCollateralAndSeed({
  connection: conn,
  swarm,
  authority,
  amountPerAgent: 100_000_000n, // 100 USDC at 6 decimals
});

const strategies: ((seed: number) => Strategy)[] = [
  () => new RandomStrategy(),
  () => new MomentumStrategy(),
  () => new MeanReverterStrategy(),
  () => new MarketMakerStrategy(),
];

const sessionId = new Date().toISOString().replace(/[:.]/g, "-");
const telemetry = new Telemetry({ dir: "./telemetry", sessionId });
console.log(`Telemetry: ${telemetry.eventLogPath}`);

const runner = new SwarmRunner({
  connection: conn,
  swarm,
  strategyFor: (agent) => strategies[agent.index % strategies.length](agent.index),
  tickIntervalMs: TICK_MS,
  maxTicks: TICKS,
  telemetry,
  verbose: true,
});

process.on("SIGINT", () => {
  console.log("\nSIGINT — finishing current tick then stopping…");
  runner.stop();
});

await runner.run();
console.log("Swarm done. Telemetry at:", telemetry.eventLogPath);
