/**
 * app://ui/ (シミュレータ UI バンドル) 用 preload。エンジンの接続先 (port / token) と再起動要求だけを渡す。
 * UI はこの情報でエンジンの WS に直接つなぐ (main を経由しないので 50Hz でも軽い)。
 */
import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("ipemuEngine", {
  endpoint: () => ipcRenderer.invoke("simui:endpoint"),
  restartEngine: () => ipcRenderer.invoke("simui:restart-engine"),
  onEndpointChanged: (cb: (ep: { port: number; token: string }) => void) => {
    const h = (_e: unknown, ep: { port: number; token: string }) => cb(ep);
    ipcRenderer.on("simui:endpoint-changed", h);
    return () => ipcRenderer.removeListener("simui:endpoint-changed", h);
  },
});
