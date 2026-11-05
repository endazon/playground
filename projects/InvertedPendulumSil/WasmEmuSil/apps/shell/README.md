# apps/shell — Electron シェル

設計は [`Docs/07-client-app.md`](../../Docs/07-client-app.md)。汎用ブラウザ (タブ / アドレスバー / 戻る進む) に、
信頼 origin 限定のブリッジ、`ipemu://` ハンドラ、エンジンの起動・監視、`app://` プロトコル、同意ダイアログを足したもの。

```text
src/main/      Electron main (Node)
  index.ts           アプリの寿命、app:// の登録、ipemu:// の受信 → 確認ダイアログ、ブリッジ IPC の送信元検査
  browser.ts         BaseWindow + WebContentsView のタブ。タブの種類 (web / sim / home) で preload と遷移範囲を固定
  session.ts         エンジンの spawn → 接続 → session.launch、restartRequired での再起動、UI バンドル (tar.gz) の展開
  engine.ts          ipemu-engine プロセス (serve --exit-on-stdin-close)。stdout 1 行目の {port, token} を読む
  app-protocol.ts    app://ui (展開した UI バンドル) と app://shell (自身のページ) の配信と CSP
  untar.ts           tar.gz の最小展開器 (絶対パス / .. / リンクを拒否)
  trusted-origins.ts 信頼 origin の厳密比較 (https 必須、開発時のみ http://localhost)
  ipemu-url.ts       ipemu://launch?... の解析
  dialogs.ts         確認 / 同意の専用ウィンドウ (OS の dialog ではない)
  settings.ts        <userData>/settings.json
src/preload/   contextBridge (sandbox: true なので CommonJS)
  bridge.ts          任意サイト用。信頼 origin のフレームにだけ window.ipemu {launch, status, openSimulator}
  simui.ts           app://ui 用。window.ipemuEngine {endpoint, restartEngine, onEndpointChanged}
  shellui.ts         app://shell 用。window.shell (タブ操作 / 設定 / ローカル起動 / ダイアログの回答)
src/renderer/  シェル自身のページ (vite): chrome.html (タブ / アドレスバー), home.html, confirm.html, consent.html
```

## 動かす
```bash
cd projects/InvertedPendulumSil/WasmEmuSil
cargo build -p ipemu-engine                      # シェルは target/{release,debug}/ipemu-engine を探す (IPEMU_ENGINE で上書き可)
pnpm install                                     # Electron 本体が取れない環境: ELECTRON_MIRROR=https://npmmirror.com/mirrors/electron/ pnpm install
pnpm --filter @ipemu/sim-ui build                # build/ui/ui-cartpole.tar.gz (dev manifest の ui.bundle)
pnpm --filter @ipemu/shell start                 # ビルドして electron を起動 (root なら --no-sandbox-if-root が効く)
```
ホーム (`app://shell/home.html`) のローカルカタログから「組み立てる」を押すと、エンジンを spawn して
`dev/cartpole.manifest.json` を組み立て、UI バンドルを `<userData>/ui-cache/<sha256>/` に展開して `app://ui/` のタブで開く。

- 設定: `<userData>/settings.json` (`trustedServers`, `allowLocalhost`, `allowLocalLaunch`, `enginePath`, `catalogDirs`, `homeUrl`, `registerProtocol`)
- `ipemu://launch?server=…&manifest=…&rev=…&ticket=…` は必ず確認ダイアログを出す。信頼していないサーバは「起動する」が押せない。
  開発用に `ipemu://launch?manifestPath=/abs/path.manifest.json` も受ける (`allowLocalLaunch`)。
  OS への登録は packaged のときだけ (`electron .` では実行ファイルが electron 自身のため)
- テスト: `pnpm --filter @ipemu/shell test` (origin 判定、URL 解析、tar 展開、app:// の経路、設定)。
  Electron を実際に動かす E2E は Playwright の `_electron` で手元で行う (Xvfb 可)。スクリーンショットは `Docs/images/m2-shell*.png`
