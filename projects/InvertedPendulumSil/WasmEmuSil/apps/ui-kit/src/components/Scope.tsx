import { useEffect, useRef } from "react";
import type { ScopeBuffer } from "../model/store";
import { fitCanvas, onFrame, paletteCached } from "./canvas";

export interface ScopeTrace {
  /** ScopeBuffer のチャネル index */
  channel: number;
  color: keyof PaletteKeys;
  width?: number;
  dash?: number[];
  /** 値の変換 (例: rad → deg) */
  transform?: (v: number) => number;
}
type PaletteKeys = { amber: 1; accent: 1; violet: 1; pink: 1; sub: 1; ok: 1; warn: 1; danger: 1; text: 1 };

export interface ScopeLane {
  title: string;
  lo: number;
  hi: number;
  /** レンジを自動で広げる候補 (lo/hi は ±この値) */
  autoRange?: number[];
  zero?: boolean;
  marks?: number[];
  /** 角度など: レンジ外や巻き戻りで線を途切れさせる */
  gapOut?: boolean;
  traces: ScopeTrace[];
}

export interface ScopeProps {
  buffer: ScopeBuffer;
  lanes: ScopeLane[];
  windowSec?: number;
  className?: string;
  style?: React.CSSProperties;
}

const niceRange = (m: number, steps: number[]) => steps.find((v) => v >= m) ?? steps[steps.length - 1]!;

/** 時系列スコープ (sim-view.js の drawScope を移植)。 */
export function Scope({ buffer, lanes, windowSec = 10, className, style }: ScopeProps) {
  const ref = useRef<HTMLCanvasElement>(null);
  const lanesRef = useRef(lanes);
  lanesRef.current = lanes;
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    return onFrame(() => drawScope(canvas, buffer, lanesRef.current, windowSec));
  }, [buffer, windowSec]);
  return <canvas ref={ref} className={`ipk-canvas ${className ?? ""}`} style={style} />;
}

export function drawScope(canvas: HTMLCanvasElement, buffer: ScopeBuffer, lanes: ScopeLane[], windowSec: number) {
  const C = paletteCached();
  const { ctx, w, h } = fitCanvas(canvas);
  if (!w || !h) return;
  ctx.clearRect(0, 0, w, h);
  const S = buffer.samples;
  if (!S.length) return;
  const tEnd = S[S.length - 1]!.t;
  const tStart = tEnd - windowSec;

  const padL = 44;
  const padR = 8;
  const gap = 8;
  const laneH = (h - gap * (lanes.length - 1)) / lanes.length;
  ctx.font = "11px system-ui, sans-serif";
  lanes.forEach((ln, i) => {
    let lo = ln.lo;
    let hi = ln.hi;
    if (ln.autoRange) {
      let m = 0;
      for (const tr of ln.traces) {
        for (const s of S) {
          const raw = s.values[tr.channel];
          if (raw === undefined || !Number.isFinite(raw)) continue;
          const v = tr.transform ? tr.transform(raw) : raw;
          if (Number.isFinite(v)) m = Math.max(m, Math.abs(v));
        }
      }
      hi = niceRange(m * 1.1, ln.autoRange);
      lo = ln.lo < 0 ? -hi : 0;
    }
    const top = i * (laneH + gap);
    const bottom = top + laneH;
    const X = (t: number) => padL + ((t - tStart) / windowSec) * (w - padL - padR);
    const Y = (v: number) => bottom - ((v - lo) / (hi - lo)) * (laneH - 4) - 2;

    ctx.fillStyle = C.panel2!;
    ctx.fillRect(padL, top, w - padL - padR, laneH);
    ctx.strokeStyle = C.grid!;
    ctx.lineWidth = 1;
    for (let g = Math.ceil(tStart); g <= tEnd; g++) {
      ctx.beginPath();
      ctx.moveTo(X(g), top);
      ctx.lineTo(X(g), bottom);
      ctx.stroke();
    }
    if (ln.zero) {
      ctx.strokeStyle = C.line!;
      ctx.beginPath();
      ctx.moveTo(padL, Y(0));
      ctx.lineTo(w - padR, Y(0));
      ctx.stroke();
    }
    for (const v of ln.marks ?? []) {
      ctx.strokeStyle = C.warn!;
      ctx.setLineDash([3, 3]);
      ctx.beginPath();
      ctx.moveTo(padL, Y(v));
      ctx.lineTo(w - padR, Y(v));
      ctx.stroke();
      ctx.setLineDash([]);
    }
    ctx.fillStyle = C.sub!;
    ctx.textAlign = "right";
    ctx.fillText(String(+hi.toPrecision(3)), padL - 5, top + 10);
    ctx.fillText(String(+lo.toPrecision(3)), padL - 5, bottom - 2);
    ctx.textAlign = "left";
    ctx.fillStyle = C.text!;
    ctx.fillText(ln.title, padL + 6, top + 13);

    ctx.save();
    ctx.beginPath();
    ctx.rect(padL, top, w - padL - padR, laneH);
    ctx.clip();
    for (const tr of ln.traces) {
      ctx.strokeStyle = C[tr.color] ?? C.text!;
      ctx.lineWidth = tr.width ?? 1.5;
      ctx.setLineDash(tr.dash ?? []);
      ctx.beginPath();
      let pen = false;
      let prev: number | null = null;
      for (const s of S) {
        if (s.t < tStart - 0.1) continue;
        const raw = s.values[tr.channel];
        const v = raw === undefined ? NaN : tr.transform ? tr.transform(raw) : raw;
        const ok =
          Number.isFinite(v) &&
          (!ln.gapOut || (v >= lo && v <= hi)) && // レンジ外は端に貼り付けず途切れさせる
          (!ln.gapOut || prev === null || Math.abs(v - prev) < (hi - lo) / 2); // ±180° の巻き戻りで線を引かない
        if (!ok) {
          pen = false;
          prev = Number.isFinite(v) ? v : null;
          continue;
        }
        const px = X(s.t);
        const py = Y(Math.max(lo, Math.min(hi, v)));
        if (pen) ctx.lineTo(px, py);
        else ctx.moveTo(px, py);
        pen = true;
        prev = v;
      }
      ctx.stroke();
    }
    ctx.setLineDash([]);
    ctx.restore();
  });
}
