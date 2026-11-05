/**
 * エンジン IPC クライアント (ブラウザ / Electron renderer / Node 22+ で共通)。
 * 文字列フレーム = JSON-RPC 2.0、バイナリフレーム = 先頭 1 バイト種別 + MessagePack。
 * 型は crates/ipemu-ipc から生成した @ipemu/ipc-types を使い、method → (params, result) を型レベルで引く。
 */
import { decode } from "@msgpack/msgpack";
import type {
  Notification,
  RequestBody,
  Response,
  ResultBody,
  RpcError,
  ScopeFrame,
  StateFrame,
} from "@ipemu/ipc-types";
import { FRAME_SCOPE, FRAME_STATE } from "@ipemu/ipc-types";

export type Method = RequestBody["method"];
export type ParamsOf<M extends Method> = Extract<RequestBody, { method: M }>["params"];
export type ResultOf<M extends Method> = Extract<ResultBody, { method: M }>["result"];

export class RpcCallError extends Error {
  readonly code: number;
  readonly data: unknown;
  constructor(e: RpcError) {
    super(e.message);
    this.name = "RpcCallError";
    this.code = e.code;
    this.data = e.data;
  }
}

export type ConnectionState = "connecting" | "open" | "closed";

type Listener<T> = (v: T) => void;

/** `ws://127.0.0.1:<port>/?token=<token>` を組み立てる */
export function endpointUrl(port: number, token: string): string {
  return `ws://127.0.0.1:${port}/?token=${encodeURIComponent(token)}`;
}

/**
 * WebSocket 1 本ぶんのクライアント。切断時の再接続は呼び出し側 (再接続では購読をやり直す必要があるため)。
 */
export class EngineClient {
  private ws: WebSocket | null = null;
  private nextId = 1;
  private pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
  private stateListeners = new Set<Listener<StateFrame>>();
  private scopeListeners = new Set<Listener<ScopeFrame>>();
  private notifyListeners = new Set<Listener<Notification>>();
  private connListeners = new Set<Listener<ConnectionState>>();
  private _state: ConnectionState = "closed";

  constructor(private readonly url: string, private readonly WebSocketImpl: typeof WebSocket = WebSocket) {}

  get connection(): ConnectionState {
    return this._state;
  }

  /** 接続して open まで待つ。失敗 (トークン不一致など) は reject */
  connect(): Promise<void> {
    if (this.ws) return Promise.resolve();
    this.setConn("connecting");
    return new Promise((resolve, reject) => {
      const ws = new this.WebSocketImpl(this.url);
      ws.binaryType = "arraybuffer";
      this.ws = ws;
      let opened = false;
      ws.onopen = () => {
        opened = true;
        this.setConn("open");
        resolve();
      };
      ws.onmessage = (ev: MessageEvent) => this.onMessage(ev.data);
      ws.onerror = () => {
        /* onclose が続くのでそこで処理 */
      };
      ws.onclose = (ev: CloseEvent) => {
        this.ws = null;
        this.setConn("closed");
        const err = new Error(`接続が閉じました (code ${ev.code}${ev.reason ? `: ${ev.reason}` : ""})`);
        for (const p of this.pending.values()) p.reject(err);
        this.pending.clear();
        if (!opened) reject(err);
      };
    });
  }

  close(): void {
    this.ws?.close();
    this.ws = null;
  }

  /** 型付き呼び出し。エンジンの error は RpcCallError で reject */
  call<M extends Method>(method: M, params: ParamsOf<M>): Promise<ResultOf<M>> {
    const ws = this.ws;
    if (!ws || ws.readyState !== ws.OPEN) {
      return Promise.reject(new Error("エンジンに接続していません"));
    }
    const id = this.nextId++;
    const text = JSON.stringify({ jsonrpc: "2.0", id, method, params });
    return new Promise<ResultOf<M>>((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
      ws.send(text);
    });
  }

  onState(fn: Listener<StateFrame>): () => void {
    this.stateListeners.add(fn);
    return () => this.stateListeners.delete(fn);
  }
  onScope(fn: Listener<ScopeFrame>): () => void {
    this.scopeListeners.add(fn);
    return () => this.scopeListeners.delete(fn);
  }
  onNotification(fn: Listener<Notification>): () => void {
    this.notifyListeners.add(fn);
    return () => this.notifyListeners.delete(fn);
  }
  onConnection(fn: Listener<ConnectionState>): () => void {
    this.connListeners.add(fn);
    return () => this.connListeners.delete(fn);
  }

  private setConn(s: ConnectionState) {
    this._state = s;
    for (const l of this.connListeners) l(s);
  }

  /** 受信フレームの振り分け (テストから直接呼べるよう public 相当の形にしてある) */
  onMessage(data: unknown): void {
    if (typeof data === "string") {
      this.onText(data);
      return;
    }
    const bytes = data instanceof ArrayBuffer ? new Uint8Array(data) : (data as Uint8Array);
    decodeBinaryFrame(bytes, {
      state: (f) => this.stateListeners.forEach((l) => l(f)),
      scope: (f) => this.scopeListeners.forEach((l) => l(f)),
    });
  }

  private onText(text: string) {
    let msg: unknown;
    try {
      msg = JSON.parse(text);
    } catch {
      return;
    }
    if (!msg || typeof msg !== "object") return;
    const m = msg as Partial<Response> & Partial<Notification>;
    if ("id" in m && m.id !== undefined && m.id !== null) {
      const id = typeof m.id === "number" ? m.id : Number(m.id);
      const p = this.pending.get(id);
      if (!p) return;
      this.pending.delete(id);
      if (m.error) p.reject(new RpcCallError(m.error));
      else p.resolve(m.result ?? null);
      return;
    }
    if (typeof m.method === "string") {
      for (const l of this.notifyListeners) l(m as Notification);
    }
  }
}

/** バイナリフレームを種別で振り分ける。未知の種別は無視 */
export function decodeBinaryFrame(
  bytes: Uint8Array,
  on: { state?: (f: StateFrame) => void; scope?: (f: ScopeFrame) => void },
): "state" | "scope" | null {
  if (bytes.length < 2) return null;
  const kind = bytes[0];
  const payload = bytes.subarray(1);
  if (kind === FRAME_STATE) {
    on.state?.(decode(payload) as StateFrame);
    return "state";
  }
  if (kind === FRAME_SCOPE) {
    on.scope?.(decode(payload) as ScopeFrame);
    return "scope";
  }
  return null;
}
