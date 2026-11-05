import { describe, expect, it } from "vitest";
import { canApprove, canEdit, canViewAdmin, isAdmin } from "../auth/AuthContext";
import { capabilityLines } from "../components/CapabilityList";
import { filterCatalog } from "../pages/portal/CatalogPage";
import { parseBody } from "../pages/admin/RevisionEditorPage";
import type { CatalogItem, Principal } from "../api/types";

const p = (roles: Principal["roles"]): Principal => ({ kind: "user", id: "u", name: "u", roles, groups: [], allowedManifests: [], credential: "session", mustChangePassword: false });

describe("roles", () => {
  it("mirrors the server's role rules", () => {
    expect(canViewAdmin(p(["user"]))).toBe(false);
    expect(canViewAdmin(p(["viewer"]))).toBe(true);
    expect(canEdit(p(["viewer"]))).toBe(false);
    expect(canEdit(p(["editor"]))).toBe(true);
    expect(canApprove(p(["editor"]))).toBe(false);
    expect(canApprove(p(["approver"]))).toBe(true);
    expect(isAdmin(p(["approver"]))).toBe(false);
    expect(isAdmin(p(["admin"]))).toBe(true);
    expect(canViewAdmin(null)).toBe(false);
  });
});

describe("capabilityLines", () => {
  it("formats declarations like the client's consent keys", () => {
    expect(capabilityLines({ net: { tcpConnect: ["192.168.10.5:502"] }, fs: [{ path: "logs", mode: "rw" }], clock: true })).toEqual(["net.tcpConnect:192.168.10.5:502", "fs.rw:logs", "clock (wasi:clocks — 決定性の保証外)"]);
    expect(capabilityLines(undefined)).toEqual([]);
  });
});

describe("catalog filter", () => {
  const items: CatalogItem[] = [
    { id: "cartpole", name: "台車型倒立振子", description: "LQR", defaultRevision: 1, revisions: [], requiresCapabilities: [], updatedAt: "" },
    { id: "other", name: "別の機構", description: "", defaultRevision: 1, revisions: [], requiresCapabilities: [], updatedAt: "" },
  ];
  it("matches id, name and description case-insensitively", () => {
    expect(filterCatalog(items, "CART").map((i) => i.id)).toEqual(["cartpole"]);
    expect(filterCatalog(items, "lqr").map((i) => i.id)).toEqual(["cartpole"]);
    expect(filterCatalog(items, "  ").length).toBe(2);
  });
});

describe("parseBody", () => {
  it("accepts a manifest-shaped object and rejects garbage", () => {
    expect(parseBody('{"components":[],"wires":[]}')).not.toBeNull();
    expect(parseBody("{nope")).toBeNull();
    expect(parseBody('{"x":1}')).toBeNull();
  });
});
