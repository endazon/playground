/**
 * 任意サイトのタブ用 preload。**信頼 origin のフレームにだけ** `window.ipemu` を注入する (07-client-app.md)。
 * 信頼判定は main に同期で問い合わせる (main は event.senderFrame の origin で判定し、renderer の申告は信用しない)。
 * 信頼されない origin には何も注入せず、普通のブラウザとして振る舞う。
 */
import { contextBridge, ipcRenderer } from "electron";

const allowed = ipcRenderer.sendSync("ipemu:bridge-allowed") === true;
if (allowed) {
  contextBridge.exposeInMainWorld("ipemu", {
    /** 構成の起動。サーバ経由 (M3) またはローカル manifest (開発) */
    launch: (req: unknown) => ipcRenderer.invoke("ipemu:launch", req),
    /** エンジンとセッションの状態 */
    status: () => ipcRenderer.invoke("ipemu:status"),
    /** シミュレータ UI のタブを開く / 前面にする */
    openSimulator: () => ipcRenderer.invoke("ipemu:open-simulator"),
    version: 1,
  });
}
