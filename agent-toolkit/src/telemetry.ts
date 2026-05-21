// Append-only JSONL telemetry: every action + result lands on disk and
// can be replayed for analysis. Also writes a periodic snapshot of
// per-agent P&L for the dashboard to read.

import { appendFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

import type { TelemetryEvent } from "./types.js";

export class Telemetry {
  private eventPath: string;
  private snapshotPath: string;

  constructor(opts: { dir: string; sessionId: string }) {
    const dir = resolve(opts.dir);
    mkdirSync(dir, { recursive: true });
    this.eventPath = `${dir}/events-${opts.sessionId}.jsonl`;
    this.snapshotPath = `${dir}/snapshot-${opts.sessionId}.json`;
  }

  emit(event: TelemetryEvent): void {
    // Stringify with bigint → string conversion.
    const line =
      JSON.stringify(event, (_, v) =>
        typeof v === "bigint"
          ? v.toString() + "n"
          : v instanceof Uint8Array
          ? "0x" + Buffer.from(v).toString("hex")
          : v,
      ) + "\n";
    appendFileSync(this.eventPath, line);
  }

  writeSnapshot(snapshot: unknown): void {
    writeFileSync(
      this.snapshotPath,
      JSON.stringify(
        snapshot,
        (_, v) => (typeof v === "bigint" ? v.toString() + "n" : v),
        2,
      ),
    );
  }

  get eventLogPath(): string {
    return this.eventPath;
  }
  get snapshotFilePath(): string {
    return this.snapshotPath;
  }
}
