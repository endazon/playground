/**
 * `ipemu://launch?...` の解析 (02-architecture.md の起動シーケンス、07 の「ipemu://launch は必ず確認ダイアログ」)。
 * 外部 (Chrome / Edge / メール) から届くので、厳しく検証して構造化する。
 */
export interface LaunchLink {
  /** サーバの origin (https のみ) */
  server?: string;
  manifestId?: string;
  revision?: number;
  /** 単回チケット (エンジンが短命トークンに交換する) */
  ticket?: string;
  /** 開発用: ローカル manifest のパス (設定 allowLocalLaunch のときだけ受理) */
  manifestPath?: string;
}

export type ParseResult = { ok: true; link: LaunchLink } | { ok: false; reason: string };

const ID_RE = /^[a-z0-9][a-z0-9-]{0,63}$/;

export function parseLaunchUrl(raw: string): ParseResult {
  let u: URL;
  try {
    u = new URL(raw);
  } catch {
    return { ok: false, reason: "URL として解釈できません" };
  }
  if (u.protocol !== "ipemu:") return { ok: false, reason: `scheme が ipemu ではありません (${u.protocol})` };
  // `ipemu://launch?...` は host = launch。`ipemu:launch?...` (host なし) は pathname = launch
  const action = (u.host || u.pathname.replace(/^\/+/, "")).toLowerCase();
  if (action !== "launch") return { ok: false, reason: `未対応の操作 \`${action}\` (launch のみ)` };
  const q = u.searchParams;
  const link: LaunchLink = {};
  const server = q.get("server");
  if (server) {
    let s: URL;
    try {
      s = new URL(server);
    } catch {
      return { ok: false, reason: "server が URL ではありません" };
    }
    if (s.protocol !== "https:" && !(s.protocol === "http:" && (s.hostname === "localhost" || s.hostname === "127.0.0.1"))) {
      return { ok: false, reason: "server は https である必要があります" };
    }
    link.server = s.origin;
  }
  const manifest = q.get("manifest") ?? q.get("manifestId");
  if (manifest) {
    if (!ID_RE.test(manifest)) return { ok: false, reason: "manifest id の形式が不正です" };
    link.manifestId = manifest;
  }
  const rev = q.get("rev") ?? q.get("revision");
  if (rev !== null && rev !== "") {
    if (!/^\d{1,9}$/.test(rev)) return { ok: false, reason: "rev は整数である必要があります" };
    link.revision = Number(rev);
  }
  const ticket = q.get("ticket");
  if (ticket) {
    if (ticket.length > 512 || !/^[A-Za-z0-9._~-]+$/.test(ticket)) return { ok: false, reason: "ticket の形式が不正です" };
    link.ticket = ticket;
  }
  const manifestPath = q.get("manifestPath");
  if (manifestPath) {
    if (manifestPath.includes("\0") || manifestPath.length > 4096) return { ok: false, reason: "manifestPath が不正です" };
    link.manifestPath = manifestPath;
  }
  if (!link.manifestPath && !(link.server && link.manifestId)) {
    return { ok: false, reason: "server と manifest (または manifestPath) が必要です" };
  }
  return { ok: true, link };
}

/** 確認ダイアログに出す 1 行要約 */
export function describeLink(l: LaunchLink): string {
  if (l.manifestPath) return `ローカル manifest ${l.manifestPath}`;
  return `${l.server} の構成 ${l.manifestId}${l.revision !== undefined ? ` rev ${l.revision}` : " (最新)"}${l.ticket ? " (チケット付き)" : ""}`;
}
