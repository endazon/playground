/**
 * `app://` プロトコル (07-client-app.md)。
 *   app://ui/...    エンジンのキャッシュ (展開した UI バンドル) を配信。CSP は app:// と ws://127.0.0.1 だけ
 *   app://shell/... シェル自身のページ (chrome / home / confirm / consent)。CSP は self だけ
 * ここは Electron に依存しない純粋な部分 (経路解決 / MIME / CSP) と、登録部分に分けている。
 */
import { readFileSync, statSync } from "node:fs";
import { extname, join, normalize, sep } from "node:path";

const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".map": "application/json",
  ".wasm": "application/wasm",
  ".txt": "text/plain; charset=utf-8",
};

export function mimeFor(path: string): string {
  return MIME[extname(path).toLowerCase()] ?? "application/octet-stream";
}

/** URL の pathname を root 配下のファイルパスにする。root の外なら null。`/` は index.html */
export function resolveUnderRoot(root: string, pathname: string): string | null {
  let p: string;
  try {
    p = decodeURIComponent(pathname);
  } catch {
    return null;
  }
  if (p.includes("\0")) return null;
  if (p === "" || p.endsWith("/")) p += "index.html";
  const target = normalize(join(root, p));
  const base = normalize(root + sep);
  if (!target.startsWith(base)) return null;
  return target;
}

export function cspFor(host: string): string {
  if (host === "ui") {
    // シミュレータ UI: 自分自身と、エンジンのループバック WS だけ。React の style 属性のため style は inline を許す
    return [
      "default-src 'self'",
      "script-src 'self'",
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data: blob:",
      "font-src 'self' data:",
      "connect-src 'self' ws://127.0.0.1:* ws://localhost:*",
      "worker-src 'self' blob:",
      "object-src 'none'",
      "base-uri 'none'",
      "frame-ancestors 'none'",
    ].join("; ");
  }
  return ["default-src 'self'", "style-src 'self' 'unsafe-inline'", "img-src 'self' data:", "object-src 'none'", "base-uri 'none'", "frame-ancestors 'none'"].join("; ");
}

export interface ServedFile {
  status: number;
  headers: Record<string, string>;
  body: Uint8Array;
}

/** 1 リクエストぶんの解決 (fs 読み出しまで)。Electron の protocol.handle から呼ぶ */
export function serveFile(roots: { [host: string]: string | undefined }, url: string): ServedFile {
  let u: URL;
  try {
    u = new URL(url);
  } catch {
    return text(400, "bad url");
  }
  const root = roots[u.hostname];
  if (!root) return text(404, `unknown app host: ${u.hostname}`);
  const path = resolveUnderRoot(root, u.pathname);
  if (!path) return text(403, "forbidden");
  try {
    const st = statSync(path);
    if (!st.isFile()) return text(404, "not found");
    return {
      status: 200,
      headers: {
        "content-type": mimeFor(path),
        "content-security-policy": cspFor(u.hostname),
        "cache-control": u.hostname === "ui" ? "no-cache" : "no-store",
        "x-content-type-options": "nosniff",
      },
      body: readFileSync(path),
    };
  } catch {
    return text(404, "not found");
  }
}

function text(status: number, body: string): ServedFile {
  return { status, headers: { "content-type": "text/plain; charset=utf-8" }, body: Buffer.from(body) };
}
