/**
 * 信頼 origin の判定 (07-client-app.md「シェル」)。
 * settings の trustedServers とページの origin を**厳密比較**する。サブドメインやパスでの前方一致はしない。
 * HTTPS 必須。開発時だけ http://localhost / 127.0.0.1 を許す。
 */
export interface TrustOptions {
  /** http://localhost と http://127.0.0.1 を許す (開発用) */
  allowLocalhost?: boolean;
}

/** 設定に書かれた文字列 (URL や origin) を正規化した origin にする。不正なら null */
export function normalizeOrigin(entry: string): string | null {
  const s = entry.trim();
  if (!s) return null;
  let u: URL;
  try {
    u = new URL(s.includes("://") ? s : `https://${s}`);
  } catch {
    return null;
  }
  if (u.protocol !== "https:" && u.protocol !== "http:") return null;
  if (u.username || u.password) return null;
  return u.origin;
}

function isLoopbackHost(host: string): boolean {
  return host === "localhost" || host === "127.0.0.1" || host === "[::1]";
}

/** `url` のフレームにブリッジを注入してよいか */
export function isTrustedOrigin(url: string, trustedServers: readonly string[], opts: TrustOptions = {}): boolean {
  let u: URL;
  try {
    u = new URL(url);
  } catch {
    return false;
  }
  if (u.origin === "null") return false; // about:blank, data:, file: など
  if (u.protocol === "https:") {
    // 何もしない
  } else if (u.protocol === "http:") {
    if (!(opts.allowLocalhost && isLoopbackHost(u.hostname))) return false;
  } else {
    return false;
  }
  const origin = u.origin;
  for (const entry of trustedServers) {
    const t = normalizeOrigin(entry);
    if (!t) continue;
    if (t.startsWith("http:") && !(opts.allowLocalhost && isLoopbackHost(new URL(t).hostname))) continue;
    if (t === origin) return true;
  }
  return false;
}
