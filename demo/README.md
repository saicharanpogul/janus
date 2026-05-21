# Janus demo

Bloomberg-terminal-styled web UI for the Janus binary-markets primitive
on Solana. Connects to **devnet** by default.

## What it does

- **Portfolio** (`/`): a big TOTAL VALUE hero stat + table of your
  YES/NO positions across every Janus market. Implied YES prices come
  straight from the LMSR pool's reserves.
- **All Markets** (`/markets`): every registered market on devnet,
  filterable + sortable. Click any row to trade.
- **Market detail** (`/markets/[market]`): hero YES price, market
  metadata, your balances, and a trade panel with split / buy YES /
  buy NO actions. Quotes show estimated output, min received after
  slippage, and price impact.
- **Create** (`/create`): one-form flow to spin up a fresh market with
  a slot-height resolver. Optionally mints a throwaway test
  collateral so you can demo without procuring USDC.

## Running locally

```bash
cd demo
pnpm install        # or: npm install / yarn
pnpm dev            # starts on http://localhost:3000
```

Connect Phantom or Solflare set to **devnet**, request an airdrop:

```bash
solana airdrop 2 <your-wallet> --url devnet
```

Then create a market with "Mint a fresh test collateral" checked — no
external USDC needed.

## Stack

- Next.js 14 (app router, TypeScript)
- Tailwind CSS — Bloomberg-inspired palette (white canvas, pale-green
  gain markers, accent orange, near-black ink)
- `@solana/wallet-adapter-react` + `react-ui` for wallets
- `@janus/sdk` (file dependency on `../sdk`)
- Devnet RPC default: `https://api.devnet.solana.com`. Override via
  `NEXT_PUBLIC_RPC` (see `.env.example`).

## Production build

```bash
pnpm build && pnpm start
```

`pnpm build` runs a `prebuild` step that first builds `../sdk` (since
`sdk/dist/` is git-ignored), then `next build`.

To switch to a different RPC (Helius, Triton, your own), set
`NEXT_PUBLIC_RPC` before building:

```bash
NEXT_PUBLIC_RPC=https://mainnet.helius-rpc.com/?api-key=... pnpm build
```

## Deploy to Vercel

```bash
vercel --cwd demo
```

The bundled `demo/vercel.json` overrides Vercel's install step to
build the sibling SDK first (`cd ../sdk && pnpm install && pnpm
build`) before installing the demo. Without this, the demo's
`file:../sdk` dependency resolves to a directory missing its
compiled `dist/`, and Next.js fails on the `@janus/sdk` import.

If you set Vercel's "Root Directory" in the project settings, point
it at `demo` so the bundled `vercel.json` is picked up.

For mainnet, also update the program IDs in
`../sdk/src/constants.ts` and rebuild the SDK (`cd ../sdk && pnpm build`).

## Architecture notes

- All on-chain reads are direct `connection.getAccountInfo` /
  `getProgramAccounts` calls — no indexer, no off-chain trade log.
- Position value is mark-to-market via the LMSR implied probability.
  Real P&L (cost basis) would need either a local trade log or an
  off-chain indexer; left to a future iteration.
- The market-factory's `Registration` account is the canonical "list
  of all markets" source. `fetchAllRegistrations` scans by
  `dataSize: 216` filter.
- The trade panel composes raw `splitIx` / `swapIx` calls from the
  SDK, signs with the connected wallet, sends, and refreshes on
  confirmation.
