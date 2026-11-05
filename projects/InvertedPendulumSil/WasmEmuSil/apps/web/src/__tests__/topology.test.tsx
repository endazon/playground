import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TopologyView, layoutColumns } from "../components/TopologyView";
import type { ComponentSpec, Wire } from "../api/types";

const components: ComponentSpec[] = [
  { instance: "plant", artifact: "plant-cartpole@0.1.0" },
  { instance: "enc", artifact: "encoder@0.1.0" },
  { instance: "mcu", kind: "builtin:cpu-virtual", capabilities: { net: { tcpConnect: ["h:1"] } } },
  { instance: "driver", artifact: "motor-driver@0.1.0" },
];
const wires: Wire[] = [
  { from: "plant.theta", to: "enc.input" },
  { from: "enc.count", to: "mcu.enc0", latency: 0 },
  { from: "mcu.pwm0", to: "driver.duty", latency: 0 },
  { from: "driver.voltage", to: "plant.voltage", latency: 0 }, // 閉路
];

describe("TopologyView", () => {
  it("assigns columns along the signal flow and breaks cycles", () => {
    const cols = layoutColumns(components, wires);
    expect(cols.get("plant")).toBe(0);
    expect(cols.get("enc")).toBe(1);
    expect(cols.get("mcu")).toBe(2);
    expect(cols.get("driver")).toBe(3);
  });

  it("renders one node per component and one path per wire", () => {
    const { container } = render(<TopologyView components={components} wires={wires} />);
    expect(container.querySelectorAll("g.node")).toHaveLength(4);
    expect(container.querySelectorAll("g.wire path")).toHaveLength(4);
    expect(container.querySelectorAll("g.wire.wire-sync")).toHaveLength(3);
    expect(container.textContent).toContain("mcu 🔐");
  });
});
