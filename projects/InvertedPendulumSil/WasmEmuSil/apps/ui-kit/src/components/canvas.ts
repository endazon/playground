/** canvas の共通ヘルパ (sim-view.js から移植) */
export function palette(): Record<string, string> {
  const cs = getComputedStyle(document.documentElement);
  const out: Record<string, string> = {};
  for (const k of ["bg", "panel", "panel2", "line", "grid", "text", "sub", "accent", "amber", "violet", "pink", "ok", "warn", "danger"]) {
    out[k] = cs.getPropertyValue(`--${k}`).trim();
  }
  return out;
}

let cachedPalette: Record<string, string> | null = null;
let cachedAt = 0;
export function paletteCached(): Record<string, string> {
  const now = performance.now();
  if (!cachedPalette || now - cachedAt > 1000) {
    cachedPalette = palette();
    cachedAt = now;
  }
  return cachedPalette;
}

export function fitCanvas(c: HTMLCanvasElement): { ctx: CanvasRenderingContext2D; w: number; h: number } {
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  const w = c.clientWidth;
  const h = c.clientHeight;
  if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) {
    c.width = Math.round(w * dpr);
    c.height = Math.round(h * dpr);
  }
  const ctx = c.getContext("2d")!;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  return { ctx, w, h };
}

/** 共有の rAF ループ。登録された描画関数を毎フレーム呼ぶ */
const frameFns = new Set<() => void>();
let rafId = 0;
function loop() {
  for (const f of frameFns) {
    try {
      f();
    } catch (e) {
      console.error(e);
    }
  }
  rafId = frameFns.size ? requestAnimationFrame(loop) : 0;
}
export function onFrame(fn: () => void): () => void {
  frameFns.add(fn);
  if (!rafId) rafId = requestAnimationFrame(loop);
  return () => {
    frameFns.delete(fn);
  };
}
