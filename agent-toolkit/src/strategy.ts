// Abstract Strategy: an agent observes an `AgentContext` and returns an
// `Action`. Strategies are plain pure functions (state lives in the
// SwarmRunner) so they're easy to test in isolation.

import type { Action, AgentContext } from "./types.js";

export interface Strategy {
  /** Human-readable name for telemetry. */
  readonly name: string;
  /** Make a decision given the current world. */
  decide(ctx: AgentContext): Action | Promise<Action>;
  /**
   * Optional learning hook: called after an action executes with the
   * result. Strategies can update internal state (Q-table, model
   * weights, etc.) here.
   */
  observe?(ctx: AgentContext, action: Action, result: import("./types.js").ActionResult): void;
}
