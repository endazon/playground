import { describe, expect, it } from "vitest";
import type { ScopeFrame } from "@ipemu/ipc-types";
import { ScopeBuffer } from "./store";

describe("ScopeBuffer", () => {
  it("scope フレームを時系列に展開し、窓の外を捨てる", () => {
    const b = new ScopeBuffer(2, 1.0);
    const f = (t0: number, n: number): ScopeFrame => ({
      subscriptionId: 1,
      epoch: 0,
      t0Ns: t0,
      periodNs: 100_000_000,
      channels: [Array.from({ length: n }, (_, i) => i), Array.from({ length: n }, () => 7)],
    });
    b.pushFrame(f(0, 5));
    expect(b.samples).toHaveLength(5);
    expect(b.samples[2]).toEqual({ t: 0.2, values: [2, 7] });
    b.pushFrame(f(5e9, 5));
    // 窓 1.0s + 0.5s: 5.0..5.4 だけ残る
    expect(b.samples.every((s) => s.t >= 3.5)).toBe(true);
    expect(b.samples).toHaveLength(5);
  });
  it("epoch が変わったら捨てる", () => {
    const b = new ScopeBuffer(1, 10);
    b.push(1, [1], 0);
    b.push(2, [2], 0);
    b.push(0.1, [3], 1);
    expect(b.samples).toHaveLength(1);
    expect(b.epoch).toBe(1);
  });
});
