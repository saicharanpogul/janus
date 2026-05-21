// E2E test against a local solana-test-validator with all 5 Janus
// programs preloaded. Exercises the SDK's createMarketWithSlotResolver
// flow end-to-end and asserts on-chain state at each checkpoint.

import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  createMint,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";

import {
  createMarketWithSlotResolver,
  splitIx,
  swapIx,
  deriveMarketPda,
} from "@janus/sdk";

const RPC = process.env.JANUS_RPC ?? "http://127.0.0.1:8899";
const conn = new Connection(RPC, "confirmed");

function logHeader(t) {
  console.log(`\n=== ${t} ===`);
}

function assertEq(actual, expected, label) {
  if (String(actual) !== String(expected)) {
    throw new Error(`assert failed (${label}): expected ${expected}, got ${actual}`);
  }
  console.log(`  ✓ ${label} = ${actual}`);
}

async function main() {
  logHeader("Setup");
  const payer = Keypair.generate();
  console.log("  payer:", payer.publicKey.toBase58());

  let sig = await conn.requestAirdrop(payer.publicKey, 50 * LAMPORTS_PER_SOL);
  await conn.confirmTransaction(sig, "confirmed");
  console.log("  airdropped 50 SOL");

  // Create a USDC-like mint.
  const collateralMint = await createMint(conn, payer, payer.publicKey, null, 6);
  console.log("  collateral mint:", collateralMint.toBase58());

  // Mint 1 USDC to the payer's collateral ATA.
  const { getOrCreateAssociatedTokenAccount } = await import("@solana/spl-token");
  const payerCollateral = await getOrCreateAssociatedTokenAccount(
    conn,
    payer,
    collateralMint,
    payer.publicKey,
  );
  await mintTo(conn, payer, collateralMint, payerCollateral.address, payer.publicKey, 1_000_000);
  const balance = (await getAccount(conn, payerCollateral.address)).amount;
  assertEq(balance.toString(), "1000000", "payer collateral seeded with 1 USDC");

  logHeader("Create market via SDK");
  const currentSlot = await conn.getSlot();
  const deadlineSlot = BigInt(currentSlot) + 100n;
  const subsidy = 100_000n; // 0.1 USDC each side

  const m = createMarketWithSlotResolver({
    payer: payer.publicKey,
    authority: payer.publicKey,
    collateralMint,
    creatorCollateral: payerCollateral.address,
    deadlineSlot,
    resolutionOutcome: 1, // Yes
    subsidy,
    feeBps: 100,
  });
  console.log("  market PDA:    ", m.market.toBase58());
  console.log("  pool PDA:      ", m.pool.toBase58());
  console.log("  yes mint PDA:  ", m.yesMint.toBase58());
  console.log("  no mint PDA:   ", m.noMint.toBase58());
  console.log("  vault PDA:     ", m.vault.toBase58());
  console.log("  resolver state:", m.resolverState.toBase58());
  console.log("  registration:  ", m.registration.toBase58());
  console.log(`  generated ${m.instructions.length} instructions`);

  // Send the createMarket instructions. The SDK's flow has 7 ixs:
  //  0: init resolver
  //  1: init market
  //  2: create yes ATA (idempotent)
  //  3: create no ATA (idempotent)
  //  4: split subsidy
  //  5: init pool
  //  6: register
  //
  // 7 ixs might overflow tx size; split into two transactions.
  logHeader("Send createMarket transactions");
  const tx1 = new Transaction().add(...m.instructions.slice(0, 4));
  sig = await sendAndConfirmTransaction(conn, tx1, [payer]);
  console.log("  tx1 (resolver + market + ATAs):", sig);

  const tx2 = new Transaction().add(...m.instructions.slice(4));
  sig = await sendAndConfirmTransaction(conn, tx2, [payer]);
  console.log("  tx2 (split + init pool + register):", sig);

  logHeader("Verify on-chain state");
  const marketAcct = await conn.getAccountInfo(m.market);
  if (!marketAcct) throw new Error("market account not created");
  assertEq(marketAcct.data.length, 248, "Market struct size");
  assertEq(marketAcct.data[1], 0, "Market status = Active (0)");

  const poolAcct = await conn.getAccountInfo(m.pool);
  if (!poolAcct) throw new Error("pool account not created");
  assertEq(poolAcct.data.length, 224, "Pool struct size");

  const yesVault = await getAccount(conn, m.yesVault);
  assertEq(yesVault.amount.toString(), subsidy.toString(), "YES vault holds subsidy");
  const noVault = await getAccount(conn, m.noVault);
  assertEq(noVault.amount.toString(), subsidy.toString(), "NO vault holds subsidy");

  const reg = await conn.getAccountInfo(m.registration);
  if (!reg) throw new Error("registration not created");
  assertEq(reg.data.length, 216, "Registration size");

  logHeader("Top-up split — give the trader YES+NO to actually trade with");
  // createMarketWithSlotResolver sweeps the full subsidy into the pool,
  // so the trader's YES/NO ATAs end at zero. Do an extra Split here so
  // the trader has 200_000 of each side to play with.
  const yesAta = getAssociatedTokenAddressSync(m.yesMint, payer.publicKey, true);
  const noAta = getAssociatedTokenAddressSync(m.noMint, payer.publicKey, true);
  const topupTx = new Transaction().add(
    splitIx({
      user: payer.publicKey,
      market: m.market,
      userCollateral: payerCollateral.address,
      vault: m.vault,
      yesMint: m.yesMint,
      noMint: m.noMint,
      userYes: yesAta,
      userNo: noAta,
      amount: 200_000n,
    }),
  );
  sig = await sendAndConfirmTransaction(conn, topupTx, [payer]);
  console.log("  topup split tx:", sig);
  const yesBefore = (await getAccount(conn, yesAta)).amount;
  const noBefore = (await getAccount(conn, noAta)).amount;
  console.log(`  trader holds: ${yesBefore} YES, ${noBefore} NO`);

  logHeader("Swap YES → NO");

  const swapTx = new Transaction().add(
    swapIx({
      user: payer.publicKey,
      pool: m.pool,
      yesVault: m.yesVault,
      noVault: m.noVault,
      userInToken: yesAta,
      userOutToken: noAta,
      amountIn: 50_000n,
      minAmountOut: 30_000n,
      direction: "yesToNo",
    }),
  );
  sig = await sendAndConfirmTransaction(conn, swapTx, [payer]);
  console.log("  swap tx:", sig);

  const yesAfter = (await getAccount(conn, yesAta)).amount;
  const noAfter = (await getAccount(conn, noAta)).amount;
  assertEq(
    (yesBefore - yesAfter).toString(),
    "50000",
    "trader debited 50_000 YES",
  );
  const noOut = noAfter - noBefore;
  if (noOut < 30_000n) throw new Error(`swap output < min: ${noOut}`);
  console.log(`  swap output: ${noOut} NO (>= min 30000)`);

  logHeader("✓ all checks passed");
}

main().catch((e) => {
  console.error("\n✗ E2E FAILED:", e);
  process.exit(1);
});
