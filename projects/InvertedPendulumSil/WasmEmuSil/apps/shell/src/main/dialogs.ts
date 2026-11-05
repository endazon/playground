/**
 * 確認ダイアログ (ipemu:// 起動) と同意ダイアログ (consent.required)。
 * OS ネイティブの dialog ではなく専用ウィンドウ (app://shell/*.html) にしているのは、
 * 外部から届いた文字列で見た目を偽装されにくくするため (07-client-app.md)。
 */
import { BrowserWindow, ipcMain, type WebContents } from "electron";

export interface DialogOptions {
  page: "confirm" | "consent";
  data: unknown;
  preload: string;
  width?: number;
  height?: number;
  parent?: BrowserWindow;
}

const pending = new Map<number, (answer: unknown) => void>();

let registered = false;
function registerOnce() {
  if (registered) return;
  registered = true;
  ipcMain.handle("dialog:answer", (event, answer: unknown) => {
    const url = event.senderFrame?.url ?? "";
    if (!url.startsWith("app://shell/")) return false; // シェル自身のページ以外からの回答は捨てる
    const resolve = pending.get(event.sender.id);
    if (!resolve) return false;
    pending.delete(event.sender.id);
    resolve(answer);
    return true;
  });
}

/** ダイアログを開いて回答を待つ。閉じられたら null */
export function showDialog<T>(o: DialogOptions): Promise<T | null> {
  registerOnce();
  const win = new BrowserWindow({
    width: o.width ?? 520,
    height: o.height ?? 420,
    parent: o.parent,
    modal: !!o.parent,
    resizable: false,
    minimizable: false,
    maximizable: false,
    title: o.page === "confirm" ? "ipemu — 起動の確認" : "ipemu — アクセスの同意",
    webPreferences: {
      preload: o.preload,
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  win.setMenuBarVisibility(false);
  return new Promise<T | null>((resolve) => {
    let done = false;
    const finish = (v: T | null) => {
      if (done) return;
      done = true;
      pending.delete(win.webContents.id);
      resolve(v);
      if (!win.isDestroyed()) win.close();
    };
    pending.set(win.webContents.id, (a) => finish(a as T));
    win.on("closed", () => finish(null));
    win.webContents.on("did-finish-load", () => sendData(win.webContents, o.data));
    void win.loadURL(`app://shell/${o.page}.html`);
  });
}

function sendData(wc: WebContents, data: unknown) {
  if (!wc.isDestroyed()) wc.send("dialog:data", data);
}
