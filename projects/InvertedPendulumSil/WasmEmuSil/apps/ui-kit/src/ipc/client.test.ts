import { encode } from "@msgpack/msgpack";
import { describe, expect, it } from "vitest";
import { decodeBinaryFrame, EngineClient, endpointUrl, RpcCallError } from "./client";

describe("decodeBinaryFrame", () => {
  it("state フレームを種別バイトで見分ける", () => {
    const payload = encode({ tNs: 5, tick: 5, epoch: 1, state: "running", pacing: "fast", speed: 1, values: [1, 2], stats: {} });
    const bytes = new Uint8Array(payload.length + 1);
    bytes[0] = 0x01;
    bytes.set(payload, 1);
    let got: unknown = null;
    expect(decodeBinaryFrame(bytes, { state: (f) => (got = f) })).toBe("state");
    expect(got).toMatchObject({ tNs: 5, values: [1, 2] });
  });
  it("未知の種別は無視", () => {
    expect(decodeBinaryFrame(new Uint8Array([0x7f, 0x80]), {})).toBeNull();
  });
});

describe("EngineClient", () => {
  it("endpointUrl はトークンをエンコードする", () => {
    expect(endpointUrl(1234, "a b")).toBe("ws://127.0.0.1:1234/?token=a%20b");
  });
  it("応答を id で対応付け、error は RpcCallError", async () => {
    // WebSocket を偽物にして onMessage を直接叩く
    const sent: string[] = [];
    class FakeWs {
      static OPEN = 1;
      readonly OPEN = 1;
      readyState = 1;
      binaryType = "arraybuffer";
      onopen: (() => void) | null = null;
      onmessage: ((ev: { data: unknown }) => void) | null = null;
      onclose: ((ev: { code: number; reason: string }) => void) | null = null;
      onerror: (() => void) | null = null;
      constructor(_url: string) {
        queueMicrotask(() => this.onopen?.());
      }
      send(t: string) {
        sent.push(t);
      }
      close() {
        this.onclose?.({ code: 1000, reason: "" });
      }
    }
    const c = new EngineClient("ws://x", FakeWs as unknown as typeof WebSocket);
    await c.connect();
    const p1 = c.call("run.start", {});
    const p2 = c.call("run.step", { n: 3 });
    expect(JSON.parse(sent[0]!)).toMatchObject({ jsonrpc: "2.0", id: 1, method: "run.start", params: {} });
    expect(JSON.parse(sent[1]!)).toMatchObject({ id: 2, method: "run.step", params: { n: 3 } });
    c.onMessage(JSON.stringify({ jsonrpc: "2.0", id: 2, error: { code: -32001, message: "no session" } }));
    c.onMessage(JSON.stringify({ jsonrpc: "2.0", id: 1, result: { state: "running" } }));
    await expect(p1).resolves.toMatchObject({ state: "running" });
    await expect(p2).rejects.toBeInstanceOf(RpcCallError);
    await p2.catch((e: RpcCallError) => expect(e.code).toBe(-32001));

    const notes: unknown[] = [];
    c.onNotification((n) => notes.push(n));
    c.onMessage(JSON.stringify({ jsonrpc: "2.0", method: "event", params: { kind: "log", message: "hi" } }));
    expect(notes).toHaveLength(1);
  });
});
