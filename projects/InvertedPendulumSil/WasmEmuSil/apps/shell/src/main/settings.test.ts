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
});
