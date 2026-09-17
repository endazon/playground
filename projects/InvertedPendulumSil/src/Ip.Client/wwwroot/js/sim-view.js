/* ============================================================
   描画モジュール — 側面図 / 3D ビュー / 時系列スコープ
   .NET 側から 50Hz で状態を受け取り、requestAnimationFrame で描く。
   物理計算も制御も持たない。ここが重くなっても制御ループは乱れない。
   ============================================================ */
"use strict";

const RAIL_VIS = 0.575, PULLEY_X = 0.62, FLOOR_Y = -0.92;
const WINDOW_SEC = 10;
const RAD2DEG = 180 / Math.PI;

const view = {
  dotnet: null,
  state: null,
  samples: [],
  lastT: -1,
  pushFx: null,
  three: null,
  raf: 0,
  colors: {},
  colorsAt: 0,
};

const $ = id => document.getElementById(id);
const wrapAngle = a => {
  a = (a + Math.PI) % (2 * Math.PI);
  if (a < 0) a += 2 * Math.PI;
  return a - Math.PI;
};

/* ---------- 入口 ---------- */

export function init(dotnetRef) {
  view.dotnet = dotnetRef;
  view.samples = [];
  view.lastT = -1;
  bind2D();
  init3D();
  if (!view.raf) view.raf = requestAnimationFrame(frame);
}

export function update(state) {
  // 壊れた状態を受け取ってもサンプル列を無限に伸ばさない (NaN は比較がすべて false になる)
  if (!state || !isFinite(state.simSeconds)) return;
  view.state = state;
  sample(state);
}

/** 3D ビューが動いているか。E2E から «CDN/同梱スクリプトが死んでいないか» を確認するために使う。 */
export function is3dActive() {
  return !!view.three;
}

export function pushEffect(dir) {
  view.pushFx = { dir, until: performance.now() + 350 };
}

export function dispose() {
  if (view.raf) cancelAnimationFrame(view.raf);
  view.raf = 0;
  view.dotnet = null;
  view.state = null;
  view.samples = [];
  view.lastT = -1;

  if (view.three) {
    try {
      // ジオメトリ/マテリアルは GC 対象にならないので明示的に解放する
      view.three.scene.traverse(obj => {
        if (obj.geometry) obj.geometry.dispose();
        const material = obj.material;
        if (Array.isArray(material)) material.forEach(m => m.dispose());
        else if (material) material.dispose();
      });
      view.three.detach();
      view.three.renderer.dispose();
    } catch { /* 破棄時の失敗は無視 */ }
    view.three = null;
  }
}

function notifyPush(dir) {
  pushEffect(dir);
  if (view.dotnet) view.dotnet.invokeMethodAsync("OnPush", dir);
}

/* ---------- 共通の描画ヘルパ ---------- */

function palette() {
  const now = performance.now();
  if (now - view.colorsAt < 1000) return view.colors;
  view.colorsAt = now;
  const cs = getComputedStyle(document.documentElement);
  for (const k of ["bg", "panel", "panel2", "line", "grid", "text", "sub",
                   "accent", "amber", "violet", "pink", "ok", "warn", "danger"]) {
    view.colors[k] = cs.getPropertyValue(`--${k}`).trim();
  }
  return view.colors;
}

function fitCanvas(c) {
  const dpr = Math.min(2, window.devicePixelRatio || 1), w = c.clientWidth, h = c.clientHeight;
  if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) {
    c.width = Math.round(w * dpr);
    c.height = Math.round(h * dpr);
  }
  const ctx = c.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  return { ctx, w, h };
}

/* ---------- 側面図 ---------- */

function bind2D() {
  const canvas = $("c2d");
  if (!canvas || canvas.dataset.bound) return;
  canvas.dataset.bound = "1";
  canvas.addEventListener("pointerup", e => {
    const r = e.currentTarget.getBoundingClientRect(), st = view.state;
    const s = Math.min(r.width / 1.45, r.height / 1.95);
    const len = st ? st.pendulumLength : 0.6;
    const tipX = st ? r.width / 2 + (st.x + Math.sin(st.theta) * len * 0.7) * s : r.width / 2;
    notifyPush((e.clientX - r.left) < tipX ? 1 : -1); // タップした側から押す
  });
}

function draw2D(st) {
  const canvas = $("c2d");
  if (!canvas) return;
  const C = palette(), { ctx, w, h } = fitCanvas(canvas);
  const s = Math.min(w / 1.45, h / 1.95), cx = w / 2, cy = h * 0.5;
  const P = (x, y) => [cx + x * s, cy - y * s];
  ctx.clearRect(0, 0, w, h);

  // 床・支柱
  ctx.strokeStyle = C.line; ctx.lineWidth = 2;
  ctx.beginPath(); ctx.moveTo(...P(-0.7, FLOOR_Y)); ctx.lineTo(...P(0.7, FLOOR_Y)); ctx.stroke();
  ctx.lineWidth = 5;
  for (const sx of [-PULLEY_X, PULLEY_X]) {
    ctx.beginPath(); ctx.moveTo(...P(sx, -0.03)); ctx.lineTo(...P(sx, FLOOR_Y)); ctx.stroke();
  }

  // レール・目盛・ソフトリミット
  ctx.lineWidth = 4; ctx.strokeStyle = C.sub;
  ctx.beginPath(); ctx.moveTo(...P(-PULLEY_X, -0.02)); ctx.lineTo(...P(PULLEY_X, -0.02)); ctx.stroke();
  ctx.lineWidth = 1; ctx.strokeStyle = C.line;
  for (let i = -5; i <= 5; i++) {
    const [px, py] = P(i / 10, -0.045);
    ctx.beginPath(); ctx.moveTo(px, py); ctx.lineTo(px, py + (i % 5 ? 4 : 8)); ctx.stroke();
  }
  ctx.setLineDash([3, 3]); ctx.strokeStyle = C.warn;
  for (const lx of [-0.45, 0.45]) {
    ctx.beginPath(); ctx.moveTo(...P(lx, 0.07)); ctx.lineTo(...P(lx, -0.09)); ctx.stroke();
  }
  ctx.setLineDash([]);
  ctx.fillStyle = C.sub; ctx.font = "10px system-ui, sans-serif"; ctx.textAlign = "center";
  ctx.fillText("ソフトリミット", ...P(0.45, -0.13));

  // ベルト・プーリ・モータ・ストッパ
  ctx.strokeStyle = C.line; ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(...P(-PULLEY_X, 0.01)); ctx.lineTo(...P(PULLEY_X, 0.01));
  ctx.moveTo(...P(-PULLEY_X, -0.05)); ctx.lineTo(...P(PULLEY_X, -0.05));
  ctx.stroke();
  const rot = st ? st.x / 0.015 : 0;
  for (const px of [-PULLEY_X, PULLEY_X]) {
    const [qx, qy] = P(px, -0.02);
    ctx.fillStyle = C.panel2; ctx.strokeStyle = C.sub; ctx.lineWidth = 1.5;
    ctx.beginPath(); ctx.arc(qx, qy, 0.03 * s, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(qx, qy);
    ctx.lineTo(qx + Math.cos(rot) * 0.03 * s, qy - Math.sin(rot) * 0.03 * s); ctx.stroke();
  }
  ctx.fillStyle = C.sub; ctx.fillText("モータ", ...P(-PULLEY_X, 0.07));
  ctx.fillStyle = C.line;
  for (const sx of [-1, 1]) {
    const [bx, by] = P(sx * RAIL_VIS - 0.01, 0.04);
    ctx.fillRect(bx, by, 0.02 * s, 0.1 * s);
  }

  if (!st) return;
  const len = st.pendulumLength;

  // 台車の目標位置
  const [tx, ty] = P(st.targetX, -0.075);
  ctx.fillStyle = C.accent;
  ctx.beginPath(); ctx.moveTo(tx, ty); ctx.lineTo(tx - 5, ty + 8); ctx.lineTo(tx + 5, ty + 8); ctx.fill();

  // バックが見ている状態 (遅れた推定値)
  if (st.hasEstimate) {
    ctx.setLineDash([5, 4]); ctx.strokeStyle = C.sub; ctx.lineWidth = 1.5; ctx.globalAlpha = 0.9;
    drawCart(ctx, P, s, len, st.estX, st.estTheta, null, null);
    ctx.setLineDash([]); ctx.globalAlpha = 1;
  }

  // プラント実状態
  drawCart(ctx, P, s, len, st.x, st.theta, st.driveEnabled ? C.accent : C.danger, C.amber);

  // 外乱エフェクト
  if (view.pushFx && performance.now() < view.pushFx.until) {
    const d = view.pushFx.dir;
    const [ax, ay] = P(st.x + Math.sin(st.theta) * len * 0.7 - d * 0.16, Math.cos(st.theta) * len * 0.7);
    ctx.strokeStyle = C.pink; ctx.fillStyle = C.pink; ctx.lineWidth = 3;
    ctx.beginPath(); ctx.moveTo(ax, ay); ctx.lineTo(ax + d * 0.1 * s, ay); ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(ax + d * 0.13 * s, ay);
    ctx.lineTo(ax + d * 0.09 * s, ay - 6);
    ctx.lineTo(ax + d * 0.09 * s, ay + 6);
    ctx.fill();
  }

  if (!st.driveEnabled) {
    ctx.fillStyle = C.danger; ctx.font = "bold 13px system-ui, sans-serif";
    ctx.fillText("ドライブ遮断中", ...P(0, 0.85));
  }
}

function drawCart(ctx, P, s, len, x, th, fill, rodColor) {
  const [x0, y0] = P(x - 0.07, 0.035), cw = 0.14 * s, ch = 0.07 * s;
  const [px, py] = P(x, 0), [ex, ey] = P(x + Math.sin(th) * len, Math.cos(th) * len);
  if (fill) { ctx.fillStyle = fill; ctx.fillRect(x0, y0, cw, ch); }
  else ctx.strokeRect(x0, y0, cw, ch);
  ctx.lineCap = "round";
  if (rodColor) { ctx.strokeStyle = rodColor; ctx.lineWidth = Math.max(4, 0.02 * s); }
  ctx.beginPath(); ctx.moveTo(px, py); ctx.lineTo(ex, ey); ctx.stroke();
  ctx.lineCap = "butt";
  if (rodColor) {
    ctx.fillStyle = palette().text;
    ctx.beginPath(); ctx.arc(px, py, Math.max(3, 0.012 * s), 0, Math.PI * 2); ctx.fill();
  }
}

/* ---------- 3D ---------- */

function init3D() {
  const canvas = $("c3d");
  if (!canvas || view.three) return;
  if (!window.THREE) {
    canvas.style.display = "none";
    const fb = $("fallback3d");
    if (fb) fb.style.display = "block";
    return;
  }
  try {
    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
    renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(38, 1, 0.05, 50);
    scene.add(new THREE.HemisphereLight(0xffffff, 0x444455, 0.75));
    const sun = new THREE.DirectionalLight(0xffffff, 0.7);
    sun.position.set(1.5, 3, 2.5);
    scene.add(sun);

    const metal = new THREE.MeshPhongMaterial({ color: 0x8a94a3, shininess: 60 });
    const dark = new THREE.MeshPhongMaterial({ color: 0x3a414c, shininess: 30 });
    const box = (w, h, d, mat, x, y, z) => {
      const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), mat);
      m.position.set(x, y, z);
      scene.add(m);
      return m;
    };

    const floor = new THREE.GridHelper(2.4, 24, 0x55606e, 0x333a44);
    floor.position.y = FLOOR_Y;
    scene.add(floor);
    for (const sx of [-PULLEY_X, PULLEY_X]) box(0.04, -FLOOR_Y, 0.12, dark, sx, FLOOR_Y / 2 - 0.03, -0.07);
    box(1.28, 0.025, 0.05, metal, 0, -0.02, -0.05);
    box(1.24, 0.006, 0.012, dark, 0, 0.01, -0.09);
    box(1.24, 0.006, 0.012, dark, 0, -0.05, -0.09);
    for (const sx of [-1, 1]) box(0.02, 0.1, 0.1, dark, sx * RAIL_VIS, 0.01, -0.05);
    const pulleys = [-PULLEY_X, PULLEY_X].map(px => {
      const g = new THREE.Mesh(new THREE.CylinderGeometry(0.03, 0.03, 0.03, 24), metal);
      g.rotation.x = Math.PI / 2;
      g.position.set(px, -0.02, -0.09);
      scene.add(g);
      return g;
    });
    const motor = new THREE.Mesh(new THREE.CylinderGeometry(0.038, 0.038, 0.09, 24), dark);
    motor.rotation.x = Math.PI / 2;
    motor.position.set(-PULLEY_X, -0.02, -0.16);
    scene.add(motor);

    const cartMat = new THREE.MeshPhongMaterial({ color: 0x4fd1c5, shininess: 40 });
    const cart = box(0.14, 0.07, 0.1, cartMat, 0, 0, -0.02);
    const rodMat = new THREE.MeshPhongMaterial({ color: 0xffb347, shininess: 50 });
    const pend = new THREE.Group();
    scene.add(pend);
    const rod = new THREE.Mesh(new THREE.BoxGeometry(0.02, 1, 0.014), rodMat);
    pend.add(rod);
    const shaft = new THREE.Mesh(new THREE.CylinderGeometry(0.01, 0.01, 0.05, 12), metal);
    shaft.rotation.x = Math.PI / 2;
    shaft.position.z = -0.02;
    pend.add(shaft);

    const ghostMat = new THREE.MeshBasicMaterial({ color: 0x98a0ac, wireframe: true, transparent: true, opacity: 0.55 });
    const gCart = new THREE.Mesh(new THREE.BoxGeometry(0.14, 0.07, 0.1), ghostMat);
    scene.add(gCart);
    const gPend = new THREE.Group();
    scene.add(gPend);
    const gRod = new THREE.Mesh(new THREE.BoxGeometry(0.02, 1, 0.014), ghostMat);
    gPend.add(gRod);

    let az = 0.55, ev = 0.18, drag = null, moved = 0;
    const listeners = [];
    const on = (name, handler) => { canvas.addEventListener(name, handler); listeners.push([name, handler]); };
    on("pointerdown", e => {
      drag = { x: e.clientX, y: e.clientY, az, ev };
      moved = 0;
      canvas.setPointerCapture(e.pointerId);
    });
    on("pointermove", e => {
      if (!drag) return;
      moved = Math.max(moved, Math.hypot(e.clientX - drag.x, e.clientY - drag.y));
      az = drag.az + (e.clientX - drag.x) * 0.008;
      ev = Math.max(-0.3, Math.min(1.1, drag.ev + (e.clientY - drag.y) * 0.006));
    });
    on("pointerup", e => {
      if (drag && moved < 5) {
        const r = canvas.getBoundingClientRect();
        notifyPush(e.clientX - r.left < r.width / 2 ? 1 : -1);
      }
      drag = null;
    });
    ["pointercancel", "pointerleave"].forEach(n => on(n, () => { drag = null; }));

    view.three = {
      renderer, scene, camera, cart, cartMat, pend, rod, gCart, gPend, gRod, pulleys,
      sizedWidth: 0, sizedHeight: 0,
      angles: () => ({ az, ev }),
      detach: () => listeners.forEach(([name, handler]) => canvas.removeEventListener(name, handler)),
    };
  } catch {
    canvas.style.display = "none";
    const fb = $("fallback3d");
    if (fb) fb.style.display = "block";
  }
}

function render3D(st) {
  const t = view.three;
  if (!t) return;
  const c = t.renderer.domElement, w = c.clientWidth, h = c.clientHeight;
  // 幅だけを見ていると、高さだけ変わるレイアウトで camera.aspect が古いまま歪む
  if (w && h && (t.sizedWidth !== w || t.sizedHeight !== h)) {
    t.sizedWidth = w;
    t.sizedHeight = h;
    t.renderer.setSize(w, h, false);
    t.camera.aspect = w / h;
    t.camera.updateProjectionMatrix();
  }
  const { az, ev } = t.angles(), R = 2.7;
  t.camera.position.set(Math.sin(az) * Math.cos(ev) * R, Math.sin(ev) * R, Math.cos(az) * Math.cos(ev) * R);
  t.camera.lookAt(0, -0.05, 0);

  if (st) {
    const len = st.pendulumLength;
    for (const r of [t.rod, t.gRod]) { r.scale.y = len; r.position.y = len / 2; }
    t.cart.position.x = st.x;
    t.cartMat.color.setHex(st.driveEnabled ? 0x4fd1c5 : 0xef5b5b);
    t.pend.position.set(st.x, 0, 0.05);
    t.pend.rotation.z = -st.theta;
    t.pulleys.forEach(p => { p.rotation.y = st.x / 0.015; });

    t.gCart.visible = t.gPend.visible = st.hasEstimate;
    if (st.hasEstimate) {
      t.gCart.position.set(st.estX, 0, -0.02);
      t.gPend.position.set(st.estX, 0, 0.05);
      t.gPend.rotation.z = -st.estTheta;
    }
  }
  t.renderer.render(t.scene, t.camera);
}

/* ---------- 時系列スコープ ---------- */

function sample(st) {
  if (!st || st.simSeconds === view.lastT) return;
  if (st.simSeconds < view.lastT) view.samples.length = 0; // プラント初期化で時間が戻ったら捨てる
  view.lastT = st.simSeconds;
  view.samples.push({
    t: st.simSeconds,
    th: wrapAngle(st.theta) * RAD2DEG,
    x: st.x,
    v: st.volts,
    e2e: st.latencyMs,
    thE: st.hasEstimate ? st.estTheta * RAD2DEG : NaN,
    xr: st.hasEstimate ? st.targetX : NaN,
    run: st.running,
  });
  while (view.samples.length && view.samples[0].t < st.simSeconds - WINDOW_SEC - 0.5) view.samples.shift();
}

const niceRange = (m, steps) => steps.find(v => v >= m) ?? steps[steps.length - 1];

function drawScope() {
  const canvas = $("scope");
  if (!canvas) return;
  const C = palette(), { ctx, w, h } = fitCanvas(canvas);
  ctx.clearRect(0, 0, w, h);
  const S = view.samples;
  if (!S.length) return;

  const tEnd = S[S.length - 1].t, tStart = tEnd - WINDOW_SEC;
  const margin = view.state ? view.state.marginMs : 0;

  // 倒立制御中は制御中サンプルだけでレンジを決める (吊り下げ状態の ±180° で潰れないように)
  const thSrc = S[S.length - 1].run ? S.filter(s => s.run) : S;
  const thMax = niceRange(Math.max(...thSrc.map(s => Math.abs(s.th))) * 1.1, [5, 10, 20, 45, 90, 180]);
  const e2eMax = niceRange(Math.max(margin * 1.4, ...S.map(s => s.run ? s.e2e * 1.2 : 0)),
    [10, 20, 50, 100, 150, 200, 300, 500, 1000]);

  const lanes = [
    { title: "振子角 θ [deg]", lo: -thMax, hi: thMax, zero: true, gapOut: true,
      traces: [{ k: "th", c: C.amber, w: 2 }, { k: "thE", c: C.sub, w: 1.3, dash: [4, 3] }] },
    { title: "台車位置 x [m]", lo: -0.5, hi: 0.5, zero: true, marks: [-0.45, 0.45],
      traces: [{ k: "x", c: C.accent, w: 2 }, { k: "xr", c: C.sub, w: 1.3, dash: [4, 3] }] },
    { title: "印加電圧 [V]", lo: -24, hi: 24, zero: true,
      traces: [{ k: "v", c: C.violet, w: 1.5 }] },
    { title: "E2E遅延 [ms]", lo: 0, hi: e2eMax, marks: margin ? [margin] : [], markColor: C.danger,
      traces: [{ k: "e2e", c: C.pink, w: 1.8, only: "run" }] },
  ];

  const padL = 40, padR = 8, gap = 8, laneH = (h - gap * (lanes.length - 1)) / lanes.length;
  ctx.font = "11px system-ui, sans-serif";
  lanes.forEach((ln, i) => {
    const top = i * (laneH + gap), bottom = top + laneH;
    const X = t => padL + (t - tStart) / WINDOW_SEC * (w - padL - padR);
    const Y = v => bottom - (v - ln.lo) / (ln.hi - ln.lo) * (laneH - 4) - 2;

    ctx.fillStyle = C.panel2; ctx.fillRect(padL, top, w - padL - padR, laneH);
    ctx.strokeStyle = C.grid; ctx.lineWidth = 1;
    for (let g = Math.ceil(tStart); g <= tEnd; g++) {
      ctx.beginPath(); ctx.moveTo(X(g), top); ctx.lineTo(X(g), bottom); ctx.stroke();
    }
    if (ln.zero) {
      ctx.strokeStyle = C.line;
      ctx.beginPath(); ctx.moveTo(padL, Y(0)); ctx.lineTo(w - padR, Y(0)); ctx.stroke();
    }
    (ln.marks || []).forEach(v => {
      ctx.strokeStyle = ln.markColor || C.warn; ctx.setLineDash([3, 3]);
      ctx.beginPath(); ctx.moveTo(padL, Y(v)); ctx.lineTo(w - padR, Y(v)); ctx.stroke();
      ctx.setLineDash([]);
    });
    ctx.fillStyle = C.sub; ctx.textAlign = "right";
    ctx.fillText(String(ln.hi), padL - 5, top + 10);
    ctx.fillText(String(ln.lo), padL - 5, bottom - 2);
    ctx.textAlign = "left"; ctx.fillStyle = C.text;
    ctx.fillText(ln.title, padL + 6, top + 13);

    ctx.save();
    ctx.beginPath(); ctx.rect(padL, top, w - padL - padR, laneH); ctx.clip();
    for (const tr of ln.traces) {
      ctx.strokeStyle = tr.c; ctx.lineWidth = tr.w; ctx.setLineDash(tr.dash || []);
      ctx.beginPath();
      let pen = false, prev = null;
      for (const s of S) {
        const v = s[tr.k];
        const ok = isFinite(v) && (!tr.only || s[tr.only])
          && (!ln.gapOut || (v >= ln.lo && v <= ln.hi))               // レンジ外は端に貼り付けず途切れさせる
          && (!ln.gapOut || prev === null || Math.abs(v - prev) < 180); // ±180° の巻き戻りで線を引かない
        if (!ok) { pen = false; prev = isFinite(v) ? v : null; continue; }
        const px = X(s.t), py = Y(Math.max(ln.lo, Math.min(ln.hi, v)));
        pen ? ctx.lineTo(px, py) : ctx.moveTo(px, py);
        pen = true;
        prev = v;
      }
      ctx.stroke();
    }
    ctx.setLineDash([]);
    ctx.restore();
  });
}

/* ---------- メインループ: 読んで描くだけ ---------- */

function frame() {
  const st = view.state;
  draw2D(st);
  render3D(st);
  drawScope();
  view.raf = requestAnimationFrame(frame);
}
