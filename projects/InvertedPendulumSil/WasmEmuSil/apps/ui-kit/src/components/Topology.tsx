import { useMemo } from "react";
import type { SessionDescription, StateFrame } from "@ipemu/ipc-types";

export interface TopologyProps {
  desc: SessionDescription;
  /** 最新 state (配線に値を添える。省略可) */
  frame?: StateFrame | null;
  selected?: string | null;
  onSelect?: (instance: string) => void;
}

interface Box {
  instance: string;
  kind: string;
  x: number;
  y: number;
  w: number;
  h: number;
  inputs: string[];
  outputs: string[];
}

const BOX_W = 128;
const ROW = 14;
const COL_GAP = 60;
const ROW_GAP = 26;
const HEAD = 30;

/**
 * 部品を実行順の「層」に並べる: latency 0 の配線で前後関係を作り、残りは manifest 順。
 * 同じ層は縦に積む。
 */
export function layoutTopology(desc: SessionDescription): { boxes: Box[]; width: number; height: number } {
  const names = desc.parts.map((p) => p.instance);
  const idx = new Map(names.map((n, i) => [n, i]));
  const depth = new Array<number>(names.length).fill(0);
  // latency 0 の辺で最長路 (閉路なしが保証されている)
  const edges = desc.wires
    .filter((w) => w.latency === 0)
    .map((w) => [idx.get(w.from.split(".")[0]!), idx.get(w.to.split(".")[0]!)] as const)
    .filter((e): e is readonly [number, number] => e[0] !== undefined && e[1] !== undefined);
  for (let iter = 0; iter < names.length; iter++) {
    let changed = false;
    for (const [a, b] of edges) {
      if (depth[b]! < depth[a]! + 1) {
        depth[b] = depth[a]! + 1;
        changed = true;
      }
    }
    if (!changed) break;
  }
  // 配線のない部品 (snapshot など) は最後の層へ
  const maxDepth = Math.max(0, ...depth);
  desc.parts.forEach((p, i) => {
    const wired = desc.wires.some((w) => w.from.startsWith(p.instance + ".") || w.to.startsWith(p.instance + "."));
    if (!wired) depth[i] = maxDepth + 1;
  });
  const layers = new Map<number, number[]>();
  depth.forEach((d, i) => layers.set(d, [...(layers.get(d) ?? []), i]));
  const boxes: Box[] = [];
  let width = 0;
  let height = 0;
  for (const [d, members] of [...layers.entries()].sort((a, b) => a[0] - b[0])) {
    let y = 10;
    for (const i of members) {
      const p = desc.parts[i]!;
      const rows = Math.max(p.inputs.length, p.outputs.length, 1);
      const h = HEAD + rows * ROW + 6;
      boxes.push({
        instance: p.instance,
        kind: p.kind,
        x: 10 + d * (BOX_W + COL_GAP),
        y,
        w: BOX_W,
        h,
        inputs: p.inputs.map((q) => q.name),
        outputs: p.outputs.map((q) => q.name),
      });
      y += h + ROW_GAP;
      height = Math.max(height, y);
    }
    width = Math.max(width, 10 + d * (BOX_W + COL_GAP) + BOX_W + 10);
  }
  return { boxes, width, height: height + 4 };
}

/** トポロジ図: 部品 (ポート付き) と配線。latency 0 の配線は強調する */
export function Topology({ desc, frame, selected, onSelect }: TopologyProps) {
  const { boxes, width, height } = useMemo(() => layoutTopology(desc), [desc]);
  const byName = new Map(boxes.map((b) => [b.instance, b]));
  const portY = (b: Box, side: "in" | "out", name: string) => {
    const list = side === "in" ? b.inputs : b.outputs;
    const k = Math.max(0, list.indexOf(name));
    return b.y + HEAD + k * ROW + ROW / 2;
  };
  const snapIdx = new Map(desc.snapshotPorts.map((p, i) => [p, i]));
  return (
    <svg className="ipk-topo" viewBox={`0 0 ${width} ${height}`} role="img" aria-label="トポロジ">
      <defs>
        <marker id="ipk-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto">
          <path d="M0,0 L8,4 L0,8 z" fill="var(--sub)" />
        </marker>
      </defs>
      {desc.wires.map((w, i) => {
        const [fi, fp] = w.from.split(".") as [string, string];
        const [ti, tp] = w.to.split(".") as [string, string];
        const a = byName.get(fi);
        const b = byName.get(ti);
        if (!a || !b) return null;
        const x1 = a.x + a.w;
        const y1 = portY(a, "out", fp);
        const x2 = b.x;
        const y2 = portY(b, "in", tp);
        const back = x2 < x1 + 10; // 後ろの層へ戻る配線 (latency 1 のフィードバック) は下を回す
        const d = back
          ? `M${x1},${y1} C${x1 + 30},${y1} ${x1 + 30},${Math.max(a.y + a.h, b.y + b.h) + 14} ${(x1 + x2) / 2},${Math.max(a.y + a.h, b.y + b.h) + 14} S${x2 - 30},${y2} ${x2},${y2}`
          : `M${x1},${y1} C${(x1 + x2) / 2},${y1} ${(x1 + x2) / 2},${y2} ${x2},${y2}`;
        const vi = snapIdx.get(w.from);
        const val = vi !== undefined && frame ? frame.values[vi] : undefined;
        return (
          <g key={i}>
            <path className={`wire ${w.latency === 0 ? "l0" : ""}`} d={d} markerEnd="url(#ipk-arrow)">
              <title>
                {w.from} → {w.to} (latency {w.latency}
                {w.gain !== 1 ? `, gain ${w.gain}` : ""}
                {w.offset ? `, offset ${w.offset}` : ""})
              </title>
            </path>
            {val !== undefined && Number.isFinite(val) && !back && (
              <text className="val" x={(x1 + x2) / 2} y={(y1 + y2) / 2 - 3} textAnchor="middle">
                {fmt(val)}
              </text>
            )}
          </g>
        );
      })}
      {boxes.map((b) => (
        <g key={b.instance} onClick={() => onSelect?.(b.instance)} style={{ cursor: onSelect ? "pointer" : "default" }}>
          <rect
            className={`box ${b.kind.startsWith("builtin") ? "builtin" : ""} ${selected === b.instance ? "selected" : ""}`}
            x={b.x}
            y={b.y}
            width={b.w}
            height={b.h}
            rx={8}
          />
          <text x={b.x + 8} y={b.y + 14} fontWeight={600}>
            {b.instance}
          </text>
          <text className="port" x={b.x + 8} y={b.y + 25}>
            {b.kind}
          </text>
          {b.inputs.map((n, k) => (
            <text key={"i" + n} className="port" x={b.x + 6} y={b.y + HEAD + k * ROW + ROW / 2 + 3.5}>
              ▸{n}
            </text>
          ))}
          {b.outputs.map((n, k) => (
            <text key={"o" + n} className="port" x={b.x + b.w - 6} y={b.y + HEAD + k * ROW + ROW / 2 + 3.5} textAnchor="end">
              {n}▸
            </text>
          ))}
        </g>
      ))}
    </svg>
  );
}

function fmt(v: number): string {
  const a = Math.abs(v);
  if (a >= 1000) return v.toFixed(0);
  if (a >= 10) return v.toFixed(2);
  return v.toFixed(3);
}
