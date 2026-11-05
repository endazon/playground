import { describe, expect, it } from "vitest";
import { describeLink, parseLaunchUrl } from "./ipemu-url";

describe("parseLaunchUrl", () => {
  it("サーバのカタログ起動リンク", () => {
    const r = parseLaunchUrl("ipemu://launch?server=https%3A%2F%2Fipemu.example&manifest=cartpole-lqr-default&rev=3&ticket=abc.def-1");
    expect(r).toEqual({ ok: true, link: { server: "https://ipemu.example", manifestId: "cartpole-lqr-default", revision: 3, ticket: "abc.def-1" } });
    if (r.ok) expect(describeLink(r.link)).toContain("cartpole-lqr-default rev 3");
  });
  it("host なしの形 (ipemu:launch?...) も受ける", () => {
    const r = parseLaunchUrl("ipemu:launch?server=https://s.example&manifest=x");
    expect(r.ok).toBe(true);
  });
  it("ローカル manifest (開発用)", () => {
    const r = parseLaunchUrl("ipemu://launch?manifestPath=/home/me/dev/cartpole.manifest.json");
    expect(r).toEqual({ ok: true, link: { manifestPath: "/home/me/dev/cartpole.manifest.json" } });
  });
  it("拒否するもの", () => {
    const bad = [
      "https://ipemu.example/launch",
      "ipemu://open?server=https://s.example&manifest=x",
      "ipemu://launch?server=http://s.example&manifest=x", // http は localhost 以外不可
      "ipemu://launch?server=https://s.example&manifest=Bad_ID",
      "ipemu://launch?server=https://s.example&manifest=x&rev=three",
      "ipemu://launch?server=https://s.example", // manifest なし
      "ipemu://launch?manifest=x", // server なし
      "ipemu://launch?server=https://s.example&manifest=x&ticket=a%20b",
      "not a url",
    ];
    for (const u of bad) {
      const r = parseLaunchUrl(u);
      expect(r.ok, u).toBe(false);
    }
  });
  it("server の path や資格情報は落として origin だけ使う", () => {
    const r = parseLaunchUrl("ipemu://launch?server=https://s.example:8443/portal/x&manifest=x");
    expect(r.ok && r.link.server).toBe("https://s.example:8443");
  });
});
