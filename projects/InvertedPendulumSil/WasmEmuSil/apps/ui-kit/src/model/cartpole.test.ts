import { describe, expect, it } from "vitest";
import type { SessionDescription, StateFrame } from "@ipemu/ipc-types";
import { CartPoleMapper, wrapAngle } from "./cartpole";

const desc = {
  snapshotPorts: ["plant.theta", "plant.x", "driver.drive-enabled"],
  parts: [{ instance: "plant", params: { pendulumLength: 0.5 } }],
} as unknown as SessionDescription;

describe("CartPoleMapper", () => {
  it("ポート名の並びに依らず値を引き、無いものは NaN", () => {
    const m = new CartPoleMapper(desc);
    const f = { tNs: 2_000_000_000, values: [3.1, 0.2, 1], state: "running", epoch: 4 } as unknown as StateFrame;
    const s = m.map(f);
    expect(s.simSeconds).toBe(2);
    expect(s.theta).toBe(3.1);
    expect(s.x).toBe(0.2);
    expect(s.driveEnabled).toBe(true);
    expect(Number.isNaN(s.volts)).toBe(true);
    expect(s.pendulumLength).toBe(0.5);
    expect(s.running).toBe(true);
    expect(s.epoch).toBe(4);
  });
  it("wrapAngle", () => {
    expect(wrapAngle(Math.PI + 0.1)).toBeCloseTo(-Math.PI + 0.1);
    expect(wrapAngle(-0.2)).toBeCloseTo(-0.2);
  });
});
