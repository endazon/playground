/** シェルの設定 (<userData>/settings.json)。エンジン側の settings.toml とは別物 (こちらは UI の都合だけ) */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

export interface ShellSettings {
  /** ブリッジ (window.ipemu) を注入する origin。厳密一致 */
  trustedServers: string[];
  /** http://localhost を信頼 origin に許す (開発用) */
  allowLocalhost: boolean;
  /** `ipemu://launch?manifestPath=` を受け付ける (開発用) */
  allowLocalLaunch: boolean;
  /** エンジンの実行ファイル (省略時は同梱 → 環境変数 IPEMU_ENGINE → 開発ツリー) */
  enginePath?: string;
  /** catalog.local に渡すディレクトリ */
  catalogDirs: string[];
  /** 起動時に開く URL (省略時は app://shell/home.html) */
  homeUrl?: string;
  /** OS に ipemu:// を登録する */
  registerProtocol: boolean;
}

export const DEFAULT_SETTINGS: ShellSettings = {
  trustedServers: [],
  allowLocalhost: true,
  // ローカル manifest (sha256 検証なし) の起動を Web ページ / ipemu:// から受けるのは開発設定のときだけ (監査 M2-1)。
  // シェル自身のローカルカタログ (shell:launch-local) はこの設定に依らない
  allowLocalLaunch: false,
  catalogDirs: [],
  registerProtocol: true,
};

export function loadSettings(path: string): ShellSettings {
  if (!existsSync(path)) return { ...DEFAULT_SETTINGS };
  try {
    const raw = JSON.parse(readFileSync(path, "utf8")) as Partial<ShellSettings>;
    return sanitize(raw);
  } catch (e) {
    console.warn(`settings.json を読めません (${String(e)})。既定値を使います`);
    return { ...DEFAULT_SETTINGS };
  }
}

export function sanitize(raw: Partial<ShellSettings>): ShellSettings {
  const strs = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : []);
  return {
    trustedServers: strs(raw.trustedServers),
    allowLocalhost: raw.allowLocalhost ?? DEFAULT_SETTINGS.allowLocalhost,
    allowLocalLaunch: raw.allowLocalLaunch ?? DEFAULT_SETTINGS.allowLocalLaunch,
    enginePath: typeof raw.enginePath === "string" ? raw.enginePath : undefined,
    catalogDirs: strs(raw.catalogDirs),
    homeUrl: typeof raw.homeUrl === "string" ? raw.homeUrl : undefined,
    registerProtocol: raw.registerProtocol ?? DEFAULT_SETTINGS.registerProtocol,
  };
}

export function saveSettings(path: string, s: ShellSettings): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, JSON.stringify(s, null, 2) + "\n");
}
