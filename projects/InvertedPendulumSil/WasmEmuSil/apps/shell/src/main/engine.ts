/**
 * エンジン (ipemu-engine) プロセスの起動・監視・再起動。
 * - `serve --exit-on-stdin-close` で spawn し、stdout の 1 行目 ({port, token, ...}) を読んで接続先にする
 * - 以降の stdout / stderr はログへ
 * - 予期しない終了は通知し、次の launch で spawn し直す。native FW などの restartRequired では明示的に再起動する
 */
import { spawn, type ChildProcess } from "node:child_process";
import { EventEmitter } from "node:events";
import { existsSync } from "node:fs";
import { join } from "node:path";
import type { Discovery } from "@ipemu/ipc-types";

export interface EngineLocator {
  /** settings.enginePath */
  configured?: string;
  /** 同梱先 (packaged のとき process.resourcesPath/bin) */
  bundledDir?: string;
  /** 開発ツリーの WasmEmuSil ルート (target/{debug,release}/ipemu-engine を探す) */
  devRoot?: string;
  env?: NodeJS.ProcessEnv;
  platform?: NodeJS.Platform;
}

/** エンジン実行ファイルの候補を優先順に返す (存在確認は呼び出し側) */
export function engineCandidates(l: EngineLocator): string[] {
  const exe = (l.platform ?? process.platform) === "win32" ? "ipemu-engine.exe" : "ipemu-engine";
  const env = l.env ?? process.env;
  const out: string[] = [];
  if (l.configured) out.push(l.configured);
  if (env.IPEMU_ENGINE) out.push(env.IPEMU_ENGINE);
  if (l.bundledDir) out.push(join(l.bundledDir, exe));
  if (l.devRoot) {
    out.push(join(l.devRoot, "target", "release", exe));
    out.push(join(l.devRoot, "target", "debug", exe));
  }
  return out;
}

export function locateEngine(l: EngineLocator): string | null {
  for (const c of engineCandidates(l)) if (existsSync(c)) return c;
  return null;
}

/** stdout の 1 行目を Discovery として解釈する。壊れていれば null */
export function parseDiscoveryLine(line: string): Discovery | null {
  try {
    const v = JSON.parse(line.trim()) as Partial<Discovery>;
    if (typeof v.port !== "number" || typeof v.token !== "string" || !v.token) return null;
    return {
      port: v.port,
      token: v.token,
      pid: typeof v.pid === "number" ? v.pid : 0,
      ipcVersion: typeof v.ipcVersion === "number" ? v.ipcVersion : 0,
      engineVersion: typeof v.engineVersion === "string" ? v.engineVersion : "",
    };
  } catch {
    return null;
  }
}

export interface EngineEvents {
  ready: (d: Discovery) => void;
  exit: (info: { code: number | null; signal: NodeJS.Signals | null; expected: boolean }) => void;
  log: (line: string) => void;
}

export class EngineProcess extends EventEmitter {
  private child: ChildProcess | null = null;
  private _discovery: Discovery | null = null;
  private stopping = false;

  constructor(
    private readonly exe: string,
    private readonly extraArgs: string[] = [],
  ) {
    super();
  }

  get discovery(): Discovery | null {
    return this._discovery;
  }
  get running(): boolean {
    return this.child !== null && this.child.exitCode === null;
  }

  /** 起動して Discovery を待つ */
  start(timeoutMs = 30_000): Promise<Discovery> {
    if (this.child) return Promise.reject(new Error("すでに起動しています"));
    this.stopping = false;
    this._discovery = null;
    const args = ["serve", "--exit-on-stdin-close", ...this.extraArgs];
    const child = spawn(this.exe, args, { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
    this.child = child;
    this.emit("log", `spawn ${this.exe} ${args.join(" ")} (pid ${child.pid})`);
    return new Promise<Discovery>((resolve, reject) => {
      let buf = "";
      let got = false;
      const timer = setTimeout(() => {
        if (!got) {
          reject(new Error("エンジンが接続情報を出しません (timeout)"));
          this.stop();
        }
      }, timeoutMs);
      child.stdout!.setEncoding("utf8");
      child.stdout!.on("data", (chunk: string) => {
        if (got) {
          for (const l of chunk.split("\n")) if (l.trim()) this.emit("log", l);
          return;
        }
        buf += chunk;
        const nl = buf.indexOf("\n");
        if (nl === -1) return;
        const line = buf.slice(0, nl);
        const rest = buf.slice(nl + 1);
        const d = parseDiscoveryLine(line);
        got = true;
        clearTimeout(timer);
        if (!d) {
          reject(new Error(`エンジンの 1 行目を解釈できません: ${line}`));
          this.stop();
          return;
        }
        this._discovery = d;
        this.emit("ready", d);
        resolve(d);
        for (const l of rest.split("\n")) if (l.trim()) this.emit("log", l);
      });
      child.stderr!.setEncoding("utf8");
      child.stderr!.on("data", (chunk: string) => {
        for (const l of chunk.split("\n")) if (l.trim()) this.emit("log", l);
      });
      child.on("error", (e) => {
        clearTimeout(timer);
        this.child = null;
        if (!got) reject(e);
        this.emit("exit", { code: null, signal: null, expected: this.stopping });
      });
      child.on("exit", (code, signal) => {
        clearTimeout(timer);
        this.child = null;
        this._discovery = null;
        if (!got) reject(new Error(`エンジンが起動直後に終了しました (code ${code})`));
        this.emit("exit", { code, signal, expected: this.stopping });
      });
    });
  }

  /** 停止 (stdin を閉じる → 猶予後に kill) */
  stop(graceMs = 2000): Promise<void> {
    const child = this.child;
    if (!child) return Promise.resolve();
    this.stopping = true;
    return new Promise((resolve) => {
      const done = () => resolve();
      child.once("exit", done);
      try {
        child.stdin?.end();
      } catch {
        /* 無視 */
      }
      setTimeout(() => {
        if (child.exitCode === null) child.kill();
      }, graceMs);
      setTimeout(() => {
        if (child.exitCode === null) child.kill("SIGKILL");
        resolve();
      }, graceMs + 2000);
    });
  }
}
