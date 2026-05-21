# Devnet operator playbook

Live deployment of Janus on Solana devnet. Use this as the staging
loop before any mainnet move.

## Deployed program IDs (devnet)

| Program | ID |
|---|---|
| conditional-tokens | `SH9ghSowHqqWR5YcXVtmkXjt8is1qERCmxHXEvf5sw1` |
| lmsr-market (CPMM v1) | `GUwcYfYGqR6WPduoB6gEEZoPG6vdAAK7gK1xP6eTJ3JK` |
| lmsr-true-market | `HrFV8Nfncv2gekc9jZPC6rXxnVVaUQi75BmwVFzd5fjQ` |
| slot-height-resolver | `3y75gGqFK1KhNF5k1sMy6ydnw6WLcbn1SPRoYbyRkjMj` |
| pyth-price-resolver | `3WDargKHd1UaP9UKPhJY8pF5bv5zJnaFAYDA9uahs5aL` |
| market-factory | `8ibKxXAWsdqyNG1wExRSvLhKBgXiPpqtE6ZkA277gPwC` |

First deployed: 2026-05-21. Same IDs hold across mainnet once promoted
(keypairs in `target/deploy/`).

## Canonical demo market

The bundled `scripts/devnet/demo.mjs` walks the full lifecycle.
First run (2026-05-21) produced market `6W98N95Rko8jgMobawWtWQnrRNEfCjKvuRUV9RxxFQdv`
with pool `FfrV9SJnyKR9LN81Yfxer2AmToxrRoe1o99BxxLicrs3` and registration
`HkHMhQm8cV3wqgTxG92jXDPkkLCwsPv5y2KBC8rMYqxB`.

## Redeploy a program

When a program changes, redeploy it idempotently:

```bash
cargo build-sbf
JANUS_RPC=https://api.devnet.solana.com bash scripts/devnet/deploy.sh
```

The script skips programs whose `program show` already returns a result
at the keypair-derived address. To force a fresh deploy:

```bash
solana --url devnet program close <program_id> --bypass-warning
# then re-run deploy.sh
```

Note: `program close` reclaims the rent into the wallet, but the
program is *gone* — any market state referencing it becomes
unreachable. Don't close conditional-tokens or any program with
live markets unless you accept that those markets are bricked.

## Rotate keypairs

If a deploy keypair is compromised:

```bash
solana-keygen new -o target/deploy/janus_<program>-keypair.json
bash scripts/sync-program-ids.sh
cd sdk && pnpm build
cargo build-sbf
JANUS_RPC=https://api.devnet.solana.com bash scripts/devnet/deploy.sh
```

The new ID propagates through Rust `declare_id!`, the SDK constants,
and the tests `ids` module. Existing on-chain accounts won't migrate
— they're tied to the old program — so this is effectively a fresh
deployment.

## SOL budget

Empirical from 2026-05-21: 6 program deploys cost ≈ 0.94 SOL total.
Plus 0.005 SOL per market created (rent for the conditional-tokens
market account + YES/NO mints + vault) and ≈ 0.0006 SOL per swap
(tx fee).

Airdrop limits on devnet are tight; if `solana airdrop 2` rate-limits,
wait 30 seconds and retry, or use the QuickNode / Helius RPC airdrop
endpoints.

## Closing a buffer (rent recovery)

`program deploy` opens a buffer account that holds the program bytes
during deployment. On success, the buffer is closed and the rent
moves into the program executable account. If a deploy fails midway,
the orphaned buffer holds ~2 SOL of stranded rent. Recover with:

```bash
solana --url devnet program show --buffers
solana --url devnet program close <buffer_pubkey>
```

## Pyth feed integration

The `pyth-price-resolver` reads `PriceUpdateV2` accounts at hard-coded
byte offsets (feed_id @ 41, price @ 73, exponent @ 89, posted_slot @
125), gated by the Anchor discriminator `[34,241,35,99,157,126,244,205]`.

**Posting a fresh feed account.** Pyth's pull-oracle accounts are
ephemeral: clients fetch a signed VAA from
[Hermes](https://hermes.pyth.network/) and post it to the Pyth
receiver program, which lands a `PriceUpdateV2` PDA you can read.
The recipe (Node):

```js
import { PythSolanaReceiver } from "@pythnetwork/pyth-solana-receiver";
import { HermesClient } from "@pythnetwork/hermes-client";

const hermes = new HermesClient("https://hermes.pyth.network");
const SOL_USD = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
const updates = await hermes.getLatestPriceUpdates([SOL_USD]);

const receiver = new PythSolanaReceiver({ connection, wallet });
const { sig, priceUpdateAccount } =
  await receiver.postPriceUpdate({ priceUpdateData: updates.binary.data });
console.log("PriceUpdateV2 pubkey:", priceUpdateAccount.toString());
```

**Verifying our offset parsing.** Once you have the pubkey:

```bash
PYTH_ACCOUNT=<pubkey> JANUS_RPC=https://api.devnet.solana.com \
  node scripts/devnet/pyth-verify.mjs
```

The script asserts the discriminator and prints feed_id / price /
exponent / posted_slot using the exact same byte offsets the
on-chain resolver does. If anything diverges, the resolver will
return `Invalid` until offsets are updated.

**Choosing `max_staleness_slots`.** When initializing a resolver
state, pick the staleness window based on how much price drift you
tolerate:

| Use case | `max_staleness_slots` | Reasoning |
|---|---|---|
| Long-horizon binary (weeks) | 7200 (≈ 1 hour @ 2 slots/s × 60 × 60) | Drift over the resolution window is dominated by macro moves, not micro noise |
| Hourly binaries | 600 (≈ 5 min) | Tight enough that a flash crash can't be retroactively exploited |
| High-frequency (1-min) markets | 60 (≈ 30 s) | Maximum freshness without bricking on missed publishes |

Devnet Pyth publishes typically lag by 30-60s; tighter windows will
fail more often. Test on devnet first with the actual feed's
historical lag distribution.

## Monitoring

- Explorer: `https://explorer.solana.com/address/<program_id>?cluster=devnet`
- Recent invocations: filter Solana Beach / Solscan by program ID
  for transaction history.
- For production traffic, push program invocations to a Helius
  webhook (devnet webhooks are free).

## Promoting to mainnet

When the entire roadmap is verified on devnet and CI is green:

1. Generate fresh mainnet keypairs (do **not** reuse devnet ones —
   the public keys would clash and the devnet ones may have been
   compromised in the open):

   ```bash
   for p in conditional_tokens lmsr_market lmsr_true_market \
            slot_height_resolver pyth_price_resolver market_factory; do
     solana-keygen new -o target/deploy/janus_${p}-keypair.json --force
   done
   bash scripts/sync-program-ids.sh
   ```

2. Update `DEVNET.md` → `MAINNET.md` (this file) with the new IDs.

3. Build and deploy with explicit `--url mainnet-beta`. Treat
   keypair files as cryptographic secrets at this point (move
   them to a hardware-wallet-controlled setup if possible).

4. Audit. Don't skip. Conditional-tokens holds collateral and is
   the highest-blast-radius program — Code4rena or OtterSec, two
   weeks minimum.

5. Surface the deployment via the SDK with mainnet IDs as default
   exports and devnet IDs as a `devnet` subexport.
