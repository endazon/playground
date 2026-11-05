/** preload/shellui.ts が注入する window.shell の型 */
interface ShellTab {
  id: number;
  kind: "web" | "sim" | "home";
  title: string;
  url: string;
  loading: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  active: boolean;
}
interface ShellSettingsView {
  trustedServers: string[];
  allowLocalhost: boolean;
  allowLocalLaunch: boolean;
  enginePath?: string;
  catalogDirs: string[];
  homeUrl?: string;
  registerProtocol: boolean;
  settingsPath: string;
}
interface CatalogEntryView {
  id: string;
  revision: number;
  name: string;
  description?: string;
  source: { kind: "local"; manifestPath: string } | { kind: "server"; server: string; manifestId: string };
}
interface ShellApi {
  navigate(url: string): Promise<void>;
  back(): Promise<void>;
  forward(): Promise<void>;
  reload(): Promise<void>;
  newTab(url?: string): Promise<void>;
  closeTab(id: number): Promise<void>;
  selectTab(id: number): Promise<void>;
  onTabs(cb: (tabs: ShellTab[]) => void): () => void;
  onToast(cb: (m: { level: string; text: string }) => void): () => void;
  getSettings(): Promise<ShellSettingsView>;
  setSettings(patch: Partial<ShellSettingsView>): Promise<ShellSettingsView>;
  catalogLocal(): Promise<{ entries: CatalogEntryView[] }>;
  launchLocal(manifestPath: string): Promise<{ ok: boolean; error?: string }>;
  engineStatus(): Promise<{ engine: { running: boolean; port?: number; pid?: number; version?: string }; session: { id: string; revision: number; name: string; run: { state: string; tNs: number } } | null; uiReady: boolean }>;
  openSimulator(): Promise<void>;
  onDialogData(cb: (data: unknown) => void): () => void;
  answer(a: unknown): Promise<boolean>;
}
interface Window {
  shell: ShellApi;
}
