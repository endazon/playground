import { describe, expect, it } from "vitest";
import { isTrustedOrigin, normalizeOrigin } from "./trusted-origins";

describe("normalizeOrigin", () => {
  it("URL でも host だけでも origin にする", () => {
    expect(normalizeOrigin("https://ipemu.example/portal?x=1")).toBe("https://ipemu.example");
    expect(normalizeOrigin("ipemu.example")).toBe("https://ipemu.example");
    expect(normalizeOrigin("https://ipemu.example:8443")).toBe("https://ipemu.example:8443");
    expect(normalizeOrigin("http://localhost:5173")).toBe("http://localhost:5173");
  });
  it("不正・危険なものは null", () => {
    expect(normalizeOrigin("")).toBeNull();
    expect(normalizeOrigin("ftp://x")).toBeNull();
    expect(normalizeOrigin("https://user:pw@x.example")).toBeNull();
    expect(normalizeOrigin("not a url at all ::")).toBeNull();
  });
});

describe("isTrustedOrigin", () => {
  const trusted = ["https://ipemu.example", "https://portal.corp:8443/", "http://localhost:5173"];
  it("厳密一致だけ許す", () => {
    expect(isTrustedOrigin("https://ipemu.example/portal/catalog", trusted)).toBe(true);
    expect(isTrustedOrigin("https://portal.corp:8443/x", trusted)).toBe(true);
    expect(isTrustedOrigin("https://portal.corp/x", trusted)).toBe(false); // ポート違い
    expect(isTrustedOrigin("https://evil.ipemu.example/", trusted)).toBe(false); // サブドメイン
    expect(isTrustedOrigin("https://ipemu.example.evil.com/", trusted)).toBe(false);
    expect(isTrustedOrigin("http://ipemu.example/", trusted)).toBe(false); // http は不可
  });
  it("http は allowLocalhost のときの loopback だけ", () => {
    expect(isTrustedOrigin("http://localhost:5173/", trusted)).toBe(false);
    expect(isTrustedOrigin("http://localhost:5173/", trusted, { allowLocalhost: true })).toBe(true);
    expect(isTrustedOrigin("http://127.0.0.1:5173/", ["http://127.0.0.1:5173"], { allowLocalhost: true })).toBe(true);
    expect(isTrustedOrigin("http://intranet:5173/", ["http://intranet:5173"], { allowLocalhost: true })).toBe(false);
  });
  it("origin のない URL は常に不可", () => {
    for (const u of ["about:blank", "data:text/html,hi", "file:///etc/passwd", "app://ui/index.html", "javascript:1", ""]) {
      expect(isTrustedOrigin(u, [...trusted, "app://ui", "file://"])).toBe(false);
    }
  });
});
