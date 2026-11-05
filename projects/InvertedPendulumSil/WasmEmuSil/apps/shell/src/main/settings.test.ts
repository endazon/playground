import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS, sanitize } from "./settings";

describe("settings", () => {
  it("型の合わない値は落とす", () => {
    const s = sanitize({ trustedServers: ["https://a", 3, null] as unknown as string[], allowLocalhost: false, enginePath: 5 as unknown as string });
    expect(s.trustedServers).toEqual(["https://a"]);
    expect(s.allowLocalhost).toBe(false);
    expect(s.enginePath).toBeUndefined();
    expect(s.registerProtocol).toBe(DEFAULT_SETTINGS.registerProtocol);
  });

  it("ローカル manifest の起動 (allowLocalLaunch) は既定で拒否 (監査 M2-1)", () => {
    expect(DEFAULT_SETTINGS.allowLocalLaunch).toBe(false);
    // settings.json に書かれていなければ既定 (拒否) のまま。明示したときだけ有効
    expect(sanitize({}).allowLocalLaunch).toBe(false);
    expect(sanitize({ allowLocalLaunch: true }).allowLocalLaunch).toBe(true);
  });
});
