/**
 * シェル自身のページ (app://shell/*: chrome / home / confirm / consent) 用 preload。
 * タブ操作・設定・ローカル起動・ダイアログの回答を main に渡す。app://shell 以外には読み込まれない。
 */
import { contextBridge, ipcRenderer } from "electron";

type Handler = (...args: unknown[]) => void;
const on = (channel: string, cb: Handler) => {
  const h = (_e: unknown, ...args: unknown[]) => cb(...args);
  ipcRenderer.on(channel, h);
  return () => ipcRenderer.removeListener(channel, h);
};

contextBridge.exposeInMainWorld("shell", {
  // タブ / ナビゲーション (chrome.html)
  navigate: (url: string) => ipcRenderer.invoke("shell:navigate", url),
  back: () => ipcRenderer.invoke("shell:back"),
  forward: () => ipcRenderer.invoke("shell:forward"),
  reload: () => ipcRenderer.invoke("shell:reload"),
  newTab: (url?: string) => ipcRenderer.invoke("shell:new-tab", url),
  closeTab: (id: number) => ipcRenderer.invoke("shell:close-tab", id),
  selectTab: (id: number) => ipcRenderer.invoke("shell:select-tab", id),
  onTabs: (cb: (tabs: unknown) => void) => on("shell:tabs", cb as Handler),
  onToast: (cb: (msg: { level: string; text: string }) => void) => on("shell:toast", cb as Handler),
  // ホーム / 設定 (home.html)
  getSettings: () => ipcRenderer.invoke("shell:get-settings"),
  setSettings: (patch: unknown) => ipcRenderer.invoke("shell:set-settings", patch),
  catalogLocal: () => ipcRenderer.invoke("shell:catalog-local"),
  launchLocal: (manifestPath: string) => ipcRenderer.invoke("shell:launch-local", manifestPath),
  engineStatus: () => ipcRenderer.invoke("ipemu:status"),
  openSimulator: () => ipcRenderer.invoke("ipemu:open-simulator"),
  // ダイアログ (confirm.html / consent.html)
  onDialogData: (cb: (data: unknown) => void) => on("dialog:data", cb as Handler),
  answer: (answer: unknown) => ipcRenderer.invoke("dialog:answer", answer),
});
