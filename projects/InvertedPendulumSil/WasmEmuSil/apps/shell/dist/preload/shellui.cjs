"use strict";

// src/preload/shellui.ts
var import_electron = require("electron");
var on = (channel, cb) => {
  const h = (_e, ...args) => cb(...args);
  import_electron.ipcRenderer.on(channel, h);
  return () => import_electron.ipcRenderer.removeListener(channel, h);
};
import_electron.contextBridge.exposeInMainWorld("shell", {
  // タブ / ナビゲーション (chrome.html)
  navigate: (url) => import_electron.ipcRenderer.invoke("shell:navigate", url),
  back: () => import_electron.ipcRenderer.invoke("shell:back"),
  forward: () => import_electron.ipcRenderer.invoke("shell:forward"),
  reload: () => import_electron.ipcRenderer.invoke("shell:reload"),
  newTab: (url) => import_electron.ipcRenderer.invoke("shell:new-tab", url),
  closeTab: (id) => import_electron.ipcRenderer.invoke("shell:close-tab", id),
  selectTab: (id) => import_electron.ipcRenderer.invoke("shell:select-tab", id),
  onTabs: (cb) => on("shell:tabs", cb),
  onToast: (cb) => on("shell:toast", cb),
  // ホーム / 設定 (home.html)
  getSettings: () => import_electron.ipcRenderer.invoke("shell:get-settings"),
  setSettings: (patch) => import_electron.ipcRenderer.invoke("shell:set-settings", patch),
  catalogLocal: () => import_electron.ipcRenderer.invoke("shell:catalog-local"),
  launchLocal: (manifestPath) => import_electron.ipcRenderer.invoke("shell:launch-local", manifestPath),
  engineStatus: () => import_electron.ipcRenderer.invoke("ipemu:status"),
  openSimulator: () => import_electron.ipcRenderer.invoke("ipemu:open-simulator"),
  // ダイアログ (confirm.html / consent.html)
  onDialogData: (cb) => on("dialog:data", cb),
  answer: (answer) => import_electron.ipcRenderer.invoke("dialog:answer", answer)
});
//# sourceMappingURL=shellui.cjs.map
