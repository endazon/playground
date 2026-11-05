/**
 * 汎用ブラウザ UI: 1 つの BaseWindow に、上部の chrome (タブ / アドレスバー、app://shell/chrome.html) と
 * タブごとの WebContentsView を並べる。タブは任意のサイトを開けるが、
 *   - 任意サイトのタブ: preload = bridge (信頼 origin にだけ window.ipemu)。app:// へは遷移できない
 *   - シミュレータタブ: preload = simui。app://ui 以外へは遷移できない
 * と、タブの種類で preload と遷移可能な範囲を固定する。
 */
import { BaseWindow, WebContentsView, shell as electronShell, type WebContents } from "electron";

export type TabKind = "web" | "sim" | "home";

export interface TabInfo {
  id: number;
  kind: TabKind;
  title: string;
  url: string;
  loading: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  active: boolean;
}

interface Tab {
  id: number;
  kind: TabKind;
  view: WebContentsView;
}

const CHROME_HEIGHT = 78;

export interface BrowserShellOptions {
  preloads: { bridge: string; simui: string; shellui: string };
  /** 任意サイトからの遷移・window.open の可否 (app:// などを塞ぐ) */
  allowNavigation: (kind: TabKind, url: string) => boolean;
  onTabsChanged: (tabs: TabInfo[]) => void;
}

export class BrowserShell {
  readonly window: BaseWindow;
  private chrome: WebContentsView;
  private tabs: Tab[] = [];
  private activeId: number | null = null;
  private nextId = 1;

  constructor(private readonly opts: BrowserShellOptions) {
    this.window = new BaseWindow({ width: 1440, height: 960, title: "ipemu" });
    this.window.setMenuBarVisibility(false); // タブ / アドレスバーが chrome。メニューは出さない
    this.chrome = new WebContentsView({
      webPreferences: { preload: opts.preloads.shellui, sandbox: true, contextIsolation: true, nodeIntegration: false },
    });
    this.window.contentView.addChildView(this.chrome);
    void this.chrome.webContents.loadURL("app://shell/chrome.html");
    this.window.on("resize", () => this.layout());
    this.layout();
  }

  get chromeContents(): WebContents {
    return this.chrome.webContents;
  }

  private layout() {
    const [w = 0, h = 0] = this.window.getContentSize();
    this.chrome.setBounds({ x: 0, y: 0, width: w, height: CHROME_HEIGHT });
    for (const t of this.tabs) {
      t.view.setBounds({ x: 0, y: CHROME_HEIGHT, width: w, height: Math.max(0, h - CHROME_HEIGHT) });
      t.view.setVisible(t.id === this.activeId);
    }
  }

  private preloadFor(kind: TabKind): string {
    return kind === "sim" ? this.opts.preloads.simui : kind === "home" ? this.opts.preloads.shellui : this.opts.preloads.bridge;
  }

  newTab(url: string, kind: TabKind = "web"): TabInfo {
    const view = new WebContentsView({
      webPreferences: {
        preload: this.preloadFor(kind),
        sandbox: true,
        contextIsolation: true,
        nodeIntegration: false,
        webviewTag: false,
      },
    });
    const id = this.nextId++;
    const tab: Tab = { id, kind, view };
    const wc = view.webContents;
    // 遷移の制限: タブの種類に合わない URL へは行かせない
    wc.on("will-navigate", (e, target) => {
      if (!this.opts.allowNavigation(kind, target)) e.preventDefault();
    });
    wc.on("will-redirect", (e, target) => {
      if (!this.opts.allowNavigation(kind, target)) e.preventDefault();
    });
    wc.setWindowOpenHandler(({ url: target }) => {
      if (kind === "web" && this.opts.allowNavigation("web", target)) this.newTab(target, "web");
      else if (/^https?:/.test(target)) void electronShell.openExternal(target);
      return { action: "deny" };
    });
    // 権限要求 (カメラ等) は一律拒否。ブラウザとしての最小構成
    wc.session.setPermissionRequestHandler((_wc, _perm, cb) => cb(false));
    for (const ev of ["did-start-loading", "did-stop-loading", "did-navigate", "did-navigate-in-page", "page-title-updated"] as const) {
      wc.on(ev as "did-navigate", () => this.notify());
    }
    this.window.contentView.addChildView(view);
    this.tabs.push(tab);
    this.activeId = id;
    this.layout();
    void wc.loadURL(url);
    this.notify();
    return this.info(tab);
  }

  closeTab(id: number) {
    const i = this.tabs.findIndex((t) => t.id === id);
    if (i < 0) return;
    const [t] = this.tabs.splice(i, 1);
    this.window.contentView.removeChildView(t!.view);
    t!.view.webContents.close();
    if (this.activeId === id) this.activeId = this.tabs[Math.min(i, this.tabs.length - 1)]?.id ?? null;
    this.layout();
    this.notify();
  }

  selectTab(id: number) {
    if (!this.tabs.some((t) => t.id === id)) return;
    this.activeId = id;
    this.layout();
    this.notify();
  }

  findTab(pred: (kind: TabKind, url: string) => boolean): Tab | undefined {
    return this.tabs.find((t) => pred(t.kind, t.view.webContents.getURL()));
  }

  get active(): Tab | undefined {
    return this.tabs.find((t) => t.id === this.activeId);
  }

  navigate(url: string) {
    const t = this.active;
    if (!t) {
      this.newTab(url, "web");
      return;
    }
    if (!this.opts.allowNavigation(t.kind, url)) {
      // 種類の合わない URL は新しい web タブで
      if (this.opts.allowNavigation("web", url)) this.newTab(url, "web");
      return;
    }
    void t.view.webContents.loadURL(url);
  }
  back() {
    const nh = this.active?.view.webContents.navigationHistory;
    if (nh?.canGoBack()) nh.goBack();
  }
  forward() {
    const nh = this.active?.view.webContents.navigationHistory;
    if (nh?.canGoForward()) nh.goForward();
  }
  reload() {
    this.active?.view.webContents.reload();
  }

  /** すべてのタブの WebContents (ブリッジ IPC の送信元照合に使う) */
  contentsOf(id: number): WebContents | undefined {
    return this.tabs.find((t) => t.view.webContents.id === id)?.view.webContents;
  }
  tabOfContents(wcId: number): Tab | undefined {
    return this.tabs.find((t) => t.view.webContents.id === wcId);
  }
  sendToKind(kind: TabKind, channel: string, ...args: unknown[]) {
    for (const t of this.tabs) if (t.kind === kind) t.view.webContents.send(channel, ...args);
  }

  private info(t: Tab): TabInfo {
    const wc = t.view.webContents;
    const nh = wc.navigationHistory;
    return {
      id: t.id,
      kind: t.kind,
      title: wc.getTitle() || wc.getURL(),
      url: wc.getURL(),
      loading: wc.isLoading(),
      canGoBack: nh.canGoBack(),
      canGoForward: nh.canGoForward(),
      active: t.id === this.activeId,
    };
  }

  notify() {
    this.opts.onTabsChanged(this.tabs.map((t) => this.info(t)));
  }
}
