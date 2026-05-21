"use client";

import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import Link from "next/link";
import { useCallback, useState, useEffect } from "react";
import {
  PublicKey,
  Transaction,
  Keypair,
} from "@solana/web3.js";
import {
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountIdempotentInstruction,
  createInitializeMintInstruction,
  createMintToInstruction,
  TOKEN_PROGRAM_ID,
  MINT_SIZE,
  getMinimumBalanceForRentExemptMint,
  AccountLayout,
} from "@solana/spl-token";
import { SystemProgram } from "@solana/web3.js";

import { createMarketWithSlotResolver } from "@janus/sdk";

export default function CreatePage() {
  const { connection } = useConnection();
  const wallet = useWallet();

  const [collateralMintStr, setCollateralMintStr] = useState("");
  const [deadlineSlot, setDeadlineSlot] = useState("");
  const [resolutionOutcome, setResolutionOutcome] = useState<1 | 2 | 3>(1);
  const [subsidyStr, setSubsidyStr] = useState("100");
  const [feeBps, setFeeBps] = useState(30);
  const [question, setQuestion] = useState("");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [currentSlot, setCurrentSlot] = useState(0);

  // Mintable test collateral
  const [mintTestColl, setMintTestColl] = useState(false);

  useEffect(() => {
    let cancelled = false;
    connection.getSlot("confirmed").then((s) => {
      if (!cancelled) {
        setCurrentSlot(s);
        if (!deadlineSlot) {
          // Default: 1 hour out (≈ 7200 slots at 2 slots/s)
          setDeadlineSlot((s + 7200).toString());
        }
      }
    });
    return () => {
      cancelled = true;
    };
  }, [connection, deadlineSlot]);

  const submit = useCallback(async () => {
    if (!wallet.publicKey || !wallet.signTransaction) return;
    setError(null);
    setResult(null);
    setBusy(true);
    try {
      const owner = wallet.publicKey;
      let collateralMint: PublicKey;
      const setupIxs: any[] = [];
      let mintSigner: Keypair | null = null;

      if (mintTestColl) {
        // Create a fresh test collateral mint owned by the user.
        mintSigner = Keypair.generate();
        const rent = await getMinimumBalanceForRentExemptMint(connection);
        setupIxs.push(
          SystemProgram.createAccount({
            fromPubkey: owner,
            newAccountPubkey: mintSigner.publicKey,
            space: MINT_SIZE,
            lamports: rent,
            programId: TOKEN_PROGRAM_ID,
          }),
          createInitializeMintInstruction(mintSigner.publicKey, 6, owner, null, TOKEN_PROGRAM_ID),
        );
        collateralMint = mintSigner.publicKey;
        // Mint a generous balance to the user's ATA.
        const ata = getAssociatedTokenAddressSync(collateralMint, owner, true);
        setupIxs.push(
          createAssociatedTokenAccountIdempotentInstruction(owner, ata, owner, collateralMint),
          createMintToInstruction(collateralMint, ata, owner, 1_000_000_000_000n /* 1e6 USDC-like */),
        );
      } else {
        try {
          collateralMint = new PublicKey(collateralMintStr.trim());
        } catch {
          throw new Error("Invalid collateral mint address");
        }
      }

      const userCollateralAta = getAssociatedTokenAddressSync(collateralMint, owner, true);
      setupIxs.push(
        createAssociatedTokenAccountIdempotentInstruction(
          owner,
          userCollateralAta,
          owner,
          collateralMint,
        ),
      );

      const subsidy = BigInt(Math.floor(parseFloat(subsidyStr) * 1e6));
      if (!(subsidy > 0n)) throw new Error("Subsidy must be > 0");
      const deadline = BigInt(deadlineSlot);
      if (deadline <= BigInt(currentSlot))
        throw new Error("Deadline slot must be in the future");

      const questionHash = question.trim()
        ? new TextEncoder().encode(question.trim()).slice(0, 32)
        : undefined;
      const paddedHash = questionHash
        ? Uint8Array.from(Array.from({ length: 32 }, (_, i) => questionHash[i] ?? 0))
        : undefined;

      const created = createMarketWithSlotResolver({
        payer: owner,
        authority: owner,
        collateralMint,
        creatorCollateral: userCollateralAta,
        deadlineSlot: deadline,
        resolutionOutcome,
        subsidy,
        feeBps,
        questionHash: paddedHash,
      });

      // Two transactions: setup + main flow. Most wallets fail with 17+
      // ix in a single tx. Split conservatively.
      const blockhash = await connection.getLatestBlockhash();

      const sigs: string[] = [];
      // tx1: setup
      if (setupIxs.length > 0) {
        const tx1 = new Transaction().add(...setupIxs);
        tx1.feePayer = owner;
        tx1.recentBlockhash = blockhash.blockhash;
        if (mintSigner) tx1.partialSign(mintSigner);
        const signed1 = await wallet.signTransaction(tx1);
        const sig1 = await connection.sendRawTransaction(signed1.serialize());
        await connection.confirmTransaction(sig1, "confirmed");
        sigs.push(sig1);
      }
      // tx2: market creation flow (SDK already splits internally)
      const tx2 = new Transaction().add(...created.instructions);
      tx2.feePayer = owner;
      tx2.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
      const signed2 = await wallet.signTransaction(tx2);
      const sig2 = await connection.sendRawTransaction(signed2.serialize());
      await connection.confirmTransaction(sig2, "confirmed");
      sigs.push(sig2);

      setResult(created.market.toBase58());
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }, [
    wallet,
    connection,
    collateralMintStr,
    mintTestColl,
    deadlineSlot,
    resolutionOutcome,
    subsidyStr,
    feeBps,
    question,
    currentSlot,
  ]);

  return (
    <div className="max-w-3xl mx-auto px-6 py-8">
      <div className="flex items-center gap-6 border-b border-line mb-6 -mt-2">
        <Link href="/" className="text-sm text-muted hover:text-black pb-3">Portfolio</Link>
        <Link href="/markets" className="text-sm text-muted hover:text-black pb-3">All Markets</Link>
        <div className="text-sm font-semibold pb-3 border-b-2 border-black">Create</div>
      </div>

      <h1 className="text-3xl font-extrabold tracking-tight mb-1">Create a new market</h1>
      <p className="text-sm text-muted mb-8">
        Slot-height resolver. The market resolves to the outcome you pick when
        the chain reaches your deadline slot. Use a fresh test collateral
        mint for a quick demo.
      </p>

      {result ? (
        <SuccessPanel market={result} onReset={() => setResult(null)} />
      ) : (
        <form
          className="space-y-6"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          {/* Collateral */}
          <fieldset>
            <legend className="stat-label mb-2">COLLATERAL</legend>
            <label className="flex items-center gap-2 text-sm mb-3">
              <input
                type="checkbox"
                checked={mintTestColl}
                onChange={(e) => setMintTestColl(e.target.checked)}
              />
              Mint a fresh test collateral (1M tokens to your wallet)
            </label>
            {!mintTestColl && (
              <input
                className="input"
                placeholder="Collateral mint pubkey…"
                value={collateralMintStr}
                onChange={(e) => setCollateralMintStr(e.target.value)}
                required={!mintTestColl}
              />
            )}
          </fieldset>

          {/* Question */}
          <fieldset>
            <legend className="stat-label mb-2">QUESTION (optional)</legend>
            <input
              className="input"
              placeholder='e.g. "Will SOL > $300 by end of Q3?"'
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              maxLength={32}
            />
            <div className="text-[10px] text-muted mt-1">
              First 32 bytes are hashed into the on-chain registration.
            </div>
          </fieldset>

          {/* Deadline + outcome */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <fieldset>
              <legend className="stat-label mb-2">DEADLINE SLOT</legend>
              <input
                className="input"
                type="number"
                value={deadlineSlot}
                onChange={(e) => setDeadlineSlot(e.target.value)}
              />
              <div className="text-[10px] text-muted mt-1">
                Current slot: {currentSlot.toLocaleString()}. Default: +7200 (≈ 1 hr).
              </div>
            </fieldset>
            <fieldset>
              <legend className="stat-label mb-2">RESOLUTION OUTCOME</legend>
              <select
                className="input"
                value={resolutionOutcome}
                onChange={(e) =>
                  setResolutionOutcome(parseInt(e.target.value) as 1 | 2 | 3)
                }
              >
                <option value={1}>YES wins at deadline</option>
                <option value={2}>NO wins at deadline</option>
                <option value={3}>INVALID (refund via merge)</option>
              </select>
            </fieldset>
          </div>

          {/* Subsidy + fee */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <fieldset>
              <legend className="stat-label mb-2">SUBSIDY (collateral)</legend>
              <input
                className="input"
                type="number"
                step="0.000001"
                min="0.000001"
                value={subsidyStr}
                onChange={(e) => setSubsidyStr(e.target.value)}
              />
              <div className="text-[10px] text-muted mt-1">
                Seeds equal YES + NO reserves on the LMSR pool.
              </div>
            </fieldset>
            <fieldset>
              <legend className="stat-label mb-2">SWAP FEE (bps)</legend>
              <input
                className="input"
                type="number"
                min={0}
                max={1000}
                value={feeBps}
                onChange={(e) => setFeeBps(parseInt(e.target.value, 10))}
              />
              <div className="text-[10px] text-muted mt-1">
                100 bps = 1%. Max 1000 (10%).
              </div>
            </fieldset>
          </div>

          {error && (
            <div className="bg-loss-soft border border-loss text-xs p-3 break-all">
              {error}
            </div>
          )}

          <div className="flex gap-3">
            <button
              type="submit"
              className="btn-primary"
              disabled={busy || !wallet.publicKey}
            >
              {busy ? "Creating…" : !wallet.publicKey ? "Connect wallet" : "Create market"}
            </button>
            <Link href="/markets" className="btn-outline flex items-center">
              Cancel
            </Link>
          </div>
        </form>
      )}
    </div>
  );
}

function SuccessPanel({ market, onReset }: { market: string; onReset: () => void }) {
  return (
    <div className="border border-gain bg-gain-soft p-6">
      <div className="stat-label text-ink mb-2">MARKET CREATED</div>
      <div className="font-mono text-sm break-all">{market}</div>
      <div className="flex gap-3 mt-6">
        <Link href={`/markets/${market}`} className="btn-primary flex items-center">
          View market
        </Link>
        <button className="btn-outline" onClick={onReset}>
          Create another
        </button>
      </div>
    </div>
  );
}
