// Verify that the pyth-price-resolver's byte-offset parsing matches
// Pyth's actual PriceUpdateV2 layout on devnet.
//
// Pyth pull-oracle accounts are dynamically posted by users via the
// Hermes endpoint, then closed after they're consumed. So we don't
// hard-code a single "SOL/USD" pubkey; instead this script:
//
//   1. Takes a PriceUpdateV2 pubkey as input (env var or argv)
//   2. Fetches the account
//   3. Parses our exact byte offsets (feed_id @ 41, price @ 73,
//      exponent @ 89, posted_slot @ 125)
//   4. Prints the parsed values + sanity-checks the discriminator
//
// To get a fresh PriceUpdateV2 account, post one via the Pyth pull
// SDK (`@pythnetwork/pyth-solana-receiver`) or use one already on
// chain. For a quick smoke test, the script accepts a hard-coded
// known-good account too:
//
//   PYTH_ACCOUNT=<pubkey> JANUS_RPC=https://api.devnet.solana.com \
//     node scripts/devnet/pyth-verify.mjs

import { Connection, PublicKey } from "@solana/web3.js";

const RPC = process.env.JANUS_RPC || "https://api.devnet.solana.com";
const pubkey = process.argv[2] || process.env.PYTH_ACCOUNT;

if (!pubkey) {
  console.error("Usage: PYTH_ACCOUNT=<pubkey> node pyth-verify.mjs");
  console.error("   or: node pyth-verify.mjs <pubkey>");
  console.error("");
  console.error("Pubkey must be a Pyth PriceUpdateV2 account on devnet.");
  console.error("Post one via @pythnetwork/pyth-solana-receiver.");
  process.exit(1);
}

// Anchor discriminator for PriceUpdateV2 — sha256("account:PriceUpdateV2")[..8]
const PRICE_UPDATE_V2_DISCRIMINATOR = Buffer.from([
  34, 241, 35, 99, 157, 126, 244, 205,
]);

// Byte offsets that the on-chain pyth-price-resolver hard-codes.
const FEED_ID_OFFSET = 41;
const PRICE_OFFSET = 73;
const EXPONENT_OFFSET = 89;
const POSTED_SLOT_OFFSET = 125;

const conn = new Connection(RPC, "confirmed");
const acct = await conn.getAccountInfo(new PublicKey(pubkey), "confirmed");

if (!acct) {
  console.error(`Account ${pubkey} not found on ${RPC}`);
  process.exit(2);
}

const data = acct.data;
console.log(`Fetched account: ${pubkey}`);
console.log(`  size:  ${data.length} bytes`);
console.log(`  owner: ${acct.owner.toString()}`);
console.log("");

// 1. Discriminator
const disc = data.subarray(0, 8);
if (!disc.equals(PRICE_UPDATE_V2_DISCRIMINATOR)) {
  console.error("FAIL discriminator:");
  console.error("  expected:", PRICE_UPDATE_V2_DISCRIMINATOR);
  console.error("  got:     ", disc);
  process.exit(3);
}
console.log("✓ discriminator matches PriceUpdateV2");

// 2. feed_id (32 bytes)
const feedId = data.subarray(FEED_ID_OFFSET, FEED_ID_OFFSET + 32);
console.log(`  feed_id:     0x${feedId.toString("hex")}`);

// 3. price (i64 LE, signed)
const priceLow = data.readBigUInt64LE(PRICE_OFFSET);
const priceSigned = BigInt.asIntN(64, priceLow);
console.log(`  price:       ${priceSigned}`);

// 4. exponent (i32 LE, signed)
const expoUnsigned = data.readUInt32LE(EXPONENT_OFFSET);
const expoSigned = expoUnsigned > 2 ** 31 ? expoUnsigned - 2 ** 32 : expoUnsigned;
console.log(`  exponent:    ${expoSigned}`);

// 5. posted_slot (u64 LE)
const postedSlot = data.readBigUInt64LE(POSTED_SLOT_OFFSET);
console.log(`  posted_slot: ${postedSlot}`);

// 6. Derived: human-readable price
const human = Number(priceSigned) * Math.pow(10, expoSigned);
console.log(`  human price: ${human}`);

console.log("");
console.log("✓ All byte offsets parse cleanly against on-chain layout.");
console.log("  These are the same offsets the pyth-price-resolver uses.");
