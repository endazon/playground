// トポロジ図: 部品 (箱) と配線 (曲線) を SVG で描く。配線の from → to から層 (列) を決める。
// 将来は apps/ui-kit (M2) の共有部品に移す前提で、依存を React + SVG だけにしてある。
import { useMemo } from "react";
import type { ComponentSpec, PartDesc, Wire } from "../api/types";

export interface TopologyProps {
  components: ComponentSpec[];
  wires: Wire[];
  /** 検証結果に含まれる部品の記述 (あればポートを全部出す。無ければ配線に現れるポートだけ) */
  descs?: { instance: string; desc: PartDesc }[];
  /** 強調する instance */
  highlight?: string | null;
  onSelect?: (instance: string) => void;
}

interface Node {
  instance: string;
  label: string;
  sub: string;
  inputs: string[];
  outputs: string[];
  caps: boolean;
  col: number;
  row: number;
}

const BOX_W = 150;
const PORT_H = 16;
const HEAD_H = 36;
const COL_GAP = 90;
const ROW_GAP = 24;
const PAD = 16;

/** 配線から各部品の列 (最長経路) を求める。閉路は latency=1 の配線を無視して切る。 */
export function layoutColumns(components: ComponentSpec[], wires: Wire[]): Map<string, number> {
  const names = components.map((c) => c.instance);
  const idx = new Map(names.map((n, i) => [n, i]));
  const edges: [number, number][] = [];
  for (const w of wires) {
    const a = idx.get(w.from.split(".")[0]);
    const b = idx.get(w.to.split(".")[0]);
    if (a !== undefined && b !== undefined && a !== b) edges.push([a, b]);
  }
  // 閉路を壊す: DFS で back edge を取り除く
  const adj = names.map(() => [] as number[]);
  for (const [a, b] of edges) adj[a].push(b);
  const state = new Array<number>(names.length).fill(0);
  const keep = new Set<string>();
  const dfs = (u: number) => {
    state[u] = 1;
    for (const v of adj[u]) {
      if (state[v] === 0) {
        keep.add(`${u}>${v}`);
        dfs(v);
      } else if (state[v] === 2) {
        keep.add(`${u}>${v}`);
      }
    }
    state[u] = 2;
  };
  names.forEach((_, i) => state[i] === 0 && dfs(i));
  // 最長経路で列を決める
  const col = new Array<number>(names.length).fill(0);
  const dag = names.map(() => [] as number[]);
  for (const [a, b] of edges) if (keep.has(`${a}>${b}`)) dag[a].push(b);
  const indeg = new Array<number>(names.length).fill(0);
  dag.forEach((vs) => vs.forEach((v) => indeg[v]++));
  const queue = names.map((_, i) => i).filter((i) => indeg[i] === 0);
  while (queue.length) {
    const u = queue.shift()!;
    for (const v of dag[u]) {
      col[v] = Math.max(col[v], col[u] + 1);
      if (--indeg[v] === 0) queue.push(v);
    }
  }
  return new Map(names.map((n, i) => [n, col[i]]));
}

function buildNodes(components: ComponentSpec[], wires: Wire[], descs?: TopologyProps["descs"]): Node[] {
  const cols = layoutColumns(components, wires);
  const byInst = new Map((descs ?? []).map((d) => [d.instance, d.desc]));
  const inPorts = new Map<string, Set<string>>();
  const outPorts = new Map<string, Set<string>>();
  for (const w of wires) {
    const [fi, fp] = w.from.split(".");
    const [ti, tp] = w.to.split(".");
    if (!outPorts.has(fi)) outPorts.set(fi, new Set());
    if (!inPorts.has(ti)) inPorts.set(ti, new Set());
    outPorts.get(fi)!.add(fp);
    inPorts.get(ti)!.add(tp);
  }
  const rowsPerCol = new Map<number, number>();
  return components.map((c) => {
    const d = byInst.get(c.instance);
    const col = cols.get(c.instance) ?? 0;
    const row = rowsPerCol.get(col) ?? 0;
    rowsPerCol.set(col, row + 1);
    const inputs = d ? d.inputs.map((p) => p.name) : [...(inPorts.get(c.instance) ?? [])];
    const outputs = d ? d.outputs.map((p) => p.name) : [...(outPorts.get(c.instance) ?? [])];
    return {
      instance: c.instance,
      label: c.instance,
      sub: c.artifact ?? c.kind ?? "",
      inputs,
      outputs,
      caps: !!c.capabilities && Object.keys(c.capabilities).length > 0,
      col,
      row,
    };
  });
}

export function TopologyView({ components, wires, descs, highlight, onSelect }: TopologyProps) {
  const { nodes, pos, width, height } = useMemo(() => {
    const nodes = buildNodes(components, wires, descs);
    const heights = nodes.map((n) => HEAD_H + Math.max(n.inputs.length, n.outputs.length, 1) * PORT_H + 8);
    const colY = new Map<number, number>();
    const pos = new Map<string, { x: number; y: number; h: number }>();
    let maxCol = 0;
    nodes.forEach((n, i) => {
      const y = colY.get(n.col) ?? PAD;
      pos.set(n.instance, { x: PAD + n.col * (BOX_W + COL_GAP), y, h: heights[i] });
      colY.set(n.col, y + heights[i] + ROW_GAP);
      maxCol = Math.max(maxCol, n.col);
    });
    const height = Math.max(...[...colY.values()], 100) + PAD;
    const width = PAD * 2 + (maxCol + 1) * BOX_W + maxCol * COL_GAP;
    return { nodes, pos, width, height };
  }, [components, wires, descs]);

  const portY = (n: Node, which: "in" | "out", name: string) => {
    const list = which === "in" ? n.inputs : n.outputs;
    const i = Math.max(0, list.indexOf(name));
    return HEAD_H + i * PORT_H + PORT_H / 2;
  };
  const byName = new Map(nodes.map((n) => [n.instance, n]));

  return (
    <svg className="topology" viewBox={`0 0 ${width} ${height}`} width={width} height={height} role="img" aria-label="トポロジ図">
      <defs>
        <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" fill="currentColor" />
        </marker>
      </defs>
      {wires.map((w, i) => {
        const [fi, fp] = w.from.split(".");
        const [ti, tp] = w.to.split(".");
        const a = byName.get(fi);
        const b = byName.get(ti);
        const pa = pos.get(fi);
        const pb = pos.get(ti);
        if (!a || !b || !pa || !pb) return null;
        const x1 = pa.x + BOX_W;
        const y1 = pa.y + portY(a, "out", fp);
        const x2 = pb.x;
        const y2 = pb.y + portY(b, "in", tp);
        const dx = Math.max(40, Math.abs(x2 - x1) / 2);
        const d = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
        const hot = highlight && (fi === highlight || ti === highlight);
        return (
          <g key={i} className={`wire ${w.latency === 0 ? "wire-sync" : ""} ${hot ? "wire-hot" : ""}`}>
            <path d={d} fill="none" markerEnd="url(#arrow)">
              <title>
                {w.from} → {w.to} (latency {w.latency ?? 1}
                {w.gain !== undefined && w.gain !== 1 ? `, gain ${w.gain}` : ""})
              </title>
            </path>
          </g>
        );
      })}
      {nodes.map((n) => {
        const p = pos.get(n.instance)!;
        return (
          <g key={n.instance} className={`node ${highlight === n.instance ? "node-hot" : ""}`} transform={`translate(${p.x},${p.y})`} onClick={() => onSelect?.(n.instance)} style={{ cursor: onSelect ? "pointer" : "default" }}>
            <rect width={BOX_W} height={p.h} rx={6} />
            <text x={8} y={16} className="node-title">
              {n.label}
              {n.caps ? " 🔐" : ""}
            </text>
            <text x={8} y={29} className="node-sub">
              {n.sub}
            </text>
            {n.inputs.map((name, i) => (
              <g key={`i-${name}`}>
                <circle cx={0} cy={HEAD_H + i * PORT_H + PORT_H / 2} r={3} className="port" />
                <text x={6} y={HEAD_H + i * PORT_H + PORT_H / 2 + 4} className="port-label">
                  {name}
                </text>
              </g>
            ))}
            {n.outputs.map((name, i) => (
              <g key={`o-${name}`}>
                <circle cx={BOX_W} cy={HEAD_H + i * PORT_H + PORT_H / 2} r={3} className="port" />
                <text x={BOX_W - 6} y={HEAD_H + i * PORT_H + PORT_H / 2 + 4} className="port-label" textAnchor="end">
                  {name}
                </text>
              </g>
            ))}
          </g>
        );
      })}
    </svg>
  );
}
