import { describe, expect, it } from "vitest";
import type { SessionDescription } from "@ipemu/ipc-types";
import { layoutTopology } from "./Topology";

const part = (instance: string, kind: string, inputs: string[], outputs: string[]) =>
  ({ instance, kind, inputs: inputs.map((name) => ({ name })), outputs: outputs.map((name) => ({ name })) }) as unknown as SessionDescription["parts"][number];

describe("layoutTopology", () => {
  it("latency 0 の配線で層を分け、配線のない部品は最後の層", () => {
    const desc = {
      parts: [part("plant", "part", ["voltage"], ["x"]), part("enc", "part", ["input"], ["count"]), part("mcu", "builtin:cpu-virtual", ["enc0"], ["pwm0"]), part("snap", "builtin:snapshot", [], [])],
      wires: [
        { from: "plant.x", to: "enc.input", latency: 1, gain: 1, offset: 0 },
        { from: "enc.count", to: "mcu.enc0", latency: 0, gain: 1, offset: 0 },
        { from: "mcu.pwm0", to: "plant.voltage", latency: 0, gain: 1, offset: 0 },
      ],
      snapshotPorts: [],
    } as unknown as SessionDescription;
    const { boxes } = layoutTopology(desc);
    const x = Object.fromEntries(boxes.map((b) => [b.instance, b.x]));
    expect(x.enc).toBeLessThan(x.mcu!);
    expect(x.mcu).toBeLessThan(x.plant!);
    expect(x.snap).toBeGreaterThan(x.plant!);
  });
});
