// Devnet demo: runs the full Janus market lifecycle against
// devnet (or any RPC the caller points at). Unlike scripts/e2e-localnet,
// this script:
//   - Uses an existing keypair from disk (~/.config/solana/id.json by
//     default), not a freshly-generated one — so devnet rate limits
//     don't bite on every run.
//   - Skips airdrop unless on localhost.
//   - Prints explorer URLs for every transaction.
//
// Usage:
//   JANUS_RPC=https://api.devnet.solana.com \
//   JANUS_WALLET=~/.config/solana/id.json \
//     node scripts/devnet/demo.mjs

import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { resolve } from "node:path";
import {
  Connection,
  Keypair,
  Transaction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  createMint,
  mintTo,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";

import {
  createMarketWithSlotResolver,
  splitIx,
  swapIx,
} from "@janus/sdk";

const RPC = process.env.JANUS_RPC ?? "https://api.devnet.solana.com";
const WALLET_PATH = (process.env.JANUS_WALLET ?? "~/.config/solana/id.json")
  .replace(/^~/, homedir());

function loadKeypair(path) {
  const bytes = JSON.parse(readFileSync(resolve(path), "utf8"));
  return Keypair.fromSecretKey(new Uint8Array(bytes));
}

function explorerLink(sig, cluster) {
  return `https://explorer.solana.com/tx/${sig}?cluster=${cluster}`;
}

function clusterFromRpc(rpc) {
  if (rpc.includes("localhost") || rpc.includes("127.0.0.1")) return "custom";
  if (rpc.includes("devnet")) return "devnet";
  if (rpc.includes("testnet")) return "testnet";
  return "mainnet-beta";
}

async function main() {
  const conn = new Connection(RPC, "confirmed");
  const cluster = clusterFromRpc(RPC);
  const payer = loadKeypair(WALLET_PATH);

  console.log("=== Setup ===");
  console.log(`RPC:    ${RPC} (${cluster})`);
  console.log(`wallet: ${payer.publicKey.toBase58()}`);
  const balance = await conn.getBalance(payer.publicKey);
  console.log(`balance: ${(balance / LAMPORTS_PER_SOL).toFixed(4)} SOL`);
  if (balance < LAMPORTS_PER_SOL && cluster !== "custom") {
    console.error(
      "Wallet has less than 1 SOL. Top up via " +
        `\`solana --url ${RPC} airdrop 2\` (devnet) or transfer from another wallet.`,
    );
    process.exit(1);
  }

  console.log("\n=== Create collateral mint + ATA ===");
  const collateralMint = await createMint(conn, payer, payer.publicKey, null, 6);
  console.log("mint:", collateralMint.toBase58());
  const payerCollateral = await getOrCreateAssociatedTokenAccount(
    conn,
    payer,
    collateralMint,
    payer.publicKey,
  );
  await mintTo(conn, payer, collateralMint, payerCollateral.address, payer.publicKey, 10_000_000);
  console.log(
    `collateral balance: ${(await getAccount(conn, payerCollateral.address)).amount} (10 USDC)`,
  );

  console.log("\n=== Create market ===");
  const currentSlot = await conn.getSlot();
  const deadlineSlot = BigInt(currentSlot) + 200n;

  const m = createMarketWithSlotResolver({
    payer: payer.publicKey,
    authority: payer.publicKey,
    collateralMint,
    creatorCollateral: payerCollateral.address,
    deadlineSlot,
    resolutionOutcome: 1,
    subsidy: 500_000n, // 0.5 USDC each side
    feeBps: 100,
  });
  console.log("market:      ", m.market.toBase58());
  console.log("pool:        ", m.pool.toBase58());
  console.log("registration:", m.registration.toBase58());

  // Send in 2 txs to keep size sane.
  const tx1 = new Transaction().add(...m.instructions.slice(0, 4));
  const sig1 = await sendAndConfirmTransaction(conn, tx1, [payer]);
  console.log("tx1:", explorerLink(sig1, cluster));

  const tx2 = new Transaction().add(...m.instructions.slice(4));
  const sig2 = await sendAndConfirmTransaction(conn, tx2, [payer]);
  console.log("tx2:", explorerLink(sig2, cluster));

  console.log("\n=== Top-up + swap ===");
  const yesAta = getAssociatedTokenAddressSync(m.yesMint, payer.publicKey, true);
  const noAta = getAssociatedTokenAddressSync(m.noMint, payer.publicKey, true);

  const topup = new Transaction().add(
    splitIx({
      user: payer.publicKey,
      market: m.market,
      userCollateral: payerCollateral.address,
      vault: m.vault,
      yesMint: m.yesMint,
      noMint: m.noMint,
      userYes: yesAta,
      userNo: noAta,
      amount: 1_000_000n,
    }),
  );
  const sig3 = await sendAndConfirmTransaction(conn, topup, [payer]);
  console.log("topup split:", explorerLink(sig3, cluster));

  const swap = new Transaction().add(
    swapIx({
      user: payer.publicKey,
      pool: m.pool,
      yesVault: m.yesVault,
      noVault: m.noVault,
      userInToken: yesAta,
      userOutToken: noAta,
      amountIn: 200_000n,
      minAmountOut: 100_000n,
      direction: "yesToNo",
    }),
  );
  const sig4 = await sendAndConfirmTransaction(conn, swap, [payer]);
  console.log("swap:       ", explorerLink(sig4, cluster));

  const noAfter = (await getAccount(conn, noAta)).amount;
  console.log(`trader NO balance after swap: ${noAfter}`);

  console.log("\n✓ Demo complete. Market is live at:");
  console.log(`  ${m.market.toBase58()}`);
  console.log("  Look up the registration account to discover the full bundle:");
  console.log(`  solana --url ${RPC} account ${m.registration.toBase58()}`);
}

main().catch((e) => {
  console.error("\n✗ Demo failed:", e);
  process.exit(1);
});
