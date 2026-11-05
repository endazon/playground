/**
 * UI バンドル (tar.gz) の最小展開器。ustar / GNU 形式の通常ファイルとディレクトリだけを扱い、
 * シンボリックリンク・絶対パス・`..` を含む経路は拒否する (アーティファクトは sha256 検証済みでも、
 * 展開先の外に書かせない)。外部依存を増やさないために自前で実装している。
 */
import { createHash } from "node:crypto";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, normalize, sep } from "node:path";
import { gunzipSync } from "node:zlib";

export interface TarEntry {
  path: string;
  data: Uint8Array;
}

function str(b: Uint8Array, off: number, len: number): string {
  const end = b.indexOf(0, off);
  const stop = end === -1 || end > off + len ? off + len : end;
  return Buffer.from(b.subarray(off, stop)).toString("utf8");
}

function octal(b: Uint8Array, off: number, len: number): number {
  const s = str(b, off, len).trim();
  return s ? parseInt(s, 8) : 0;
}

/** 展開先に置いてよい相対パスか */
export function isSafeRelativePath(p: string): boolean {
  if (!p || p.includes("\0")) return false;
  if (p.startsWith("/") || p.startsWith("\\") || /^[A-Za-z]:/.test(p)) return false;
  const parts = p.split(/[\\/]+/);
  return parts.every((x) => x !== ".." && x !== "");
}

/** tar (非圧縮) をエントリ列にする */
export function parseTar(buf: Uint8Array): TarEntry[] {
  const out: TarEntry[] = [];
  let off = 0;
  let longName: string | null = null;
  while (off + 512 <= buf.length) {
    const hdr = buf.subarray(off, off + 512);
    if (hdr.every((x) => x === 0)) break; // 終端ブロック
    const name = str(hdr, 0, 100);
    const size = octal(hdr, 124, 12);
    const type = String.fromCharCode(hdr[156] ?? 48);
    const magic = str(hdr, 257, 6);
    const prefix = magic.startsWith("ustar") ? str(hdr, 345, 155) : "";
    const dataStart = off + 512;
    const dataEnd = dataStart + size;
    if (dataEnd > buf.length) throw new Error("tar が途中で切れています");
    const data = buf.subarray(dataStart, dataEnd);
    off = dataStart + Math.ceil(size / 512) * 512;
    if (type === "L") {
      longName = str(data, 0, data.length); // GNU long name: 次のエントリの名前
      continue;
    }
    let full = longName ?? (prefix ? `${prefix}/${name}` : name);
    longName = null;
    full = full.replace(/^\.\//, "");
    if (type === "5" || full.endsWith("/")) continue; // ディレクトリは作成時に補う
    if (type === "x" || type === "g") continue; // pax ヘッダ (属性) は無視
    if (type !== "0" && type !== "\0" && type !== "7") {
      throw new Error(`tar に未対応のエントリ (type ${JSON.stringify(type)}): ${full}`);
    }
    if (!isSafeRelativePath(full)) throw new Error(`tar に危険な経路: ${full}`);
    out.push({ path: full.split("\\").join("/"), data });
  }
  return out;
}

/** tar.gz をメモリ上で展開する */
export function extractTarGz(gz: Uint8Array): TarEntry[] {
  return parseTar(gunzipSync(gz));
}

export function sha256Hex(data: Uint8Array): string {
  return createHash("sha256").update(data).digest("hex");
}

/** エントリをディレクトリに書き出す。`dest` の外には決して書かない */
export function writeEntries(entries: TarEntry[], dest: string): void {
  const root = normalize(dest + sep);
  for (const e of entries) {
    const target = normalize(join(dest, e.path));
    if (!target.startsWith(root)) throw new Error(`展開先の外: ${e.path}`);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, e.data);
  }
}
