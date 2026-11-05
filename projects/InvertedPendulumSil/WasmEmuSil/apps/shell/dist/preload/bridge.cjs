"use strict";

// src/preload/bridge.ts
var import_electron = require("electron");
var allowed = import_electron.ipcRenderer.sendSync("ipemu:bridge-allowed") === true;
if (allowed) {
  import_electron.contextBridge.exposeInMainWorld("ipemu", {
    /** 構成の起動。サーバ経由 (M3) またはローカル manifest (開発) */
    launch: (req) => import_electron.ipcRenderer.invoke("ipemu:launch", req),
    /** エンジンとセッションの状態 */
    status: () => import_electron.ipcRenderer.invoke("ipemu:status"),
    /** シミュレータ UI のタブを開く / 前面にする */
    openSimulator: () => import_electron.ipcRenderer.invoke("ipemu:open-simulator"),
    version: 1
  });
}
//# sourceMappingURL=bridge.cjs.map
