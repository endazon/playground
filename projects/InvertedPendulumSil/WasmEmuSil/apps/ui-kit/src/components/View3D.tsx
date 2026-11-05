import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import type { SimStore } from "../model/store";
import { onFrame } from "./canvas";

const PULLEY_X = 0.62;
const FLOOR_Y = -0.92;
const RAIL_VIS = 0.575;

export interface View3DProps {
  store: SimStore;
  className?: string;
  style?: React.CSSProperties;
}

/**
 * 3D ビュー (sim-view.js の init3D / render3D を移植)。three.js は npm から同梱 (CDN を使わない)。
 * ドラッグで視点を回す。WebGL が使えない環境では文言だけ出す。
 */
export function View3D({ store, className, style }: View3DProps) {
  const ref = useRef<HTMLCanvasElement>(null);
  const [failed, setFailed] = useState<string | null>(null);
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    let scene: Scene3D;
    try {
      scene = buildScene(canvas);
    } catch (e) {
      setFailed(String(e));
      return;
    }
    const stop = onFrame(() => scene.render(store.latest));
    return () => {
      stop();
      scene.dispose();
    };
  }, [store]);
  if (failed) return <div className="ipk-fallback">3D ビューを初期化できません (WebGL): {failed}</div>;
  return <canvas ref={ref} className={`ipk-canvas ${className ?? ""}`} style={{ touchAction: "none", ...style }} />;
}

interface Scene3D {
  render(st: { x: number; theta: number; pendulumLength: number; driveEnabled: boolean } | null): void;
  dispose(): void;
}

function buildScene(canvas: HTMLCanvasElement): Scene3D {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
  renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(38, 1, 0.05, 50);
  // r155 以降は物理ベースの光量なので、r128 版より強めにする
  scene.add(new THREE.HemisphereLight(0xffffff, 0x444455, 2.2));
  const sun = new THREE.DirectionalLight(0xffffff, 2.4);
  sun.position.set(1.5, 3, 2.5);
  scene.add(sun);

  const metal = new THREE.MeshPhongMaterial({ color: 0x8a94a3, shininess: 60 });
  const dark = new THREE.MeshPhongMaterial({ color: 0x3a414c, shininess: 30 });
  const box = (w: number, h: number, d: number, mat: THREE.Material, x: number, y: number, z: number) => {
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
  const pulleys = [-PULLEY_X, PULLEY_X].map((px) => {
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

  let az = 0.55;
  let ev = 0.18;
  let drag: { x: number; y: number; az: number; ev: number } | null = null;
  const listeners: [string, EventListener][] = [];
  const on = <K extends keyof HTMLElementEventMap>(name: K, h: (e: HTMLElementEventMap[K]) => void) => {
    canvas.addEventListener(name, h as EventListener);
    listeners.push([name, h as EventListener]);
  };
  on("pointerdown", (e) => {
    drag = { x: e.clientX, y: e.clientY, az, ev };
    canvas.setPointerCapture(e.pointerId);
  });
  on("pointermove", (e) => {
    if (!drag) return;
    az = drag.az + (e.clientX - drag.x) * 0.008;
    ev = Math.max(-0.3, Math.min(1.1, drag.ev + (e.clientY - drag.y) * 0.006));
  });
  for (const n of ["pointerup", "pointercancel", "pointerleave"] as const) on(n, () => (drag = null));

  let sizedW = 0;
  let sizedH = 0;
  return {
    render(st) {
      const w = canvas.clientWidth;
      const h = canvas.clientHeight;
      if (!w || !h) return;
      // 幅だけ見ていると高さだけ変わるレイアウトで aspect が古いまま歪む
      if (sizedW !== w || sizedH !== h) {
        sizedW = w;
        sizedH = h;
        renderer.setSize(w, h, false);
        camera.aspect = w / h;
        camera.updateProjectionMatrix();
      }
      const R = 2.7;
      camera.position.set(Math.sin(az) * Math.cos(ev) * R, Math.sin(ev) * R, Math.cos(az) * Math.cos(ev) * R);
      camera.lookAt(0, -0.05, 0);
      if (st && Number.isFinite(st.x) && Number.isFinite(st.theta)) {
        const len = st.pendulumLength;
        rod.scale.y = len;
        rod.position.y = len / 2;
        cart.position.x = st.x;
        cartMat.color.setHex(st.driveEnabled ? 0x4fd1c5 : 0xef5b5b);
        pend.position.set(st.x, 0, 0.05);
        pend.rotation.z = -st.theta;
        for (const p of pulleys) p.rotation.y = st.x / 0.015;
      }
      renderer.render(scene, camera);
    },
    dispose() {
      for (const [n, h] of listeners) canvas.removeEventListener(n, h);
      // ジオメトリ / マテリアルは GC 対象にならないので明示的に解放する
      scene.traverse((obj) => {
        const m = obj as THREE.Mesh;
        if (m.geometry) m.geometry.dispose();
        const mat = m.material as THREE.Material | THREE.Material[] | undefined;
        if (Array.isArray(mat)) mat.forEach((x) => x.dispose());
        else mat?.dispose();
      });
      renderer.dispose();
    },
  };
}
