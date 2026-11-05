import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { cspFor, mimeFor, resolveUnderRoot, serveFile } from "./app-protocol";

describe("app-protocol", () => {
  const root = mkdtempSync(join(tmpdir(), "ipemu-app-"));
  mkdirSync(join(root, "assets"));
  writeFileSync(join(root, "index.html"), "<html>");
  writeFileSync(join(root, "assets", "a.js"), "1");

  it("root の外へ出られない", () => {
    expect(resolveUnderRoot(root, "/index.html")).toBe(join(root, "index.html"));
    expect(resolveUnderRoot(root, "/")).toBe(join(root, "index.html"));
    expect(resolveUnderRoot(root, "/../etc/passwd")).toBeNull();
    expect(resolveUnderRoot(root, "/assets/%2e%2e/%2e%2e/x")).toBeNull();
    expect(resolveUnderRoot(root, "/a%00.js")).toBeNull();
    expect(resolveUnderRoot(root, "/%zz")).toBeNull();
  });
  it("配信と CSP", () => {
    const roots = { ui: root, shell: undefined };
    const r = serveFile(roots, "app://ui/assets/a.js");
    expect(r.status).toBe(200);
    expect(r.headers["content-type"]).toContain("text/javascript");
    expect(r.headers["content-security-policy"]).toContain("connect-src 'self' ws://127.0.0.1:*");
    expect(serveFile(roots, "app://ui/").status).toBe(200);
    expect(serveFile(roots, "app://ui/nope.js").status).toBe(404);
    expect(serveFile(roots, "app://shell/index.html").status).toBe(404); // root 未設定
    expect(serveFile(roots, "app://evil/index.html").status).toBe(404);
    expect(serveFile(roots, "app://ui/../x").status).not.toBe(200);
  });
  it("mime / csp", () => {
    expect(mimeFor("x.wasm")).toBe("application/wasm");
    expect(mimeFor("x.unknown")).toBe("application/octet-stream");
    expect(cspFor("shell")).not.toContain("ws://");
    expect(cspFor("ui")).toContain("frame-ancestors 'none'");
  });
});
