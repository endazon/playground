/**
 * Electron main: アプリの寿命、app:// プロトコル、ipemu:// ハンドラ、ブリッジ IPC、エンジンの起動・監視。
 * 全ページで nodeIntegration: false / contextIsolation: true / sandbox: true。
 */
import { app, ipcMain, protocol, net, BrowserWindow, type IpcMainInvokeEvent } from "electron";
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import type { ConsentRequest, LaunchParams } from "@ipemu/ipc-types";
import { serveFile } from "./app-protocol";
import { BrowserShell, type TabKind } from "./browser";
import { showDialog } from "./dialogs";
import { locateEngine } from "./engine";
import { describeLink, parseLaunchUrl, type LaunchLink } from "./ipemu-url";
import { SessionManager } from "./session";
import { loadSettings, saveSettings, type ShellSettings } from "./settings";
import { isTrustedOrigin } from "./trusted-origins";

// ---- 定数と場所 -------------------------------------------------------------
const distDir = resolve(__dirname, ".."); // dist/
const rendererDir = join(distDir, "renderer");
const preloads = {
  bridge: join(distDir, "preload", "bridge.cjs"),
  simui: join(distDir, "preload", "simui.cjs"),
  shellui: join(distDir, "preload", "shellui.cjs"),
};
// 開発ツリーの WasmEmuSil ルート。packaged では asar の外の親を指してしまうので使わない (同梱先は resources/bin)
const devRoot = app.isPackaged ? undefined : resolve(distDir, "..", "..", "..");
const settingsPath = () => join(app.getPath("userData"), "settings.json");

let settings: ShellSettings;
let browser: BrowserShell | null = null;
let sessions: SessionManager;
const log = (line: string) => console.log(`[ipemu-shell] ${line}`);

// app:// は標準スキームとして登録 (fetch / ES module / localStorage が使える)。ready 前に呼ぶ必要がある
protocol.registerSchemesAsPrivileged([
  { scheme: "app", privileges: { standard: true, secure: true, supportFetchAPI: true, corsEnabled: false, stream: true } },
]);

// 単一インスタンス: ipemu:// で 2 つ目が起動したら argv を既存インスタンスに渡す
if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on("second-instance", (_e, argv) => {
    const link = argv.find((a) => a.startsWith("ipemu:"));
    if (link) void handleLaunchLink(link);
    if (browser) browser.window.focus();
  });
  app.on("open-url", (e, url) => {
    e.preventDefault();
    void whenReady().then(() => handleLaunchLink(url));
  });
  void main();
}

let readyPromise: Promise<void> | null = null;
function whenReady() {
  return (readyPromise ??= app.whenReady());
}

async function main() {
  // root での実行 (コンテナ等) は Chromium のサンドボックスが使えないので明示フラグでだけ許す
  if (process.argv.includes("--no-sandbox-if-root") && process.getuid?.() === 0) app.commandLine.appendSwitch("no-sandbox");
  // テスト用: 設定とキャッシュを別の場所に置く
  if (process.env.IPEMU_SHELL_USER_DATA) app.setPath("userData", process.env.IPEMU_SHELL_USER_DATA);
  await whenReady();
  settings = loadSettings(settingsPath());
  log(`settings: ${settingsPath()} trustedServers=${JSON.stringify(settings.trustedServers)}`);

  // ipemu:// の登録 (初回起動時)。開発中 (electron .) は実行ファイルが electron なので、packaged のときだけ
  if (settings.registerProtocol && app.isPackaged) {
    const ok = app.setAsDefaultProtocolClient("ipemu");
    log(`ipemu:// handler registered: ${ok}`);
  }

  // app:// の配信
  protocol.handle("app", (req) => {
    const r = serveFile({ shell: rendererDir, ui: sessions.uiRoot ?? undefined }, req.url);
    return new Response(r.body as BodyInit, { status: r.status, headers: r.headers });
  });

  // エンジン
  const enginePath = locateEngine({
    configured: settings.enginePath,
    bundledDir: app.isPackaged ? join(process.resourcesPath, "bin") : undefined,
    devRoot,
  });
  if (!enginePath) {
    log(`ipemu-engine が見つかりません。settings.enginePath か IPEMU_ENGINE で指定してください (開発: cargo build -p ipemu-engine)`);
  }
  const catalogDirs = settings.catalogDirs.length ? settings.catalogDirs : devRoot && existsSync(join(devRoot, "dev")) ? [join(devRoot, "dev")] : [];
  sessions = new SessionManager({
    enginePath: enginePath ?? "ipemu-engine",
    engineArgs: catalogDirs.flatMap((d) => ["--catalog-dir", d]),
    uiCacheDir: join(app.getPath("userData"), "ui-cache"),
    log,
  });
  sessions.on("toast", (t) => toast(t.level, t.text));
  sessions.on("endpoint", (ep) => browser?.sendToKind("sim", "simui:endpoint-changed", ep));
  sessions.on("consent-required", (req: ConsentRequest) => void handleConsent(req));
  sessions.on("session", () => {
    // UI バンドルが変わったかもしれないので、シミュレータタブを開き直す
    const t = browser?.findTab((k) => k === "sim");
    if (t) t.view.webContents.reload();
  });

  registerIpc();
  browser = new BrowserShell({
    preloads,
    allowNavigation,
    onTabsChanged: (tabs) => browser?.chromeContents.send("shell:tabs", tabs),
  });
  browser.newTab(settings.homeUrl ?? "app://shell/home.html", settings.homeUrl ? "web" : "home");

  const link = process.argv.find((a) => a.startsWith("ipemu:"));
  if (link) void handleLaunchLink(link);

  app.on("window-all-closed", () => {
    void sessions.shutdownEngine().finally(() => app.quit());
  });
  app.on("before-quit", () => {
    // 開きっぱなしのダイアログ (確認 / 同意) が終了を妨げないよう先に閉じる
    for (const w of BrowserWindow.getAllWindows()) if (!w.isDestroyed()) w.destroy();
    void sessions.shutdownEngine();
  });
}

/** タブの種類ごとの遷移制限 (07: 任意サイトは app:// にも IPC にも到達できない) */
function allowNavigation(kind: TabKind, url: string): boolean {
  if (kind === "sim") return url.startsWith("app://ui/");
  if (kind === "home") return url.startsWith("app://shell/");
  return /^(https?:|about:blank)/.test(url);
}

function toast(level: string, text: string) {
  log(`${level}: ${text}`);
  browser?.chromeContents.send("shell:toast", { level, text });
}

// ---- ipemu:// -----------------------------------------------------------------
async function handleLaunchLink(raw: string) {
  const parsed = parseLaunchUrl(raw);
  if (!parsed.ok) {
    toast("error", `ipemu リンクを受け付けません: ${parsed.reason}`);
    return;
  }
  const link = parsed.link;
  const problems: string[] = [];
  if (link.server && !isTrustedOrigin(link.server + "/", settings.trustedServers, { allowLocalhost: settings.allowLocalhost })) {
    problems.push(`${link.server} は信頼するサーバに登録されていません`);
  }
  if (link.manifestPath && !settings.allowLocalLaunch) problems.push("ローカル manifest の起動は設定で無効です");
  // 外部から届くので必ず確認する。信頼していないサーバは拒否 (ボタンを出さない)
  const answer = await showDialog<{ ok: boolean }>({
    page: "confirm",
    preload: preloads.shellui,
    parent: undefined,
    data: { summary: describeLink(link), link, problems, raw },
  });
  if (!answer?.ok || problems.length) return;
  await launchFromLink(link);
}

async function launchFromLink(link: LaunchLink) {
  const params: LaunchParams = link.manifestPath
    ? { source: { kind: "local", manifestPath: link.manifestPath, allowUnhashed: true }, firmware: {}, params: [], pacing: { mode: "realtime", speed: 1 }, autoStart: false }
    : {
        source: {
          kind: "server",
          server: link.server!,
          manifestId: link.manifestId!,
          revision: link.revision,
          credential: { kind: "ticket", ticket: link.ticket ?? "" },
          offline: false,
        },
        firmware: {},
        params: [],
        pacing: { mode: "realtime", speed: 1 },
        autoStart: false,
      };
  await doLaunch(params);
}

async function doLaunch(params: LaunchParams) {
  try {
    const desc = await sessions.launch(params);
    toast("info", `組み立てました: ${desc.manifest.name}`);
    openSimulator();
    return { ok: true, session: { id: desc.manifest.id, revision: desc.manifest.revision, name: desc.manifest.name } };
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    toast("error", `起動に失敗: ${msg}`);
    return { ok: false, error: msg };
  }
}

function openSimulator() {
  if (!browser) return;
  if (!sessions.uiRoot) {
    toast("warn", "UI バンドルが未展開です (manifest の ui.bundle を確認してください)");
    return;
  }
  const t = browser.findTab((k) => k === "sim");
  if (t) {
    browser.selectTab(t.id);
    t.view.webContents.reload();
  } else browser.newTab("app://ui/index.html", "sim");
}

// ---- 同意 (M5 の受け口) --------------------------------------------------------
async function handleConsent(req: ConsentRequest) {
  const parent = browser ? BrowserWindow.getFocusedWindow() ?? undefined : undefined;
  const a = await showDialog<{ grant: boolean; remember: boolean }>({ page: "consent", preload: preloads.shellui, data: req, parent });
  const grant = a?.grant ?? false;
  const remember = a?.remember ?? false;
  try {
    await sessions.consentAnswer(req.requestId, grant, remember);
  } catch (e) {
    toast("error", `同意の送信に失敗: ${String(e)}`);
  }
}

// ---- IPC ----------------------------------------------------------------------
function senderIs(event: IpcMainInvokeEvent | Electron.IpcMainEvent, pred: (url: string, kind: TabKind | "chrome" | "dialog") => boolean): boolean {
  const url = event.senderFrame?.url ?? "";
  const tab = browser?.tabOfContents(event.sender.id);
  const kind: TabKind | "chrome" | "dialog" = tab ? tab.kind : event.sender.id === browser?.chromeContents.id ? "chrome" : "dialog";
  return pred(url, kind);
}
const isShellPage = (url: string) => url.startsWith("app://shell/");
const isTrustedWeb = (url: string) => isTrustedOrigin(url, settings.trustedServers, { allowLocalhost: settings.allowLocalhost });

function registerIpc() {
  // ブリッジ注入の可否。renderer の申告ではなく senderFrame の URL で判定する
  ipcMain.on("ipemu:bridge-allowed", (event) => {
    event.returnValue = senderIs(event, (url, kind) => kind === "web" && isTrustedWeb(url));
  });
  const guardBridge = (event: IpcMainInvokeEvent) => {
    if (!senderIs(event, (url, kind) => (kind === "web" && isTrustedWeb(url)) || ((kind === "home" || kind === "chrome") && isShellPage(url)))) {
      throw new Error("この origin からは許可されていません");
    }
  };
  ipcMain.handle("ipemu:launch", async (event, req: unknown) => {
    guardBridge(event);
    const r = (req ?? {}) as { server?: string; manifestId?: string; revision?: number; sessionToken?: string; manifestPath?: string };
    const origin = new URL(event.senderFrame?.url ?? "about:blank").origin;
    // ローカル manifest (sha256 検証なし) の起動を Web ページから頼めるのは開発設定のときだけ。
    // ipemu:// と同じ扱い (監査 M2-1)。シェル自身のローカルカタログは shell:launch-local を使う
    if (r.manifestPath && !settings.allowLocalLaunch) {
      throw new Error("ローカル manifest の起動は設定 (allowLocalLaunch) で無効です");
    }
    const params: LaunchParams = r.manifestPath
      ? { source: { kind: "local", manifestPath: r.manifestPath, allowUnhashed: true }, firmware: {}, params: [], pacing: { mode: "realtime", speed: 1 }, autoStart: false }
      : {
          source: {
            kind: "server",
            server: r.server ?? origin, // ページ自身の origin を既定にする (他サーバへ資格情報を流させない)
            manifestId: String(r.manifestId ?? ""),
            revision: typeof r.revision === "number" ? r.revision : undefined,
            credential: { kind: "sessionToken", token: String(r.sessionToken ?? "") },
            offline: false,
          },
          firmware: {},
          params: [],
          pacing: { mode: "realtime", speed: 1 },
          autoStart: false,
        };
    if (params.source.kind === "server" && params.source.server !== origin) throw new Error("server はページ自身の origin である必要があります");
    return doLaunch(params);
  });
  ipcMain.handle("ipemu:status", (event) => {
    guardBridge(event);
    return sessions.status();
  });
  ipcMain.handle("ipemu:open-simulator", (event) => {
    guardBridge(event);
    openSimulator();
  });

  // シミュレータ UI (app://ui) にだけ接続先を渡す
  const guardSim = (event: IpcMainInvokeEvent) => {
    if (!senderIs(event, (url, kind) => kind === "sim" && url.startsWith("app://ui/"))) throw new Error("not allowed");
  };
  ipcMain.handle("simui:endpoint", (event) => {
    guardSim(event);
    const d = sessions.discovery;
    return d ? { port: d.port, token: d.token } : null;
  });
  ipcMain.handle("simui:restart-engine", async (event) => {
    guardSim(event);
    await sessions.restart();
  });

  // シェル自身のページ (chrome / home)
  const guardShell = (event: IpcMainInvokeEvent) => {
    if (!senderIs(event, (url, kind) => (kind === "chrome" || kind === "home") && isShellPage(url))) throw new Error("not allowed");
  };
  ipcMain.handle("shell:navigate", (e, url: string) => {
    guardShell(e);
    browser?.navigate(normalizeAddress(url));
  });
  ipcMain.handle("shell:back", (e) => (guardShell(e), browser?.back()));
  ipcMain.handle("shell:forward", (e) => (guardShell(e), browser?.forward()));
  ipcMain.handle("shell:reload", (e) => (guardShell(e), browser?.reload()));
  ipcMain.handle("shell:new-tab", (e, url?: string) => {
    guardShell(e);
    const u = url ? normalizeAddress(url) : "app://shell/home.html";
    browser?.newTab(u, u.startsWith("app://shell/") ? "home" : "web");
  });
  ipcMain.handle("shell:close-tab", (e, id: number) => (guardShell(e), browser?.closeTab(id)));
  ipcMain.handle("shell:select-tab", (e, id: number) => (guardShell(e), browser?.selectTab(id)));
  ipcMain.handle("shell:get-settings", (e) => (guardShell(e), { ...settings, settingsPath: settingsPath() }));
  ipcMain.handle("shell:set-settings", (e, patch: Partial<ShellSettings>) => {
    guardShell(e);
    settings = { ...settings, ...patch };
    saveSettings(settingsPath(), settings);
    return settings;
  });
  ipcMain.handle("shell:catalog-local", async (e) => {
    guardShell(e);
    return sessions.catalogLocal();
  });
  ipcMain.handle("shell:launch-local", (e, manifestPath: string) => {
    guardShell(e);
    return doLaunch({ source: { kind: "local", manifestPath, allowUnhashed: true }, firmware: {}, params: [], pacing: { mode: "realtime", speed: 1 }, autoStart: false });
  });
}

/** アドレスバーの入力を URL にする */
export function normalizeAddress(input: string): string {
  const s = input.trim();
  if (!s) return "app://shell/home.html";
  if (/^(https?|app|about):/i.test(s)) return s;
  if (/^[\w.-]+(:\d+)?(\/.*)?$/.test(s) && (s.includes(".") || s.startsWith("localhost"))) return `https://${s}`;
  return `https://duckduckgo.com/?q=${encodeURIComponent(s)}`;
}

void net;
