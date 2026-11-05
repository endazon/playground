import { describe, expect, it, vi } from "vitest";
import { buildLaunchUrl, hasShellBridge, launchManifest } from "../launch";

describe("launch", () => {
  it("builds the ipemu:// URL in the same shape as the server", () => {
    expect(buildLaunchUrl("https://ipemu.example", "cartpole", 3, "tk_abc")).toBe("ipemu://launch?server=https%3A%2F%2Fipemu.example&manifest=cartpole&rev=3&ticket=tk_abc");
    expect(buildLaunchUrl("http://127.0.0.1:8080", "x", "default", "t")).toContain("rev=default");
  });

  it("detects the shell bridge", () => {
    expect(hasShellBridge({ ipemu: undefined })).toBe(false);
    expect(hasShellBridge({ ipemu: { launch: async () => undefined } })).toBe(true);
  });

  it("uses window.ipemu.launch inside the shell with the agreed argument shape", async () => {
    const launch = vi.fn(async () => undefined);
    const createTicket = vi.fn();
    const r = await launchManifest("cartpole", 2, { server: "https://s", token: "sess_x", createTicket, bridge: { launch } });
    expect(r).toEqual({ mode: "bridge" });
    expect(launch).toHaveBeenCalledWith({ server: "https://s", manifestId: "cartpole", revision: 2, sessionToken: "sess_x" });
    expect(createTicket).not.toHaveBeenCalled();
  });

  it("falls back to a single-use ticket and ipemu:// navigation in a normal browser", async () => {
    const createTicket = vi.fn(async () => ({ launchUrl: "ipemu://launch?server=s&manifest=cartpole&rev=default&ticket=tk_1", expiresAt: "2026-01-01T00:01:00Z" }));
    const navigate = vi.fn();
    const r = await launchManifest("cartpole", "default", { server: "https://s", token: "sess_x", createTicket, navigate, bridge: undefined });
    expect(createTicket).toHaveBeenCalledWith("cartpole", null);
    expect(navigate).toHaveBeenCalledWith("ipemu://launch?server=s&manifest=cartpole&rev=default&ticket=tk_1");
    expect(r.mode).toBe("protocol");
  });
});
