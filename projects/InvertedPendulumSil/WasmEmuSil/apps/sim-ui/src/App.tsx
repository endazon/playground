import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import type { EventNotice, LaunchParams, PacingConfig, RunStatus, SessionDescription } from "@ipemu/ipc-types";
import { codes } from "@ipemu/ipc-types";
import {
  CartPoleMapper,
  ControlPanel,
  EngineClient,
  EventLog,
  RpcCallError,
  Scope,
  SideView2D,
  SimStore,
  Topology,
  View3D,
  endpointUrl,
  wrapAngle,
  type ScopeLane,
} from "@ipemu/ui-kit";

interface Endpoint {
  port: number;
  token: string;
}

/** スコープで購読するポート (順番がチャネル index) */
const SCOPE_PORTS = ["plant.theta", "plant.x", "driver.voltage", "mcu.pwm0"];
const RAD2DEG = 180 / Math.PI;
const SCOPE_LANES: ScopeLane[] = [
  { title: "振子角 θ [deg]", lo: -180, hi: 180, autoRange: [5, 10, 20, 45, 90, 180], zero: true, gapOut: true, traces: [{ channel: 0, color: "amber", width: 2, transform: (v) => wrapAngle(v) * RAD2DEG }] },
  { title: "台車位置 x [m]", lo: -0.5, hi: 0.5, zero: true, marks: [-0.45, 0.45], traces: [{ channel: 1, color: "accent", width: 2 }] },
  { title: "印加電圧 [V]", lo: -24, hi: 24, zero: true, traces: [{ channel: 2, color: "violet", width: 1.5 }] },
  { title: "PWM duty", lo: -1, hi: 1, zero: true, traces: [{ channel: 3, color: "pink", width: 1.5 }] },
];

/** 接続先の決定: Electron のブリッジ → URL (?port=&token=) → localStorage */
async function resolveEndpoint(): Promise<Endpoint | null> {
  if (window.ipemuEngine) {
    const ep = await window.ipemuEngine.endpoint();
    if (ep) return ep;
  }
  const q = new URLSearchParams(location.search);
  const port = Number(q.get("port"));
  const token = q.get("token");
  if (port && token) return { port, token };
  try {
    const saved = localStorage.getItem("ipemu.endpoint");
    if (saved) return JSON.parse(saved) as Endpoint;
  } catch {
    /* 無視 */
  }
  return null;
}

export function App() {
  const [endpoint, setEndpoint] = useState<Endpoint | null | undefined>(undefined);
  useEffect(() => {
    void resolveEndpoint().then(setEndpoint);
    return window.ipemuEngine?.onEndpointChanged((ep) => setEndpoint({ ...ep }));
  }, []);
  if (endpoint === undefined) return <div className="app">接続先を確認中…</div>;
  if (!endpoint) return <ConnectForm onConnect={setEndpoint} />;
  return <Session key={`${endpoint.port}:${endpoint.token}`} endpoint={endpoint} onDisconnect={() => setEndpoint(null)} />;
}

function ConnectForm({ onConnect }: { onConnect: (ep: Endpoint) => void }) {
  const [port, setPort] = useState("");
  const [token, setToken] = useState("");
  return (
    <div className="app">
      <div className="ipk-panel connect">
        <h2 style={{ marginTop: 0 }}>エンジンに接続</h2>
        <p className="sub" style={{ color: "var(--sub)", fontSize: 12.5 }}>
          シェル (Electron) の中では自動で接続します。ブラウザで開発するときは <code>ipemu-engine serve</code> が stdout に出す port と token を入力してください
          (URL に <code>?port=…&amp;token=…</code> を付けても可)。
        </p>
        <label>
          port
          <input value={port} onChange={(e) => setPort(e.target.value)} inputMode="numeric" />
        </label>
        <label>
          token
          <input value={token} onChange={(e) => setToken(e.target.value)} />
        </label>
        <button
          className="ipk-btn"
          disabled={!Number(port) || !token}
          onClick={() => {
            const ep = { port: Number(port), token };
            try {
              localStorage.setItem("ipemu.endpoint", JSON.stringify(ep));
            } catch {
              /* 無視 */
            }
            onConnect(ep);
          }}
        >
          接続
        </button>
      </div>
    </div>
  );
}

function Session({ endpoint, onDisconnect }: { endpoint: Endpoint; onDisconnect: () => void }) {
  const client = useMemo(() => new EngineClient(endpointUrl(endpoint.port, endpoint.token)), [endpoint]);
  const store = useMemo(() => new SimStore(SCOPE_PORTS.length), []);
  const [conn, setConn] = useState(client.connection);
  const [error, setError] = useState<string | null>(null);
  const [desc, setDesc] = useState<SessionDescription | null>(null);
  const [run, setRun] = useState<RunStatus | null>(null);
  const [events, setEvents] = useState<EventNotice[]>([]);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [view, setView] = useState<"2d" | "3d" | "both">("both");
  const [selected, setSelected] = useState<string | null>(null);
  const mapper = useRef<CartPoleMapper | null>(null);
  const version = useSyncExternalStore(store.subscribe, store.getVersion);
  const latestFrame = store.latestFrame;
  void version;

  const pushEvent = useCallback((e: EventNotice) => setEvents((xs) => [...xs.slice(-499), e]), []);

  const describe = useCallback(async () => {
    const d = await client.call("session.describe", {});
    setDesc(d.session ?? null);
    setRun(d.session?.run ?? null);
    if (d.session) {
      mapper.current = new CartPoleMapper(d.session);
      const want = SCOPE_PORTS.filter((p) => d.session!.parts.some((part) => p.startsWith(part.instance + ".")));
      try {
        await client.call("scope.subscribe", { ports: want, rateHz: 200, windowS: 0.05, subscriptionId: 1 });
      } catch (e) {
        console.warn("scope.subscribe", e);
      }
    }
    return d.session ?? null;
  }, [client]);

  // 接続 → describe。state / scope / 通知の購読
  useEffect(() => {
    const offs = [
      client.onConnection(setConn),
      client.onState((f) => {
        const m = mapper.current;
        if (!m) return;
        store.setState(m.map(f), f);
        if (store.scope.epoch !== f.epoch && store.scope.samples.length === 0) store.scope.epoch = f.epoch;
      }),
      client.onScope((f) => {
        if (f.subscriptionId === 1) store.scope.pushFrame(f);
      }),
      client.onNotification((n) => {
        if (n.method === "event") {
          pushEvent(n.params);
          if (n.params.kind === "run" && n.params.data) setRun(n.params.data as RunStatus);
          if (n.params.kind === "session" || n.params.kind === "firmware" || n.params.kind === "param") void describe().catch(() => {});
        } else if (n.method === "bootstrap.progress") {
          setNotice(`${n.params.stage}: ${n.params.message}`);
        } else if (n.method === "consent.required") {
          setNotice(`同意が必要: ${n.params.instance} ${n.params.capability} (シェルのダイアログで回答)`);
        }
      }),
    ];
    client
      .connect()
      .then(describe)
      .catch((e: Error) => setError(e.message));
    return () => {
      offs.forEach((f) => f());
      client.close();
    };
  }, [client, store, describe, pushEvent]);

  const act = useCallback(
    async (f: () => Promise<unknown>) => {
      setBusy(true);
      setNotice(null);
      try {
        await f();
      } catch (e) {
        if (e instanceof RpcCallError) setNotice(`${e.message} (code ${e.code})`);
        else setNotice(String(e));
      } finally {
        setBusy(false);
      }
    },
    [],
  );

  const launch = (manifestPath: string) =>
    act(async () => {
      const p: LaunchParams = {
        source: { kind: "local", manifestPath, allowUnhashed: true },
        firmware: {},
        params: [],
        pacing: { mode: "realtime", speed: 1 },
        autoStart: false,
      };
      await client.call("session.launch", p);
      await describe();
    });

  if (error) {
    return (
      <div className="app">
        <div className="ipk-panel connect">
          <h2 style={{ marginTop: 0 }}>接続できません</h2>
          <p className="notice">{error}</p>
          <p className="sub" style={{ color: "var(--sub)", fontSize: 12.5 }}>
            ws://127.0.0.1:{endpoint.port} — エンジンが起動しているか、トークンが合っているか確認してください。
          </p>
          <button className="ipk-btn ghost" onClick={onDisconnect}>
            接続先を変える
          </button>
        </div>
      </div>
    );
  }

  const fw = desc?.parts.find((p) => p.firmware)?.firmware;
  const stats = latestFrame?.stats;
  return (
    <div className="app">
      <header className="app-head">
        <h1>{desc ? desc.manifest.name : "ipemu シミュレータ"}</h1>
        {desc && (
          <span className="sub">
            {desc.manifest.id} rev {desc.manifest.revision} · baseStep {desc.manifest.baseStepNs / 1e6} ms · {desc.parts.length} 部品 / {desc.wires.length} 配線
          </span>
        )}
        <span className={`ipk-badge ${conn === "open" ? "" : "danger"}`} style={{ marginLeft: "auto" }}>
          {conn === "open" ? `engine :${endpoint.port}` : conn}
        </span>
        {stats && (
          <>
            <span className="ipk-badge" title="仮想時間 / 壁時計">
              ×{stats.realtimeRatio.toFixed(2)}
            </span>
            <span className={`ipk-badge ${stats.slips ? "warn" : ""}`} title="maxSlip 超過で捨てた時間">
              slip {stats.slipMsTotal.toFixed(0)} ms
            </span>
            <span className="ipk-badge" title="1 ステップの壁時計">
              {stats.stepWallUs.toFixed(1)} µs/step
            </span>
          </>
        )}
        {fw && (
          <span className="ipk-badge" title={fw.source}>
            FW {fw.slot}: {fw.flavor ?? "?"}
          </span>
        )}
      </header>
      {notice && <p className="notice">{notice}</p>}

      {!desc && (
        <LaunchForm
          busy={busy}
          onLaunch={launch}
          onCatalog={async () => {
            const c = await client.call("catalog.local", {});
            return c.entries.map((e) => ({ label: `${e.id} rev ${e.revision} — ${e.name}`, path: e.source.kind === "local" ? e.source.manifestPath : "" }));
          }}
        />
      )}

      {desc && run && (
        <div className="app-grid">
          <div className="side">
            <div className="views" style={view === "both" ? {} : { gridTemplateColumns: "1fr" }}>
              {(view === "2d" || view === "both") && (
                <section className="ipk-view">
                  <div className="ipk-vhead">
                    <span>側面図</span>
                    <ViewTabs view={view} setView={setView} />
                  </div>
                  <div className="view-body">
                    <SideView2D store={store} />
                  </div>
                </section>
              )}
              {(view === "3d" || view === "both") && (
                <section className="ipk-view">
                  <div className="ipk-vhead">
                    <span>3D (ドラッグで視点)</span>
                    {view === "3d" && <ViewTabs view={view} setView={setView} />}
                  </div>
                  <div className="view-body">
                    <View3D store={store} />
                  </div>
                </section>
              )}
            </div>
            <section className="ipk-view">
              <div className="ipk-vhead">
                <span>スコープ (scope.subscribe 200 Hz, 直近 10 s)</span>
                <span className="ipk-legend">
                  <i /> θ <i style={{ borderTopColor: "var(--accent)" }} /> x <i style={{ borderTopColor: "var(--violet)" }} /> V <i style={{ borderTopColor: "var(--pink)" }} /> duty
                </span>
              </div>
              <div className="scope-body">
                <Scope buffer={store.scope} lanes={SCOPE_LANES} />
              </div>
            </section>
            <section className="ipk-view">
              <div className="ipk-vhead">
                <span>トポロジ (部品と配線。実線 = latency 0、値は snapshot から)</span>
                {selected && <span>{selected}</span>}
              </div>
              <Topology desc={desc} frame={latestFrame} selected={selected} onSelect={setSelected} />
              {selected && <PartDetail desc={desc} instance={selected} />}
            </section>
          </div>
          <aside className="side">
            <ControlPanel
              desc={desc}
              run={run}
              busy={busy}
              onStart={() => act(async () => setRun(await client.call("run.start", {})))}
              onStop={() => act(async () => setRun(await client.call("run.stop", {})))}
              onReset={() => act(async () => setRun(await client.call("run.reset", {})))}
              onStep={(n) => act(async () => setRun(await client.call("run.step", { n })))}
              onPacing={(p: PacingConfig) => act(async () => setRun(await client.call("run.setPacing", p)))}
              onParam={(instance, key, value) =>
                act(async () => {
                  const r = await client.call("param.set", { instance, key, value: value as never });
                  if (r.restartRequired) setNotice(`${instance}.${key} は再起動 (再 launch) で反映されます`);
                  await describe();
                })
              }
              onFirmware={(slot, source) =>
                act(async () => {
                  const r = await client.call("firmware.load", { slot, source });
                  setNotice(r.message);
                  if (r.restartRequired) {
                    if (window.ipemuEngine) await window.ipemuEngine.restartEngine();
                    else setNotice(`${r.message} — ブラウザ開発では ipemu-engine serve を --firmware ${slot}=${source} で起動し直してください`);
                  }
                  await describe();
                })
              }
              onWatch={(slot, source) =>
                act(async () => {
                  await client.call("firmware.watch", { slot, source: source ?? undefined });
                  await describe();
                })
              }
            />
            <section className="ipk-panel">
              <div className="ipk-h" style={{ marginTop: 0 }}>
                イベント
              </div>
              <EventLog events={events} />
            </section>
            {desc.warnings.length > 0 && (
              <section className="ipk-panel">
                <div className="ipk-h" style={{ marginTop: 0 }}>
                  manifest の警告
                </div>
                <div className="ipk-log">
                  {desc.warnings.map((w, i) => (
                    <div key={i} className="warn">
                      [{w.rule}] {w.message}
                    </div>
                  ))}
                </div>
              </section>
            )}
          </aside>
        </div>
      )}
    </div>
  );
}

function ViewTabs({ view, setView }: { view: "2d" | "3d" | "both"; setView: (v: "2d" | "3d" | "both") => void }) {
  return (
    <span className="tabs">
      {(["2d", "3d", "both"] as const).map((v) => (
        <button key={v} className={view === v ? "on" : ""} onClick={() => setView(v)}>
          {v === "both" ? "2D+3D" : v.toUpperCase()}
        </button>
      ))}
    </span>
  );
}

function PartDetail({ desc, instance }: { desc: SessionDescription; instance: string }) {
  const p = desc.parts.find((x) => x.instance === instance);
  if (!p) return null;
  return (
    <div className="kv" style={{ marginTop: 8 }}>
      <b>kind</b>
      <span>
        {p.kind} {p.artifact ?? ""} ({p.name} {p.version})
      </span>
      <b>doc</b>
      <span>{p.doc}</span>
      <b>rateDivider</b>
      <span>{p.rateDivider}</span>
      <b>params</b>
      <code style={{ fontSize: 11.5, whiteSpace: "pre-wrap" }}>{JSON.stringify(p.params)}</code>
      {p.firmware && (
        <>
          <b>firmware</b>
          <span>
            slot {p.firmware.slot} · {p.firmware.flavor} · {p.firmware.source} · allowOverride [{p.firmware.allowOverride.join(", ")}]
          </span>
        </>
      )}
    </div>
  );
}

function LaunchForm({ busy, onLaunch, onCatalog }: { busy: boolean; onLaunch: (path: string) => void; onCatalog: () => Promise<{ label: string; path: string }[]> }) {
  const [path, setPath] = useState(new URLSearchParams(location.search).get("manifest") ?? "");
  const [entries, setEntries] = useState<{ label: string; path: string }[]>([]);
  useEffect(() => {
    onCatalog()
      .then(setEntries)
      .catch(() => setEntries([]));
  }, [onCatalog]);
  return (
    <div className="ipk-panel connect">
      <h2 style={{ marginTop: 0 }}>セッションがありません</h2>
      <p className="sub" style={{ color: "var(--sub)", fontSize: 12.5 }}>
        ローカル manifest を指定して組み立てます (開発用。サーバのカタログからの起動は M3)。
      </p>
      {entries.length > 0 && (
        <select value="" onChange={(e) => setPath(e.target.value)} style={{ marginBottom: 8 }}>
          <option value="">ローカルカタログから選ぶ…</option>
          {entries.map((e) => (
            <option key={e.path} value={e.path}>
              {e.label}
            </option>
          ))}
        </select>
      )}
      <input value={path} onChange={(e) => setPath(e.target.value)} placeholder="/path/to/dev/cartpole.manifest.json" />
      <button className="ipk-btn" disabled={busy || !path} onClick={() => onLaunch(path)}>
        組み立てる
      </button>
    </div>
  );
}

// 使っていない import を lint に怒られないための参照 (将来の NOT_IMPLEMENTED 分岐用)
void codes;
