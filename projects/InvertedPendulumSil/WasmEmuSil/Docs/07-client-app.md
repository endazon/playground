# 07. クライアントアプリ (Electron シェル + Rust エンジン)

## 2 つのプロセス
| プロセス | 技術 | 責務 |
| --- | --- | --- |
| シェル `apps/shell` | Electron (main: Node, renderer: Chromium) | **汎用ブラウザ** (タブ・アドレスバー・履歴)。信頼 origin へのブリッジ注入、`ipemu://` ハンドラ、シミュレータ UI の表示、同意ダイアログ、エンジンの起動と監視、`app://` プロトコル |
| エンジン `ipemu-engine` | Rust + Wasmtime + tokio | サーバとの通信、manifest 検証、組み立て、スケジューラ、capability の実施、IPC サーバ |

シェルはシミュレータの**状態を持ちません**。エンジンから受けたスナップショットを描き、操作をエンジンへ送るだけです。
これにより `ipemu-engine --headless` も、テストからの直接接続も、同じエンジンで動きます。
v1 で同時に動くセッションは 1 つで、別の構成を起動すると現在のセッションを停止して切り替えます
(複数化はエンジンを複数 spawn する拡張で、IPC 契約は変えません)。

## IPC 契約 (`crates/ipemu-ipc`)
```text
transport : ループバック WebSocket (127.0.0.1:<ephemeral>)。接続時にトークンを要求
discovery : エンジンが起動直後に stdout の 1 行目へ {"port": 54321, "token": "…"} を出す
            シェルが spawn したときは環境変数 IPEMU_IPC_TOKEN でも渡す
text frame: JSON-RPC 2.0  (要求 / 応答 / 通知)
bin  frame: MessagePack   (高頻度ストリーム。先頭 1 バイトでチャネル種別)
```

### 要求 (シェル / CLI → エンジン)
| メソッド | 内容 |
| --- | --- |
| `session.launch {server, manifestId, revision, credential}` | 構成の取得・検証・組み立て。実行中のセッションがあれば停止して置き換える |
| `session.describe` | 現在の manifest、部品一覧、ポート、capability と同意状態、上書きの有無、資格情報の種別 (利用者 / API キー) |
| `catalog.local` | キャッシュ済みの構成一覧 (ポータルに届かないときのローカルカタログ) |
| `run.start` / `run.stop` / `run.reset` / `run.step {n}` | 実行制御。`reset` は Epoch を進める |
| `run.setPacing {realtime\|fast\|step, speed}` | ペーシング変更 |
| `param.set {instance, key, value}` | 部品パラメータの変更 (部品が `init` で再作成されることを UI に明示) |
| `firmware.load {slot, source}` | ファームウェアの差し替え。`source` は `artifact:` / `wasm:` / `native:`。wasm はホット (状態保持)、native はエンジン再起動 (応答に `restartRequired: true`) |
| `firmware.watch {slot, path}` | ビルド成果物の更新を監視して自動で `firmware.load` |
| `scope.subscribe {ports[], rateHz, window}` | スコープ用ストリームの購読 |
| `consent.answer {requestId, grant, remember}` | 同意ダイアログの回答 |
| `manifest.refresh` | サーバへ再問い合わせ (手動更新) |

### 通知 (エンジン → シェル)
| 通知 | 内容 |
| --- | --- |
| `state` (bin, 50Hz 既定) | 仮想時刻、tick、ペーシング、`builtin:snapshot` に載せたポート値、実行時間 / オーバーラン、スリップ |
| `scope` (bin) | 購読したポートの高レート値 (リングバッファのチャンク) |
| `event` | 状態遷移、FAULT 理由、保護動作、ログ (レベル付き)、capability 使用 |
| `consent.required` | 未同意の capability。`{requestId, instance, artifactSha256, capability, reason}` |
| `bootstrap.progress` | manifest / artifact 取得の進捗、キャッシュ構成でのオフライン起動の通知 |

契約は `ipemu-ipc` に型として置き、TypeScript の型定義を `cargo` のビルドで生成します (`ts-rs` 等)。
シェルを Electron から別物に替えても、この契約だけを満たせばよい構造です。

## シェル (Electron)
```text
apps/shell/
  main/      ブラウザ UI: タブ / アドレスバー / 戻る進む / 履歴 / ダウンロード (任意のサイトを開ける)
             信頼 origin の判定: settings の trustedServers と、ページの origin を厳密比較
             ipemu:// プロトコルハンドラの登録 (初回起動) と受信 → 確認ダイアログ → session.launch
             エンジン spawn (同梱バイナリ。開発時は PATH / 環境変数で差し替え)、クラッシュ時の再起動と通知
             app:// プロトコル: エンジンのキャッシュにある UI バンドル (manifest の ui.bundle) を配信
             同意ダイアログ (OS ネイティブの dialog ではなく専用ウィンドウ。内容を偽装されにくくする)
  preload/   contextBridge。信頼 origin のページにだけ window.ipemu (launch / status / openSimulator) を注入する。
             それ以外の origin には何も注入しない (通常のブラウザとして振る舞う)
  renderer/  シミュレータ UI は配信された UI バンドル (app://)。React + ui-kit。sim-view は rAF で描画
```
- 全ページで `nodeIntegration: false`, `contextIsolation: true`, `sandbox: true`。renderer から Node は触れません。
- ブリッジ `window.ipemu` は **origin が `trustedServers` に一致するフレームにだけ**注入します (iframe・リダイレクト後も再判定)。
  信頼 origin は HTTPS 必須 (開発時のみ `http://localhost` を許す)。
- `app://` (シミュレータ UI) の CSP は `app://` と `ws://127.0.0.1` のみ許可。任意サイトのタブは通常のブラウザと同じ制約で、
  エンジンの IPC にも `app://` にも到達できません。
- `ipemu://launch?...` は外部 (Chrome / Edge / メール) からも届くため、**必ず確認ダイアログ**を出してから進みます。
  表示するのはサーバ origin・構成名・revision・要求 capability。信頼していないサーバからの起動は拒否します。
- UI バンドルはアーティファクトなので、manifest で**部品に合わせた UI を配れます** (`ui.views`)。
  UI バンドルの JS もエンジンのキャッシュ経由で sha256 検証済みです。

## エンジン (`crates/ipemu-engine`)
```text
ipemu-engine
  ├ settings   : <config dir>/ipemu/settings.toml  (trustedServers, apiKey (headless 用), cacheDir, telemetry = true)
  ├ bootstrap  : session.launch で受けた資格情報で manifest → 署名 → artifacts → キャッシュ (reqwest, rustls)
  │              資格情報は利用者のセッショントークン (シェル経由) / 単回チケットの交換 / API キーの 3 通り
  ├ firmware   : wasm はホット差替 (Store を入れ替え)、native は restartRequired を返してシェルに再起動させる
  ├ core       : ipemu-core (manifest 検証、配線グラフ、実行順、capability 判定)
  ├ wasm       : ipemu-wasm (Wasmtime Engine 1 つ、部品ごとに Store、WIT バインディング、WASI 付与)
  ├ firmware   : wasm ローダ / native ローダ (libloading)。cpu-virtual から呼ばれる
  ├ scheduler  : 専用スレッド。03-component-model.md のループ
  ├ ipc        : tokio + tungstenite。トークン認証。1 シェル + 複数の観測専用接続を許す
  └ telemetry  : 実行要約をバッチ送信 (settings で無効化可能)
```

### 対象プラットフォーム
Windows / macOS / Linux × x64 / arm64 の 6 通りをビルドします。Wasmtime (Cranelift) と Electron はいずれも対応済みで、
ファームウェアの native 形はホストと同じアーキでビルドします (Apple Silicon では arm64 の `.dylib`)。
wasm 形はアーキに依存しないため、配信物は 1 種類です。

### Wasmtime の設定
| 設定 | 値 | 理由 |
| --- | --- | --- |
| `wasm_component_model` | on | WIT 境界 |
| `cranelift_nan_canonicalization` | on | 決定性 |
| `epoch_interruption` | on (全部品) | 暴走した `step` を trap させる |
| `consume_fuel` | ファームウェア slot のみ | 実行時間モデル |
| `debug_info` | `--wasm-debug-info` のときのみ | LLDB からのソースデバッグ。性能が落ちるので既定 off |
| `cache` | on (キャッシュ dir 配下) | コンパイル結果の再利用で起動を速く |
| `wasm_threads` / `shared_memory` | off | 決定性と単純さ |

### capability の実施
| 宣言 | 実施 |
| --- | --- |
| `net.tcpConnect` | `wasi:sockets` を link し、`WasiCtxBuilder::socket_addr_check` で宛先を許可リストと照合。名前解決も許可ホストに限定 |
| `net.tcpListen` | 同上 (bind 可能なアドレスのみ) |
| `fs` | `preopened_dir` でデータディレクトリ配下の指定パスだけを、`mode` に応じた権限で開く |
| `clock` / `random` | `wasi:clocks` / `wasi:random` を link する。未宣言なら link しない |
| `hostExt.*` | `ip:emu/host-ext/<name>` を個別に link。各インターフェースはエンジン側で OS 資源に橋渡しし、使用回数を記録 |

**宣言のないものは import 自体を link しない**ので、実体化の時点で `unresolved import` として失敗し、
「承認なしに呼ばれる」経路がありません。これが WASI preview 2 を選んだ理由です。

### 同意 (consent)
```text
consent store: <data dir>/ipemu/consent.json
  key   : sha256(artifact) + ":" + capability 文字列 (例 "net.tcpConnect:192.168.10.5:502")
  value : { granted: bool, at, manifestId, revision, rememberUntil? }
```
- 実体化の前に未同意分をまとめてシェルへ問い合わせ、回答が揃うまで実行しません。
- 「今回だけ」「記憶する」「拒否」の 3 択。拒否された部品は実体化せず、構成全体を FAULT ではなく**起動不可**として表示します
  (部分的に動いた結果を信じないため)。
- headless 実行では `--consent-file` で事前同意を渡すか、未同意があれば非ゼロ終了します。

## テレメトリ
既定で on です。送るのは実行要約 (開始 / 終了 / FAULT 理由 / オーバーラン数 / 時間スリップ / capability 使用回数 / エンジン版・OS・アーキ) だけで、
信号の生値・パラメータ・ファームウェアの内容は送りません。`settings.toml` の `telemetry = false` で止まり、
UI の「このセッション」画面に送信の有無と直近の送信内容を表示します。

## キャッシュとオフライン
```text
<cache dir>/ipemu/
  sha256/ab/cd/<hex>            アーティファクト本体 (イミュータブル)
  manifests/<server>/<id>/<revision>.json 署名付き manifest (複数の構成を保持)
  catalog.json                   キャッシュ済み構成の一覧 (ローカルカタログ) と最終起動日時
  wasmtime/                      コンパイルキャッシュ
```
- サーバに届かないときは、ローカルカタログから選んだ構成の manifest と参照アーティファクトが揃っていれば起動します。
- GC は「直近 N 回起動した構成から参照されるもの」以外を、サイズ上限超過時に LRU で削除します。

## 開発者向け CLI
```bash
ipemu-engine --headless --manifest dev/cartpole.manifest.json --no-signature-check \
             --firmware balance=wasm:target/wasm32-wasip2/release/fw_balance.wasm \
             --run-seconds 10 --record out.csv --pacing fast
ipemu-engine --headless --server https://ipemu.example --api-key "$IPEMU_API_KEY" \
             --launch cartpole-lqr-default@3 --run-seconds 60     # API キーでカタログの構成を起動
ipemu-engine --watch balance=wasm:target/wasm32-wasip2/debug/fw_balance.wasm   # 再ビルドで自動ホット差替
ipemu-engine validate dev/cartpole.manifest.json        # サーバと同じ検証をローカルで
ipemu-engine inspect components/plant-cartpole.wasm     # WIT / describe の表示
ipemu attach --port 54321                               # 動作中のエンジンの state を端末に流す
```
