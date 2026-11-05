/**
 * 描画用の状態置き場。state フレームは 50Hz で来るので React の再描画はさせず、
 * canvas 側が requestAnimationFrame で `latest` を読む (NetworkedSil の sim-view と同じ構え)。
 * React に見せたい低頻度の値 (実行状態など) は throttle して通知する。
 */
import type { ScopeFrame, StateFrame } from "@ipemu/ipc-types";
import type { CartPoleState } from "./cartpole";

export interface ScopeSample {
  t: number;
  values: number[];
}

export class ScopeBuffer {
  readonly samples: ScopeSample[] = [];
  epoch = -1;
  constructor(
    public readonly channels: number,
    public readonly windowSec: number,
  ) {}

  /** scope フレーム (チャネル別の列) を時系列サンプルに展開する */
  pushFrame(f: ScopeFrame): void {
    if (f.epoch !== this.epoch) {
      this.samples.length = 0;
      this.epoch = f.epoch;
    }
    const n = f.channels[0]?.length ?? 0;
    for (let k = 0; k < n; k++) {
      const t = (f.t0Ns + k * f.periodNs) * 1e-9;
      this.samples.push({ t, values: f.channels.map((ch) => ch[k] ?? NaN) });
    }
    this.trim();
  }

  /** state フレームからの低レート sample (scope 購読がないときの代替) */
  push(t: number, values: number[], epoch: number): void {
    if (epoch !== this.epoch) {
      this.samples.length = 0;
      this.epoch = epoch;
    }
    const last = this.samples[this.samples.length - 1];
    if (last && last.t === t) return;
    if (last && t < last.t) this.samples.length = 0;
    this.samples.push({ t, values });
    this.trim();
  }

  private trim() {
    const last = this.samples[this.samples.length - 1];
    if (!last) return;
    const cut = last.t - this.windowSec - 0.5;
    let i = 0;
    while (i < this.samples.length && this.samples[i]!.t < cut) i++;
    if (i > 0) this.samples.splice(0, i);
  }
}

type Listener = () => void;

export class SimStore {
  latest: CartPoleState | null = null;
  latestFrame: StateFrame | null = null;
  readonly scope: ScopeBuffer;
  private listeners = new Set<Listener>();
  private notifyScheduled = false;
  private version = 0;

  constructor(scopeChannels: number, windowSec = 10) {
    this.scope = new ScopeBuffer(scopeChannels, windowSec);
  }

  setState(s: CartPoleState, f: StateFrame): void {
    this.latest = s;
    this.latestFrame = f;
    this.scheduleNotify();
  }

  /** React (useSyncExternalStore) 向け。版番号だけを返し、値は latest から読む */
  subscribe = (fn: Listener): (() => void) => {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  };
  getVersion = (): number => this.version;

  private scheduleNotify() {
    if (this.notifyScheduled) return;
    this.notifyScheduled = true;
    // 50Hz のフレームを 5Hz 程度に間引いて React に渡す
    setTimeout(() => {
      this.notifyScheduled = false;
      this.version++;
      for (const l of this.listeners) l();
    }, 200);
  }
}
