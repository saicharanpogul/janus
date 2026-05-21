import { PublicKey } from "@solana/web3.js";

/** Pool curve type as observed on-chain. */
export type PoolType = "cpmm" | "true-lmsr";

export interface MarketSnapshot {
  // Identity
  market: PublicKey;
  pool: PublicKey;
  poolType: PoolType;
  status: number; // 0=Active, 1=ResolvedYes, 2=ResolvedNo, 3=Invalid
  creator: PublicKey;
  questionHash: Uint8Array;
  deadlineSlot: bigint;
  createdAtSlot: bigint;

  // Pricing
  /** Implied YES probability ∈ [0, 1] from the curve. */
  yesPrice: number;
  /** Total visible liquidity, in collateral base units. */
  liquidity: number;
  /** Fee in basis points (CPMM only — true-LMSR has 0 explicit fees). */
  feeBps: number;

  // Pool internals
  yesReserves: bigint;
  noReserves: bigint;
  /** True-LMSR only. */
  bLiquidity?: bigint;
  /** True-LMSR only. Vault holdings in collateral base units. */
  collateralVaultBalance?: bigint;
}

export interface AgentPosition {
  market: PublicKey;
  yesBalance: bigint;
  noBalance: bigint;
}

export type ActionKind =
  | "noop"
  | "create-market"
  | "split"
  | "swap"
  | "buy"
  | "sell"
  | "redeem";

export interface NoopAction {
  kind: "noop";
  reason?: string;
}

export interface CreateMarketAction {
  kind: "create-market";
  pool: PoolType;
  resolver: "slot" | "pyth";
  // Slot-resolver only
  outcomeAtOrAfter?: 1 | 2 | 3;
  // Pyth-resolver only
  feedId?: Uint8Array;
  thresholdPrice?: bigint;
  thresholdExpo?: number;
  comparison?: "gte" | "lt";
  // Common
  deadlineSlot: bigint;
  subsidy: bigint; // CPMM: per-side subsidy. True-LMSR: collateral subsidy >= ceil(b·ln2)
  /** True-LMSR liquidity parameter; ignored for CPMM. */
  b?: bigint;
  feeBps?: number;
  /** Optional 32-byte question tag. */
  questionHash?: Uint8Array;
}

export interface SplitAction {
  kind: "split";
  market: PublicKey;
  amount: bigint;
}

export interface SwapAction {
  kind: "swap";
  market: PublicKey;
  direction: "yesToNo" | "noToYes";
  amountIn: bigint;
  minAmountOut: bigint;
}

export interface BuyAction {
  kind: "buy";
  market: PublicKey;
  side: "yes" | "no";
  delta: bigint;
  maxCollateralIn: bigint;
}

export interface SellAction {
  kind: "sell";
  market: PublicKey;
  side: "yes" | "no";
  delta: bigint;
  minCollateralOut: bigint;
}

export interface RedeemAction {
  kind: "redeem";
  market: PublicKey;
  side: "yes" | "no";
  amount: bigint;
}

export type Action =
  | NoopAction
  | CreateMarketAction
  | SplitAction
  | SwapAction
  | BuyAction
  | SellAction
  | RedeemAction;

export interface AgentContext {
  /** Agent's own identity for telemetry. */
  agentId: string;
  /** Public key the agent uses to sign. */
  pubkey: PublicKey;
  /** Live USDC balance in base units. */
  collateral: bigint;
  /** Slot at decision time. */
  slot: bigint;
  /** All known markets. */
  markets: MarketSnapshot[];
  /** Agent's outstanding positions across markets. */
  positions: AgentPosition[];
  /** Tick number since swarm start. */
  tick: number;
}

export interface ActionResult {
  ok: boolean;
  /** Transaction signature if the action landed on-chain. */
  signature?: string;
  /** Error message if the action failed. */
  error?: string;
  /** Slot the action was executed at. */
  slot?: bigint;
}

export interface TelemetryEvent {
  ts: number; // Date.now()
  tick: number;
  agentId: string;
  action: Action;
  result: ActionResult;
}
