import { useEffect, useRef } from "react";
import type { CartPoleState } from "../model/cartpole";
import type { SimStore } from "../model/store";
import { fitCanvas, onFrame, paletteCached } from "./canvas";

const RAIL_VIS = 0.575;
const PULLEY_X = 0.62;
const FLOOR_Y = -0.92;

export interface SideView2DProps {
  store: SimStore;
  /** 台車の目標位置 [m] (表示のみ) */
  targetX?: number;
  className?: string;
  style?: React.CSSProperties;
}

/** 側面図 (sim-view.js の draw2D を移植)。rAF で store.latest を読む */
export function SideView2D({ store, targetX = 0, className, style }: SideView2DProps) {
  const ref = useRef<HTMLCanvasElement>(null);
  const target = useRef(targetX);
  target.current = targetX;
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    return onFrame(() => draw2D(canvas, store.latest, target.current));
  }, [store]);
  return <canvas ref={ref} className={`ipk-canvas ${className ?? ""}`} style={style} />;
}

type P = (x: number, y: number) => [number, number];

export function draw2D(canvas: HTMLCanvasElement, st: CartPoleState | null, targetX: number) {
  const C = paletteCached();
  const { ctx, w, h } = fitCanvas(canvas);
  if (!w || !h) return;
  const s = Math.min(w / 1.45, h / 1.95);
  const cx = w / 2;
  const cy = h * 0.5;
  const P: P = (x, y) => [cx + x * s, cy - y * s];
  ctx.clearRect(0, 0, w, h);

  // 床・支柱
  ctx.strokeStyle = C.line!;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(...P(-0.7, FLOOR_Y));
  ctx.lineTo(...P(0.7, FLOOR_Y));
  ctx.stroke();
  ctx.lineWidth = 5;
  for (const sx of [-PULLEY_X, PULLEY_X]) {
    ctx.beginPath();
    ctx.moveTo(...P(sx, -0.03));
    ctx.lineTo(...P(sx, FLOOR_Y));
    ctx.stroke();
  }

  // レール・目盛・ソフトリミット
  ctx.lineWidth = 4;
  ctx.strokeStyle = C.sub!;
  ctx.beginPath();
  ctx.moveTo(...P(-PULLEY_X, -0.02));
  ctx.lineTo(...P(PULLEY_X, -0.02));
  ctx.stroke();
  ctx.lineWidth = 1;
  ctx.strokeStyle = C.line!;
  for (let i = -5; i <= 5; i++) {
    const [px, py] = P(i / 10, -0.045);
    ctx.beginPath();
    ctx.moveTo(px, py);
    ctx.lineTo(px, py + (i % 5 ? 4 : 8));
    ctx.stroke();
  }
  ctx.setLineDash([3, 3]);
  ctx.strokeStyle = C.warn!;
  for (const lx of [-0.45, 0.45]) {
    ctx.beginPath();
    ctx.moveTo(...P(lx, 0.07));
    ctx.lineTo(...P(lx, -0.09));
    ctx.stroke();
  }
  ctx.setLineDash([]);
  ctx.fillStyle = C.sub!;
  ctx.font = "10px system-ui, sans-serif";
  ctx.textAlign = "center";
  ctx.fillText("ソフトリミット", ...P(0.45, -0.13));

  // ベルト・プーリ・モータ・ストッパ
  ctx.strokeStyle = C.line!;
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(...P(-PULLEY_X, 0.01));
  ctx.lineTo(...P(PULLEY_X, 0.01));
  ctx.moveTo(...P(-PULLEY_X, -0.05));
  ctx.lineTo(...P(PULLEY_X, -0.05));
  ctx.stroke();
  const rot = st ? st.x / 0.015 : 0;
  for (const px of [-PULLEY_X, PULLEY_X]) {
    const [qx, qy] = P(px, -0.02);
    ctx.fillStyle = C.panel2!;
    ctx.strokeStyle = C.sub!;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(qx, qy, 0.03 * s, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(qx, qy);
    ctx.lineTo(qx + Math.cos(rot) * 0.03 * s, qy - Math.sin(rot) * 0.03 * s);
    ctx.stroke();
  }
  ctx.fillStyle = C.sub!;
  ctx.fillText("モータ", ...P(-PULLEY_X, 0.07));
  ctx.fillStyle = C.line!;
  for (const sx of [-1, 1]) {
    const [bx, by] = P(sx * RAIL_VIS - 0.01, 0.04);
    ctx.fillRect(bx, by, 0.02 * s, 0.1 * s);
  }

  if (!st || !Number.isFinite(st.x) || !Number.isFinite(st.theta)) return;
  const len = st.pendulumLength;

  // 台車の目標位置
  const [tx, ty] = P(targetX, -0.075);
  ctx.fillStyle = C.accent!;
  ctx.beginPath();
  ctx.moveTo(tx, ty);
  ctx.lineTo(tx - 5, ty + 8);
  ctx.lineTo(tx + 5, ty + 8);
  ctx.fill();

  // プラント実状態
  drawCart(ctx, P, s, len, st.x, st.theta, st.driveEnabled ? C.accent! : C.danger!, C.amber!, C.text!);

  if (!st.driveEnabled) {
    ctx.fillStyle = C.danger!;
    ctx.font = "bold 13px system-ui, sans-serif";
    ctx.fillText(st.fault ? "ドライブ遮断中 (FAULT)" : "ドライブ遮断中", ...P(0, 0.85));
  }
}

function drawCart(
  ctx: CanvasRenderingContext2D,
  P: P,
  s: number,
  len: number,
  x: number,
  th: number,
  fill: string,
  rodColor: string,
  pivotColor: string,
) {
  const [x0, y0] = P(x - 0.07, 0.035);
  const cw = 0.14 * s;
  const ch = 0.07 * s;
  const [px, py] = P(x, 0);
  const [ex, ey] = P(x + Math.sin(th) * len, Math.cos(th) * len);
  ctx.fillStyle = fill;
  ctx.fillRect(x0, y0, cw, ch);
  ctx.lineCap = "round";
  ctx.strokeStyle = rodColor;
  ctx.lineWidth = Math.max(4, 0.02 * s);
  ctx.beginPath();
  ctx.moveTo(px, py);
  ctx.lineTo(ex, ey);
  ctx.stroke();
  ctx.lineCap = "butt";
  ctx.fillStyle = pivotColor;
  ctx.beginPath();
  ctx.arc(px, py, Math.max(3, 0.012 * s), 0, Math.PI * 2);
  ctx.fill();
}
