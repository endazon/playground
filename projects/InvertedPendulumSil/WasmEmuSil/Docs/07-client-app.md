# 07. クライアントアプリ (Electron シェル + Rust エンジン)

## 2 つのプロセス
| プロセス | 技術 | 責務 |
| --- | --- | --- |
| シェル `apps/shell` | Electron (main: Node, renderer: Chromium) | ウィンドウ、UI の描画、同意ダイアログ、エンジンの起動と監視、`app://` プロトコル |
| エンジン `ipemu-engine` | Rust + Wasmtime + tokio | サーバとの通信、manifest 検証、組み立て、スケジューラ、capability の実施、IPC サーバ |

シェルは**状態を持ちません**。エンジンから受けたスナップショットを描き、操作をエンジンへ送るだけです。
これにより `ipemu-engine --headless` も、テストからの直接接続も、同じエンジンで動きます。

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
| `session.describe` | 現在の manifest、部品一覧、ポート、capability と同意状態、上書きの有無 |
| `run.start` / `run.stop` / `run.reset` / `run.step {n}` | 実行制御。`reset` は Epoch を進める |
| `run.setPacing {realtime\|fast\|step, speed}` | ペーシング変更 |
| `param.set {instance, key, value}` | 部品パラメータの変更 (部品が `init` で再作成されることを UI に明示) |
| `firmware.load {slot, source}` | ファームウェアの差し替え。`source` は `artifact:` / `wasm:` / `native:` |
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
| `bootstrap.progress` | hello / manifest / artifact 取得の進捗、オフライン起動の通知 |

契約は `ipemu-ipc` に型として置き、TypeScript の型定義を `cargo` のビルドで生成します (`ts-rs` 等)。
シェルを Electron から別物に替えても、この契約だけを満たせばよい構造です。

## シェル (Electron)
```text
apps/shell/
  main/      エンジン spawn (同梱バイナリ。開発時は PATH / 環境変数で差し替え)、クラッシュ時の再起動と通知
             app:// プロトコル: エンジンのキャッシュにある UI バンドル (manifest の ui.bundle) を配信
             同意ダイアログ (OS ネイティブの dialog ではなく renderer の専用ウィンドウ。内容を偽装されにくくする)
  preload/   contextBridge で renderer に最小限の API だけ晒す (IPC 接続・ウィンドウ操作)
  renderer/  React + ui-kit。sim-view (Canvas 2D / three.js) は rAF で描画。状態は IPC の state 通知を直接反映
```
- `nodeIntegration: false`, `contextIsolation: true`, `sandbox: true`。renderer から Node は触れません。
- CSP は `app://` と `ws://127.0.0.1` のみ許可。外部ネットワークに renderer から出ません。
- UI バンドルはアーティファクトなので、manifest で**部品に合わせた UI を配れます** (`ui.views`)。
  UI バンドルの JS もエンジンのキャッシュ経由で sha256 検証済みです。

## エンジン (`crates/ipemu-engine`)
```text
ipemu-engine
  ├ settings   : <config dir>/ipemu/settings.toml  (serverUrl, apiKey, cacheDir, telemetry)
  ├ bootstrap  : hello → manifest → 署名 → artifacts → キャッシュ (reqwest, rustls)
  ├ core       : ipemu-core (manifest 検証、配線グラフ、実行順、capability 判定)
  ├ wasm       : ipemu-wasm (Wasmtime Engine 1 つ、部品ごとに Store、WIT バインディング、WASI 付与)
  ├ firmware   : wasm ローダ / native ローダ (libloading)。cpu-virtual から呼ばれる
  ├ scheduler  : 専用スレッド。03-component-model.md のループ
  ├ ipc        : tokio + tungstenite。トークン認証。1 シェル + 複数の観測専用接続を許す
  └ telemetry  : 実行要約をバッチ送信 (settings で無効化可能)
```

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

## キャッシュとオフライン
```text
<cache dir>/ipemu/
  sha256/ab/cd/<hex>            アーティファクト本体 (イミュータブル)
  manifests/<id>/<revision>.json 署名付き manifest
  last-good.json                 直近に正常起動した (id, revision)
  wasmtime/                      コンパイルキャッシュ
```
- サーバに届かないときは `last-good` の manifest と、その参照するアーティファクトが揃っていれば起動します。
- GC は「`last-good` と現在割当の revision から参照されるもの」以外を、サイズ上限超過時に LRU で削除します。

## 開発者向け CLI
```bash
ipemu-engine --headless --manifest dev/cartpole.manifest.json --no-signature-check \
             --firmware balance=wasm:target/wasm32-wasip2/release/fw_balance.wasm \
             --run-seconds 10 --record out.csv --pacing fast
ipemu-engine validate dev/cartpole.manifest.json        # サーバと同じ検証をローカルで
ipemu-engine inspect components/plant-cartpole.wasm     # WIT / describe の表示
ipemu attach --port 54321                               # 動作中のエンジンの state を端末に流す
```
