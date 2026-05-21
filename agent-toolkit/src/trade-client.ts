// High-level trade client: translates an `Action` into one or more SDK
// instruction calls, signs with the agent's keypair, sends, confirms,
// and returns an `ActionResult`.

import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  AccountLayout,
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  createMarketWithSlotResolver,
  initializePythPriceResolverIx,
  splitIx,
  swapIx,
  redeemIx,
  trueBuyIx,
  trueSellIx,
  deriveTruePoolPda,
  deriveTruePoolCollateralVaultPda,
  derivePoolYesVaultPda,
  derivePoolNoVaultPda,
} from "@janus/sdk";

import type {
  Action,
  ActionResult,
  CreateMarketAction,
  MarketSnapshot,
} from "./types.js";

export interface TradeClientOpts {
  connection: Connection;
  /** Shared test collateral mint for the swarm session. */
  collateralMint: PublicKey;
}

export class TradeClient {
  private conn: Connection;
  private collateralMint: PublicKey;

  constructor(opts: TradeClientOpts) {
    this.conn = opts.connection;
    this.collateralMint = opts.collateralMint;
  }

  async execute(
    agent: Keypair,
    action: Action,
    markets: MarketSnapshot[],
  ): Promise<ActionResult> {
    try {
      switch (action.kind) {
        case "noop":
          return { ok: true };
        case "create-market":
          return await this.createMarket(agent, action);
        case "split":
          return await this.split(agent, action.market, action.amount);
        case "swap":
          return await this.swap(agent, action, markets);
        case "buy":
        case "sell":
          return await this.buyOrSell(agent, action, markets);
        case "redeem":
          return await this.redeem(agent, action, markets);
      }
    } catch (e: any) {
      return { ok: false, error: e?.message ?? String(e) };
    }
  }

  // --------------------------------------------------------
  // create-market
  // --------------------------------------------------------

  private async createMarket(
    agent: Keypair,
    a: CreateMarketAction,
  ): Promise<ActionResult> {
    if (a.pool !== "cpmm") {
      return {
        ok: false,
        error: "true-LMSR market creation not yet wired in TradeClient (use raw SDK)",
      };
    }
    if (a.resolver !== "slot") {
      return {
        ok: false,
        error: "only slot resolver wired in TradeClient (use raw SDK for Pyth)",
      };
    }
    const userCollateral = getAssociatedTokenAddressSync(
      this.collateralMint,
      agent.publicKey,
      true,
    );
    const seedKey = agent.publicKey;
    const result = createMarketWithSlotResolver({
      payer: agent.publicKey,
      authority: agent.publicKey,
      collateralMint: this.collateralMint,
      creatorCollateral: userCollateral,
      deadlineSlot: a.deadlineSlot,
      resolutionOutcome: (a.outcomeAtOrAfter ?? 1) as 1 | 2 | 3,
      subsidy: a.subsidy,
      feeBps: a.feeBps ?? 100,
      questionHash: a.questionHash,
      // Use a fresh seed key per create attempt to avoid PDA collisions.
      resolverSeedKey: Keypair.generate().publicKey,
    });
    const sig = await this.sendIxs(agent, result.instructions);
    return { ok: true, signature: sig };
  }

  // --------------------------------------------------------
  // split (conditional-tokens)
  // --------------------------------------------------------

  private async split(
    agent: Keypair,
    market: PublicKey,
    amount: bigint,
  ): Promise<ActionResult> {
    const m = await this.fetchMarketHeader(market);
    if (!m) return { ok: false, error: "market not found" };
    const userCollateral = getAssociatedTokenAddressSync(
      m.collateralMint,
      agent.publicKey,
      true,
    );
    const userYes = getAssociatedTokenAddressSync(m.yesMint, agent.publicKey, true);
    const userNo = getAssociatedTokenAddressSync(m.noMint, agent.publicKey, true);
    const ixs: TransactionInstruction[] = [
      createAssociatedTokenAccountIdempotentInstruction(
        agent.publicKey,
        userYes,
        agent.publicKey,
        m.yesMint,
      ),
      createAssociatedTokenAccountIdempotentInstruction(
        agent.publicKey,
        userNo,
        agent.publicKey,
        m.noMint,
      ),
      splitIx({
        user: agent.publicKey,
        market,
        userCollateral,
        vault: m.vault,
        yesMint: m.yesMint,
        noMint: m.noMint,
        userYes,
        userNo,
        amount,
      }),
    ];
    const sig = await this.sendIxs(agent, ixs);
    return { ok: true, signature: sig };
  }

  // --------------------------------------------------------
  // swap (CPMM)
  // --------------------------------------------------------

  private async swap(
    agent: Keypair,
    a: Extract<Action, { kind: "swap" }>,
    markets: MarketSnapshot[],
  ): Promise<ActionResult> {
    const ms = markets.find((m) => m.market.equals(a.market));
    if (!ms) return { ok: false, error: "market snapshot missing" };
    if (ms.poolType !== "cpmm")
      return { ok: false, error: "swap only valid on CPMM pools" };
    const header = await this.fetchMarketHeader(a.market);
    if (!header) return { ok: false, error: "market not found" };

    const [yesVault] = derivePoolYesVaultPda(ms.pool);
    const [noVault] = derivePoolNoVaultPda(ms.pool);
    const userYes = getAssociatedTokenAddressSync(
      header.yesMint,
      agent.publicKey,
      true,
    );
    const userNo = getAssociatedTokenAddressSync(
      header.noMint,
      agent.publicKey,
      true,
    );

    const sig = await this.sendIxs(agent, [
      createAssociatedTokenAccountIdempotentInstruction(
        agent.publicKey,
        userYes,
        agent.publicKey,
        header.yesMint,
      ),
      createAssociatedTokenAccountIdempotentInstruction(
        agent.publicKey,
        userNo,
        agent.publicKey,
        header.noMint,
      ),
      swapIx({
        user: agent.publicKey,
        pool: ms.pool,
        yesVault,
        noVault,
        userInToken: a.direction === "yesToNo" ? userYes : userNo,
        userOutToken: a.direction === "yesToNo" ? userNo : userYes,
        amountIn: a.amountIn,
        minAmountOut: a.minAmountOut,
        direction: a.direction,
      }),
    ]);
    return { ok: true, signature: sig };
  }

  // --------------------------------------------------------
  // buy / sell (true-LMSR)
  // --------------------------------------------------------

  private async buyOrSell(
    agent: Keypair,
    a: Extract<Action, { kind: "buy" | "sell" }>,
    markets: MarketSnapshot[],
  ): Promise<ActionResult> {
    const ms = markets.find((m) => m.market.equals(a.market));
    if (!ms) return { ok: false, error: "market snapshot missing" };
    if (ms.poolType !== "true-lmsr")
      return { ok: false, error: "buy/sell only valid on true-LMSR pools" };
    const header = await this.fetchTrueMarketHeader(ms.pool);
    if (!header) return { ok: false, error: "true-LMSR pool not found" };

    const [collateralVault] = deriveTruePoolCollateralVaultPda(ms.pool);
    const userCollateral = getAssociatedTokenAddressSync(
      header.collateralMint,
      agent.publicKey,
      true,
    );
    const outcomeMint = a.side === "yes" ? header.yesMint : header.noMint;
    const userOutcome = getAssociatedTokenAddressSync(
      outcomeMint,
      agent.publicKey,
      true,
    );

    const ixs: TransactionInstruction[] = [
      createAssociatedTokenAccountIdempotentInstruction(
        agent.publicKey,
        userOutcome,
        agent.publicKey,
        outcomeMint,
      ),
    ];
    if (a.kind === "buy") {
      ixs.push(
        trueBuyIx({
          user: agent.publicKey,
          pool: ms.pool,
          collateralVault,
          userCollateral,
          outcomeMint,
          userOutcome,
          delta: a.delta,
          maxCollateralIn: a.maxCollateralIn,
          side: a.side,
        }),
      );
    } else {
      ixs.push(
        trueSellIx({
          user: agent.publicKey,
          pool: ms.pool,
          collateralVault,
          userCollateral,
          outcomeMint,
          userOutcome,
          delta: a.delta,
          minCollateralOut: a.minCollateralOut,
          side: a.side,
        }),
      );
    }
    const sig = await this.sendIxs(agent, ixs);
    return { ok: true, signature: sig };
  }

  // --------------------------------------------------------
  // redeem (post-resolution)
  // --------------------------------------------------------

  private async redeem(
    agent: Keypair,
    a: Extract<Action, { kind: "redeem" }>,
    markets: MarketSnapshot[],
  ): Promise<ActionResult> {
    const ms = markets.find((m) => m.market.equals(a.market));
    if (!ms) return { ok: false, error: "market snapshot missing" };
    const header = await this.fetchMarketHeader(a.market);
    if (!header) return { ok: false, error: "market not found" };
    const winningMint = a.side === "yes" ? header.yesMint : header.noMint;
    const userWinning = getAssociatedTokenAddressSync(
      winningMint,
      agent.publicKey,
      true,
    );
    const userCollateral = getAssociatedTokenAddressSync(
      header.collateralMint,
      agent.publicKey,
      true,
    );
    const sig = await this.sendIxs(agent, [
      createAssociatedTokenAccountIdempotentInstruction(
        agent.publicKey,
        userCollateral,
        agent.publicKey,
        header.collateralMint,
      ),
      redeemIx({
        user: agent.publicKey,
        market: a.market,
        userCollateral,
        vault: header.vault,
        winningMint,
        userWinning,
        amount: a.amount,
      }),
    ]);
    return { ok: true, signature: sig };
  }

  // --------------------------------------------------------
  // helpers
  // --------------------------------------------------------

  private async sendIxs(
    agent: Keypair,
    ixs: TransactionInstruction[],
  ): Promise<string> {
    const tx = new Transaction().add(...ixs);
    tx.feePayer = agent.publicKey;
    tx.recentBlockhash = (await this.conn.getLatestBlockhash()).blockhash;
    tx.sign(agent);
    const sig = await this.conn.sendRawTransaction(tx.serialize(), {
      skipPreflight: false,
    });
    await this.conn.confirmTransaction(sig, "confirmed");
    return sig;
  }

  // Conditional-tokens Market = 248 bytes. We only need the mints + vault.
  private async fetchMarketHeader(market: PublicKey): Promise<{
    collateralMint: PublicKey;
    yesMint: PublicKey;
    noMint: PublicKey;
    vault: PublicKey;
  } | null> {
    const info = await this.conn.getAccountInfo(market, "confirmed");
    if (!info || info.data.length !== 248) return null;
    return {
      collateralMint: new PublicKey(info.data.subarray(8, 40)),
      yesMint: new PublicKey(info.data.subarray(40, 72)),
      noMint: new PublicKey(info.data.subarray(72, 104)),
      vault: new PublicKey(info.data.subarray(104, 136)),
    };
  }

  // True-LMSR Pool = 264 bytes. Returns the mints + collateral mint
  // referenced by the pool.
  private async fetchTrueMarketHeader(pool: PublicKey): Promise<{
    collateralMint: PublicKey;
    yesMint: PublicKey;
    noMint: PublicKey;
  } | null> {
    const info = await this.conn.getAccountInfo(pool, "confirmed");
    if (!info || info.data.length !== 264) return null;
    return {
      collateralMint: new PublicKey(info.data.subarray(104, 136)),
      yesMint: new PublicKey(info.data.subarray(168, 200)),
      noMint: new PublicKey(info.data.subarray(200, 232)),
    };
  }
}
