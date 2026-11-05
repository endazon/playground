// 「起動」ボタンの振る舞い (02-architecture.md 起動シーケンス 2)。
// シェル内: window.ipemu.launch({server, manifestId, revision, sessionToken}) (M2 と合わせた引数)
// 通常ブラウザ: 単回チケットを発行して ipemu://launch?... へ遷移。未インストールなら案内を出す

export interface LaunchArgs {
  server: string;
  manifestId: string;
  revision: number | "default";
  sessionToken: string;
}

export interface IpemuBridge {
  launch(args: LaunchArgs): Promise<unknown>;
  status?(): Promise<unknown>;
}

declare global {
  interface Window {
    ipemu?: IpemuBridge;
  }
}

export function hasShellBridge(w: Pick<Window, "ipemu"> = window): boolean {
  return typeof w.ipemu?.launch === "function";
}

/** サーバ側 (POST /api/v1/launch-tickets) と同じ形式。テスト用にクライアントでも組み立てられる */
export function buildLaunchUrl(server: string, manifestId: string, revision: number | "default", ticket: string): string {
  const q = new URLSearchParams({ server, manifest: manifestId, rev: String(revision), ticket });
  return `ipemu://launch?${q.toString()}`;
}

export type LaunchResult = { mode: "bridge" } | { mode: "protocol"; url: string; expiresAt: string };

export interface LaunchDeps {
  server: string;
  token: string;
  createTicket: (manifestId: string, revision: number | null) => Promise<{ launchUrl: string; expiresAt: string }>;
  navigate?: (url: string) => void;
  bridge?: IpemuBridge;
}

export async function launchManifest(manifestId: string, revision: number | "default", deps: LaunchDeps): Promise<LaunchResult> {
  const bridge = deps.bridge ?? (typeof window !== "undefined" ? window.ipemu : undefined);
  if (bridge && typeof bridge.launch === "function") {
    await bridge.launch({ server: deps.server, manifestId, revision, sessionToken: deps.token });
    return { mode: "bridge" };
  }
  const t = await deps.createTicket(manifestId, revision === "default" ? null : revision);
  (deps.navigate ?? ((u: string) => (window.location.href = u)))(t.launchUrl);
  return { mode: "protocol", url: t.launchUrl, expiresAt: t.expiresAt };
}

/** 現在のページの origin。サーバ側の IPEMU_PUBLIC_URL と一致している前提 (ipemu:// のチケット URL はサーバが作る) */
export function currentServer(): string {
  return window.location.origin;
}
