import { describe, expect, it } from "vitest";
import { engineCandidates, parseDiscoveryLine } from "./engine";

describe("engine", () => {
  it("Discovery 行の解釈", () => {
    expect(parseDiscoveryLine('{"port":54321,"token":"abc","pid":7,"ipcVersion":1,"engineVersion":"0.1.0"}\n')).toEqual({
      port: 54321,
      token: "abc",
      pid: 7,
      ipcVersion: 1,
      engineVersion: "0.1.0",
    });
    expect(parseDiscoveryLine("not json")).toBeNull();
    expect(parseDiscoveryLine('{"port":"x","token":"a"}')).toBeNull();
    expect(parseDiscoveryLine('{"port":1,"token":""}')).toBeNull();
  });
  it("実行ファイルの探索順: 設定 → 環境変数 → 同梱 → 開発ツリー", () => {
    const c = engineCandidates({ configured: "/cfg/e", bundledDir: "/res/bin", devRoot: "/dev", env: { IPEMU_ENGINE: "/env/e" }, platform: "linux" });
    expect(c).toEqual(["/cfg/e", "/env/e", "/res/bin/ipemu-engine", "/dev/target/release/ipemu-engine", "/dev/target/debug/ipemu-engine"]);
    expect(engineCandidates({ bundledDir: "C:\\app", env: {}, platform: "win32" })[0]).toMatch(/ipemu-engine\.exe$/);
  });
});
