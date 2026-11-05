/**
 * state フレーム (ポート値の配列) → sim-view が描く CartPoleState への写像。
 * ポート名は manifest の builtin:snapshot の並びから引く。無いポートは NaN / false になる。
 */
import type { SessionDescription, StateFrame } from "@ipemu/ipc-types";

export interface CartPoleState {
  simSeconds: number;
  x: number;
  theta: number;
  xDot: number;
  thetaDot: number;
  volts: number;
  driveEnabled: boolean;
  fault: boolean;
  pwm: number;
  execNs: number;
  overruns: number;
  linkOut: number;
  linkSpikes: number;
  pendulumLength: number;
  running: boolean;
  epoch: number;
}

/** 既定のポート対応 (dev/cartpole.manifest.json の snapshot)。UI バンドルごとに差し替えられる */
export const DEFAULT_PORT_MAP = {
  x: "plant.x",
  theta: "plant.theta",
  xDot: "plant.x-dot",
  thetaDot: "plant.theta-dot",
  volts: "driver.voltage",
  driveEnabled: "driver.drive-enabled",
  fault: "driver.fault",
  pwm: "mcu.pwm0",
  execNs: "mcu.exec-ns",
  overruns: "mcu.overruns",
  linkOut: "link-mon.out",
  linkSpikes: "link-mon.spikes",
} as const;

export type PortMap = { [K in keyof typeof DEFAULT_PORT_MAP]: string };

export class CartPoleMapper {
  private readonly idx: Partial<Record<keyof PortMap, number>> = {};
  readonly pendulumLength: number;

  constructor(desc: SessionDescription, map: PortMap = DEFAULT_PORT_MAP) {
    for (const k of Object.keys(map) as (keyof PortMap)[]) {
      const i = desc.snapshotPorts.indexOf(map[k]);
      if (i >= 0) this.idx[k] = i;
    }
    const plant = desc.parts.find((p) => p.instance === "plant");
    const len = (plant?.params as { pendulumLength?: unknown } | undefined)?.pendulumLength;
    this.pendulumLength = typeof len === "number" && len > 0 ? len : 0.6;
  }

  map(f: StateFrame): CartPoleState {
    const v = (k: keyof PortMap) => {
      const i = this.idx[k];
      return i === undefined ? NaN : (f.values[i] ?? NaN);
    };
    return {
      simSeconds: f.tNs * 1e-9,
      x: v("x"),
      theta: v("theta"),
      xDot: v("xDot"),
      thetaDot: v("thetaDot"),
      volts: v("volts"),
      driveEnabled: v("driveEnabled") > 0.5,
      fault: v("fault") > 0.5,
      pwm: v("pwm"),
      execNs: v("execNs"),
      overruns: v("overruns"),
      linkOut: v("linkOut"),
      linkSpikes: v("linkSpikes"),
      pendulumLength: this.pendulumLength,
      running: f.state === "running",
      epoch: f.epoch,
    };
  }
}

export const wrapAngle = (a: number): number => {
  a = (a + Math.PI) % (2 * Math.PI);
  if (a < 0) a += 2 * Math.PI;
  return a - Math.PI;
};
