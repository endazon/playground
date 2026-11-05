# 02. アーキテクチャ

## 全体像
![構成図](images/architecture.svg)

2 つのアプリと、その間を流れる 3 種類の物 (構成情報・アーティファクト・テレメトリ) で構成します。
クライアントは **UI シェル (Electron)** と **エンジン (Rust + Wasmtime)** の 2 プロセスです。

```text
┌─ サーバアプリ ipemu-server (Rust / axum) ─────────────────────────────────────┐
│  配信 API (/api/v1)                        管理 SPA (/admin, Vite + React + TS)   │
│   POST clients/hello   → 割当 manifest      部品の登録 (wasm アップロード + WIT 検査) │
│   GET  manifests/{id}  → 署名付き JSON      manifest の編集 / 検証 / 承認 / 公開    │
│   GET  artifacts/{name}/{ver} → .wasm       capability の承認、クライアント割当、監査 │
│   POST telemetry        ← 実行要約 (任意)                                         │
│  保存: アーティファクト = 内容アドレス (sha256) のファイル置き場                     │
│        メタデータ = SQLite (sqlx): artifacts / manifests / clients / consents / audit│
│  鍵:   manifest 署名用 Ed25519 秘密鍵 (サーバのみ)。公開鍵はクライアントに同梱        │
│  検証: ipemu-core (クライアントと同一クレート) で manifest と wasm を検査             │
└───────────────────────────────────────────────────────────────────────────────┘
                 │ HTTPS (クライアント API キー)。起動時と手動更新時だけ通信する
┌────────────────┴──────────────────────────────────────────────────────────────┐
│─ クライアント (Windows / macOS / Linux) ───────────────────────────────────────│
│                                                                               │
│  ┌ UI シェル: Electron ─────────────────┐   ┌ エンジン: ipemu-engine (Rust) ──────┐ │
│  │ main:  エンジンを子プロセスで起動       │   │ bootstrap  hello → manifest → 署名   │ │
│  │        app:// で UI バンドルを配信       │   │            → artifacts → sha256 照合  │ │
│  │        同意ダイアログの表示              │◀─▶│ core       配線グラフ / 実行順 / 検査  │ │
│  │ renderer: 2D / 3D / スコープ / 操作     │IPC│ scheduler  仮想時間・固定ステップ      │ │
│  │        (sim-view.js の移植)             │   │ wasm       Wasmtime: 部品と FW の実体化│ │
│  └──────────────────────────────────────┘   │            WASI p2 を capability で付与│ │
│    IPC: ループバック WebSocket + トークン       │ firmware   wasm | native (.dll/.so)   │ │
│         JSON-RPC (操作) / MessagePack (50Hz)   │ telemetry  実行要約を送信 (任意)      │ │
│                                              └──────────────────────────────────┘ │
│  同じ IPC で ipemu CLI (headless) / テストもエンジンに接続できる                      │
│  デバッガ (Visual Studio / VS Code + LLDB) はエンジンプロセスにアタッチする            │
└───────────────────────────────────────────────────────────────────────────────┘
```

## なぜ 2 プロセスか
- **シェルを差し替えられる。** Chromium を同梱する手段は Electron のほかに Electrobun (CEF)、Qt WebEngine、cef-rs があり、
  成熟度が揃っていません ([08-decisions.md](08-decisions.md))。IPC 契約を固定しておけばシェルだけ入れ替えられます。
- **エンジン単体で動く。** headless の CI 実行、`ipemu` CLI、デバッガのアタッチがシェル抜きでできます。
- **障害が波及しない。** 描画が重くてもエンジンの積分周期は乱れず、エンジンが panic してもシェルは生きていて再起動できます。

## 起動シーケンス
```text
Shell (Electron main)         Engine (ipemu-engine)                 Server
 │ 1. エンジンを spawn ─────────▶│
 │ ◀── stdout 1 行目: {port, token}│
 │ 2. WS 接続 (token) ─────────▶│
 │                               │ 3. settings (serverUrl, apiKey, cacheDir) を読む
 │                               │ 4. POST /api/v1/clients/hello ───────────▶│ クライアント ID, エンジン版,
 │                               │ ◀── { manifestId, revision, flags } ──────│ 対応 WIT 版を登録。割当を返す
 │                               │ 5. GET /api/v1/manifests/{id}?rev=n ─────▶│
 │                               │ ◀── manifest JSON + 署名 ──────────────────│
 │                               │ 6. 署名を検証 (同梱の公開鍵)
 │                               │ 7. artifacts を走査。キャッシュに sha256 がなければ
 │                               │    GET /api/v1/artifacts/{name}/{ver} ───▶│
 │                               │ ◀── .wasm (ETag = sha256, immutable) ─────│
 │                               │ 8. sha256 照合 → キャッシュ (cache/sha256/<hex>)
 │                               │ 9. 各 wasm をコンパイル。WIT ワールドを manifest と照合
 │                               │10. 配線グラフ・実行順 (閉路検査)、capability 判定
 │ ◀── event: consent-required ──│11. 未同意の capability があればシェルに問い合わせ
 │ 12. 同意ダイアログ → 回答 ────▶│    回答はローカルの consent 保存へ
 │                               │13. 部品を実体化、ファームウェアを slot に装着
 │ ◀── event: ready {manifest} ──│    (manifest 既定 or CLI 指定の wasm / native)
 │ 14. app://ui/ を読み込み表示    │    (UI バンドルもアーティファクト。キャッシュから配信)
 │ 15. run ────────────────────▶│15. Scheduler 開始。50Hz でスナップショットを送る
```
4〜8 のいずれかで失敗した場合、キャッシュに**前回起動時の manifest 一式が揃っていればそれで起動**し、
UI に「オフライン / 旧構成」を表示します。揃っていなければ起動せずエラーを出します。
同意 (11〜12) は**実体化の前**にまとめて行い、ステップの途中で問い合わせは発生させません。

## プロセスとスレッド
| 実行主体 | 役割 | 備考 |
| --- | --- | --- |
| Shell main (Node) | エンジンの起動・監視、`app://` プロトコル、同意ダイアログ、ウィンドウ | 制御には関与しない |
| Shell renderer (Chromium) | 描画と操作。rAF で描く | 状態は IPC で受け取るだけ |
| Engine scheduler スレッド | 仮想時間を進め、部品とファームウェアを順に呼ぶ | **単一スレッド**。Wasmtime の `Store` はこのスレッドだけが触る |
| Engine I/O (tokio) | IPC サーバ、HTTP 取得、キャッシュ書き込み、テレメトリ | scheduler とは `crossbeam` チャネルで疎結合 |

scheduler → I/O はトリプルバッファでスナップショットを渡し、I/O → scheduler の操作 (開始 / 停止 / パラメータ変更 /
ファームウェア差し替え) はチャネルに積んでステップ境界で取り出します。NetworkedSil の「Hub は Channel に積むだけ」と同じ理由です。

## 仮想時間とペーシング
```text
baseStep (例 1ms) ごとに:
  1. 操作キューを処理 (ステップ境界でのみ状態を変える)
  2. 実行順に従って各部品を step。rateDivider で間引く
       例: plant 1ms, encoder 1ms, cpu-virtual 1ms (tickHz=1000), comm-link 1ms, snapshot 20ms
  3. 配線表に従って出力 → 入力を転記 (latency=1 の配線はここ、latency=0 は 2 の中で順次)
  4. 仮想時刻 += baseStep
  5. モードが realtime なら、壁時計が追いつくまで待つ
       遅れが maxSlip を超えたら追いつかず「時間スリップ」として数える (NetworkedSil と同じ)
       デバッガ接続中 (--debug-friendly / 自動検出) は追いつきを一切行わない
     モードが fast なら待たない / step なら 1 ステップごとに停止
```
部品から見える時刻は `ip:emu/host.now-ns` (仮想時刻) だけです。
WASI の `wall-clock` / `monotonic-clock` は capability として宣言・承認された場合にのみ与えます
(与えた場合、その部品は決定性の保証から外れることを manifest 検証が警告します)。

## 配置と配布
| 物 | 配布経路 | 更新の単位 |
| --- | --- | --- |
| サーバアプリ | Docker (`compose.yaml`、TLS 終端はリバースプロキシ)。NetworkedSil と同じ流儀 | デプロイ |
| クライアント本体 (シェル + エンジン) | OS ごとのインストーラ (electron-builder)。エンジンは同梱 | インストーラ |
| 部品 WASM / ファームウェア WASM / UI バンドル | サーバのアーティファクト API | manifest の公開 |
| ファームウェアのネイティブ形 (開発用) | ローカルビルド成果物を CLI / 設定で指定 | ビルド |

クライアント本体の更新を減らすため、**UI (HTML/JS) もアーティファクトとして配信**し、
シェルは `app://ui/` プロトコルでキャッシュ内の UI バンドルを読み込みます。
