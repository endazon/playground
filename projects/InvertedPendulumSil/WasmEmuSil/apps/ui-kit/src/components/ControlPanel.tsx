import { useState } from "react";
import type { PacingConfig, PacingMode, RunStatus, SessionDescription } from "@ipemu/ipc-types";

export interface ControlPanelProps {
  desc: SessionDescription;
  run: RunStatus;
  busy?: boolean;
  onStart(): void;
  onStop(): void;
  onReset(): void;
  onStep(n: number): void;
  onPacing(p: PacingConfig): void;
  onParam(instance: string, key: string, value: unknown): void;
  /** source: wasm:<path> | native:<path> | default */
  onFirmware(slot: string, source: string): void;
  onWatch(slot: string, source: string | null): void;
}

/** 操作パネル: 開始 / 停止 / リセット / ステップ / ペーシング / パラメータ / FW 差替。状態は持たず、操作を上へ渡すだけ */
export function ControlPanel(p: ControlPanelProps) {
  const running = p.run.state === "running";
  const fault = p.run.state === "fault";
  const [stepN, setStepN] = useState(100);
  const [speed, setSpeed] = useState(p.run.pacing.speed);
  const slots = p.desc.parts.flatMap((part) => (part.firmware ? [{ part, fw: part.firmware }] : []));
  return (
    <div className="ipk-panel">
      <div className="ipk-row">
        <span className="ipk-mode" data-m={p.run.state}>
          {p.run.state === "running" ? "RUNNING" : p.run.state === "fault" ? "FAULT" : "IDLE"}
        </span>
        <span className="ipk-badge">t = {(p.run.tNs * 1e-9).toFixed(2)} s</span>
        <span className="ipk-badge">epoch {p.run.epoch}</span>
        {fault && (
          <span className="ipk-badge danger" title={p.run.fault ?? ""}>
            {p.run.fault}
          </span>
        )}
      </div>
      <div className="ipk-row" style={{ marginTop: 10 }}>
        <button className="ipk-btn" disabled={p.busy || running || fault} onClick={p.onStart}>
          開始
        </button>
        <button className="ipk-btn ghost" disabled={p.busy || !running} onClick={p.onStop}>
          停止
        </button>
        <button className="ipk-btn ghost" disabled={p.busy} onClick={p.onReset} title="全部品を reset して t=0 へ (Epoch が進む)">
          リセット
        </button>
        <span className="ipk-field" style={{ marginLeft: "auto" }}>
          <input
            type="number"
            min={1}
            value={stepN}
            onChange={(e) => setStepN(Math.max(1, Number(e.target.value) || 1))}
            style={{ width: "6em" }}
            aria-label="ステップ数"
          />
          <button
            className="ipk-btn ghost"
            disabled={p.busy || fault || (running && p.run.pacing.mode !== "step")}
            onClick={() => p.onStep(stepN)}
            title="停止中 (または step モード) に baseStep 単位で進める"
          >
            ステップ
          </button>
        </span>
      </div>

      <div className="ipk-h">ペーシング</div>
      <div className="ipk-row">
        {(["realtime", "fast", "step"] as PacingMode[]).map((m) => (
          <button
            key={m}
            className={`ipk-btn ${p.run.pacing.mode === m ? "" : "ghost"}`}
            disabled={p.busy}
            onClick={() => p.onPacing({ mode: m, speed })}
          >
            {m === "realtime" ? "実時間" : m === "fast" ? "最速" : "手動ステップ"}
          </button>
        ))}
        <label className="ipk-field">
          <span>倍速</span>
          <select
            value={speed}
            onChange={(e) => {
              const s = Number(e.target.value);
              setSpeed(s);
              p.onPacing({ mode: p.run.pacing.mode, speed: s });
            }}
          >
            {[0.1, 0.25, 0.5, 1, 2, 4, 10].map((s) => (
              <option key={s} value={s}>
                ×{s}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="ipk-h">ファームウェア</div>
      {slots.map(({ part, fw }) => (
        <FirmwareRow key={fw.slot} instance={part.instance} fw={fw} busy={p.busy} onLoad={p.onFirmware} onWatch={p.onWatch} />
      ))}

      <div className="ipk-h">パラメータ</div>
      <ParamEditor desc={p.desc} busy={p.busy} onParam={p.onParam} />
    </div>
  );
}

function FirmwareRow(props: {
  instance: string;
  fw: NonNullable<SessionDescription["parts"][number]["firmware"]>;
  busy?: boolean;
  onLoad(slot: string, source: string): void;
  onWatch(slot: string, source: string | null): void;
}) {
  const { fw } = props;
  const [src, setSrc] = useState(fw.source === "default" ? "" : fw.source);
  const isNative = src.startsWith("native:");
  const label = isNative ? "再起動して差替" : "ホット差替";
  return (
    <div style={{ fontSize: 12.5, marginBottom: 6 }}>
      <div className="ipk-row">
        <b>{fw.slot}</b>
        <span className="ipk-badge">{props.instance}</span>
        <span className="ipk-badge">{fw.flavor ?? "?"}</span>
        <span className="sub" style={{ color: "var(--sub)" }}>
          {fw.source}
        </span>
        {fw.watching && (
          <span className="ipk-badge warn" title={fw.watching}>
            監視中
          </span>
        )}
      </div>
      <div className="ipk-row" style={{ marginTop: 4 }}>
        <input
          placeholder="wasm:<path> | native:<path> | default"
          value={src}
          onChange={(e) => setSrc(e.target.value)}
          style={{ flex: 1, minWidth: 160, font: "inherit", padding: "4px 6px", border: "1px solid var(--line)", borderRadius: 6, background: "var(--panel2)", color: "var(--text)" }}
          aria-label={`${fw.slot} のファームウェア`}
        />
        <button className="ipk-btn ghost" disabled={props.busy || !src} onClick={() => props.onLoad(fw.slot, src)} title="wasm はホット (状態保持)、native はエンジン再起動 (D23)">
          {label}
        </button>
        {fw.watching ? (
          <button className="ipk-btn ghost" disabled={props.busy} onClick={() => props.onWatch(fw.slot, null)}>
            監視停止
          </button>
        ) : (
          <button className="ipk-btn ghost" disabled={props.busy || !src || src === "default"} onClick={() => props.onWatch(fw.slot, src)} title="ビルド成果物の更新を検知して自動で差し替える">
            監視
          </button>
        )}
      </div>
    </div>
  );
}

function ParamEditor(props: { desc: SessionDescription; busy?: boolean; onParam(instance: string, key: string, value: unknown): void }) {
  const editable = props.desc.parts.filter((p) => p.kind === "part");
  const [inst, setInst] = useState(editable[0]?.instance ?? "");
  const part = editable.find((p) => p.instance === inst);
  const params = (part?.params ?? {}) as Record<string, unknown>;
  const keys = Object.keys(params).filter((k) => typeof params[k] === "number" || typeof params[k] === "boolean" || typeof params[k] === "string");
  const [key, setKey] = useState(keys[0] ?? "");
  const curKey = keys.includes(key) ? key : (keys[0] ?? "");
  const [text, setText] = useState("");
  const current = params[curKey];
  return (
    <div className="ipk-row" style={{ fontSize: 12.5 }}>
      <select
        value={inst}
        onChange={(e) => {
          setInst(e.target.value);
          setText("");
        }}
        aria-label="部品"
      >
        {editable.map((p) => (
          <option key={p.instance} value={p.instance}>
            {p.instance}
          </option>
        ))}
      </select>
      <select
        value={curKey}
        onChange={(e) => {
          setKey(e.target.value);
          setText("");
        }}
        aria-label="キー"
      >
        {keys.map((k) => (
          <option key={k} value={k}>
            {k}
          </option>
        ))}
      </select>
      <input
        value={text}
        placeholder={current === undefined ? "" : JSON.stringify(current)}
        onChange={(e) => setText(e.target.value)}
        style={{ width: "8em", font: "inherit", padding: "4px 6px", border: "1px solid var(--line)", borderRadius: 6, background: "var(--panel2)", color: "var(--text)" }}
        aria-label="値"
      />
      <button
        className="ipk-btn ghost"
        disabled={props.busy || !inst || !curKey || text === ""}
        onClick={() => {
          let v: unknown = text;
          try {
            v = JSON.parse(text);
          } catch {
            /* 文字列のまま */
          }
          props.onParam(inst, curKey, v);
          setText("");
        }}
        title="部品は init し直される (内部状態は初期化)"
      >
        適用
      </button>
    </div>
  );
}
