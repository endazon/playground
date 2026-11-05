import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { gzipSync } from "node:zlib";
import { describe, expect, it } from "vitest";
import { extractTarGz, isSafeRelativePath, parseTar, writeEntries } from "./untar";

function makeTar(dir: string, files: Record<string, string>): Buffer {
  const src = join(dir, "src");
  mkdirSync(join(src, "assets"), { recursive: true });
  for (const [p, c] of Object.entries(files)) {
    mkdirSync(join(src, p, ".."), { recursive: true });
    writeFileSync(join(src, p), c);
  }
  const out = join(dir, "b.tar.gz");
  execFileSync("tar", ["-czf", out, "-C", src, "."]);
  return readFileSync(out);
}

/** 手組みの ustar ヘッダ (危険な経路のテスト用) */
function rawEntry(name: string, body: string, type = "0"): Buffer {
  const hdr = Buffer.alloc(512);
  hdr.write(name, 0, "utf8");
  hdr.write("0000644\0", 100);
  hdr.write("0000000\0", 108);
  hdr.write("0000000\0", 116);
  hdr.write(body.length.toString(8).padStart(11, "0") + "\0", 124);
  hdr.write("00000000000\0", 136);
  hdr.write("        ", 148);
  hdr.write(type, 156);
  hdr.write("ustar\0", 257);
  hdr.write("00", 263);
  let sum = 0;
  for (const b of hdr) sum += b;
  hdr.write(sum.toString(8).padStart(6, "0") + "\0 ", 148);
  const data = Buffer.alloc(Math.ceil(body.length / 512) * 512);
  data.write(body);
  return Buffer.concat([hdr, data]);
}

describe("untar", () => {
  it("tar コマンドで作った tar.gz を展開できる", () => {
    const dir = mkdtempSync(join(tmpdir(), "ipemu-untar-"));
    const gz = makeTar(dir, { "index.html": "<html>", "assets/app.js": "console.log(1)", "assets/深い/日本語.css": "a{}" });
    const entries = extractTarGz(gz);
    const paths = entries.map((e) => e.path).sort();
    expect(paths).toEqual(["assets/app.js", "assets/深い/日本語.css", "index.html"]);
    expect(Buffer.from(entries.find((e) => e.path === "index.html")!.data).toString()).toBe("<html>");
    const dest = join(dir, "out");
    writeEntries(entries, dest);
    expect(existsSync(join(dest, "assets/深い/日本語.css"))).toBe(true);
  });
  it("絶対パス・.. ・シンボリックリンクは拒否", () => {
    expect(isSafeRelativePath("a/b.js")).toBe(true);
    expect(isSafeRelativePath("../x")).toBe(false);
    expect(isSafeRelativePath("a/../../x")).toBe(false);
    expect(isSafeRelativePath("/etc/passwd")).toBe(false);
    expect(isSafeRelativePath("C:\\x")).toBe(false);
    const end = Buffer.alloc(1024);
    expect(() => parseTar(Buffer.concat([rawEntry("../evil.js", "x"), end]))).toThrow(/危険な経路/);
    expect(() => parseTar(Buffer.concat([rawEntry("link", "", "2"), end]))).toThrow(/未対応/);
    const ok = parseTar(Buffer.concat([rawEntry("./fine.txt", "hello"), end]));
    expect(ok).toHaveLength(1);
    expect(ok[0]!.path).toBe("fine.txt");
  });
  it("gzip されていないものはエラー", () => {
    expect(() => extractTarGz(Buffer.from("plain"))).toThrow();
    expect(extractTarGz(gzipSync(Buffer.alloc(1024)))).toEqual([]);
  });
});
