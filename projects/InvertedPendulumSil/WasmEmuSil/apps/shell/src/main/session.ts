/**
 * main 側のエンジン操作: spawn → 接続 → session.launch / describe、restartRequired での再起動、
 * consent.required の受信、UI バンドル (tar.gz) の展開先の管理。
 * シェルはシミュレータの状態を持たない。ここで扱うのは「どの構成を、どの上書きで起動したか」だけ。
 */
import { EventEmitter } from "node:events";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { ConsentRequest, Discovery, LaunchParams, Notification, SessionDescription, UiBundleInfo } from "@ipemu/ipc-types";
import { EngineClient, endpointUrl } from "@ipemu/ui-kit/ipc"; // React 部品を main に束ねないよう ipc だけ
import { EngineProcess } from "./engine";
import { extractTarGz, sha256Hex, writeEntries } from "./untar";

export interface SessionManagerOptions {
  enginePath: string;
  engineArgs: string[];
  /** 展開した UI バンドルを置く場所 (<userData>/ui-cache) */
  uiCacheDir: string;
  log: (line: string) => void;
}

export interface EngineStatus {
  engine: { running: boolean; port?: number; pid?: number; version?: string };
  session: { id: string; revision: number; name: string; run: SessionDescription["run"] } | null;
  uiReady: boolean;
}

export class SessionManager extends EventEmitter {
  private proc: EngineProcess | null = null;
  private client: EngineClient | null = null;
  private lastLaunch: LaunchParams | null = null;
  private lastDesc: SessionDescription | null = null;
  /** 展開済み UI バンドルのディレクトリ (app://ui の root) */
  uiRoot: string | null = null;
  private restarting = false;

  constructor(private readonly opts: SessionManagerOptions) {
    super();
  }

  get discovery(): Discovery | null {
    return this.proc?.discovery ?? null;
  }

  /** エンジンが動いていなければ起動して接続する */
  async ensureEngine(): Promise<Discovery> {
    if (this.proc?.running && this.proc.discovery && this.client?.connection === "open") return this.proc.discovery;
    await this.shutdownEngine();
    const proc = new EngineProcess(this.opts.enginePath, this.opts.engineArgs);
    proc.on("log", (l) => this.opts.log(`[engine] ${l}`));
    proc.on("exit", (info) => {
      this.opts.log(`[engine] exit code=${info.code} signal=${info.signal} expected=${info.expected}`);
      this.client?.close();
      this.client = null;
      if (this.proc === proc) this.proc = null;
      this.emit("engine-exit", info);
      // 予期しない終了は、最後の launch を覚えたまま次の操作で自動的に立ち上げ直す
      if (!info.expected && !this.restarting && this.lastLaunch) this.emit("toast", { level: "warn", text: "エンジンが終了しました。次の操作で再起動します" });
    });
    this.proc = proc;
    const d = await proc.start();
    const client = new EngineClient(endpointUrl(d.port, d.token));
    client.onNotification((n) => this.onNotification(n));
    await client.connect();
    this.client = client;
    this.emit("endpoint", { port: d.port, token: d.token });
    return d;
  }

  private onNotification(n: Notification) {
    if (n.method === "consent.required") this.emit("consent-required", n.params as ConsentRequest);
    else if (n.method === "event") {
      const e = n.params;
      if (e.kind === "fault") this.emit("toast", { level: "error", text: `FAULT: ${e.message}` });
      if (e.kind === "firmware" && e.data && (e.data as { restartRequired?: boolean }).restartRequired) {
        // watch で native が更新されたときなど。UI からの firmware.load は UI 側が restartEngine を呼ぶ
        this.emit("toast", { level: "warn", text: e.message });
      }
    }
  }

  /** 構成を起動 (実行中のセッションは置き換わる)。UI バンドルがあれば展開して app://ui を更新する */
  async launch(p: LaunchParams): Promise<SessionDescription> {
    await this.ensureEngine();
    const desc = await this.client!.call("session.launch", p);
    this.lastLaunch = p;
    this.lastDesc = desc;
    this.prepareUi(desc.ui ?? null);
    this.emit("session", desc);
    return desc;
  }

  async describe(): Promise<SessionDescription | null> {
    if (!this.client || this.client.connection !== "open") return null;
    const d = await this.client.call("session.describe", {});
    this.lastDesc = d.session ?? null;
    return this.lastDesc;
  }

  async catalogLocal() {
    await this.ensureEngine();
    return this.client!.call("catalog.local", {});
  }

  async consentAnswer(requestId: string, grant: boolean, remember: boolean) {
    if (!this.client) throw new Error("エンジンに接続していません");
    return this.client.call("consent.answer", { requestId, grant, remember });
  }

  /**
   * エンジンの再起動 (native FW の差し替えなど、05-firmware.md「リビルド後の反映」)。
   * 直近の describe にある上書き (firmware / params) を launch に畳み込んで同じ構成を組み立て直す。
   */
  async restart(): Promise<SessionDescription | null> {
    const base = this.lastLaunch;
    if (!base) {
      await this.shutdownEngine();
      await this.ensureEngine();
      return null;
    }
    const latest = (await this.describe().catch(() => null)) ?? this.lastDesc;
    const merged: LaunchParams = {
      ...base,
      firmware: { ...base.firmware, ...(latest?.overrides.firmware ?? {}) },
      params: mergeParams(base.params, latest?.overrides.params ?? []),
      pacing: latest ? latest.run.pacing : base.pacing,
      autoStart: latest ? latest.run.state === "running" : base.autoStart,
    };
    this.restarting = true;
    try {
      await this.shutdownEngine();
      this.emit("toast", { level: "info", text: "エンジンを再起動しています…" });
      return await this.launch(merged);
    } finally {
      this.restarting = false;
    }
  }

  async status(): Promise<EngineStatus> {
    const d = this.proc?.discovery ?? null;
    const s = await this.describe().catch(() => null);
    return {
      engine: { running: !!this.proc?.running, port: d?.port, pid: d?.pid, version: d?.engineVersion },
      session: s ? { id: s.manifest.id, revision: s.manifest.revision, name: s.manifest.name, run: s.run } : null,
      uiReady: this.uiRoot !== null,
    };
  }

  /**
   * UI バンドル (単一ファイルの tar.gz アーティファクト) を <uiCacheDir>/<sha256>/ に展開する。
   * sha256 が manifest にあればそれで、なければ中身のハッシュでディレクトリを決め、同じものは展開し直さない。
   */
  prepareUi(ui: UiBundleInfo | null): void {
    this.uiRoot = null;
    if (!ui) return;
    if (!existsSync(ui.path)) {
      this.opts.log(`[ui] バンドルがありません: ${ui.path} (apps/sim-ui を build してください)`);
      this.emit("toast", { level: "warn", text: `UI バンドルがありません: ${ui.path}` });
      return;
    }
    try {
      const bytes = readFileSync(ui.path);
      const sha = sha256Hex(bytes);
      if (ui.sha256 && ui.sha256 !== sha) throw new Error(`sha256 不一致 (manifest ${ui.sha256}, 実体 ${sha})`);
      const dir = join(this.opts.uiCacheDir, sha);
      const stamp = join(dir, ".ipemu-ok");
      if (!existsSync(stamp)) {
        rmSync(dir, { recursive: true, force: true });
        mkdirSync(dir, { recursive: true });
        const entries = extractTarGz(bytes);
        if (!entries.some((e) => e.path === "index.html")) throw new Error("index.html がありません");
        writeEntries(entries, dir);
        writeFileSync(stamp, `${ui.artifact}\n`);
      }
      this.uiRoot = dir;
      this.opts.log(`[ui] ${ui.artifact} → ${dir}`);
    } catch (e) {
      this.opts.log(`[ui] 展開に失敗: ${String(e)}`);
      this.emit("toast", { level: "error", text: `UI バンドルを展開できません: ${String(e)}` });
    }
  }

  async shutdownEngine(): Promise<void> {
    this.client?.close();
    this.client = null;
    const p = this.proc;
    this.proc = null;
    if (p) await p.stop();
  }
}

function mergeParams(a: LaunchParams["params"], b: LaunchParams["params"]): LaunchParams["params"] {
  const out = [...a];
  for (const o of b) {
    const i = out.findIndex((x) => x.instance === o.instance && x.key === o.key);
    if (i >= 0) out[i] = o;
    else out.push(o);
  }
  return out;
}
