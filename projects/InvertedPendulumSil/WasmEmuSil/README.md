# WasmEmuSil — WASM 部品で組む倒立振子エミュレータ (設計草案)

> **状態: M1 (エンジン核) と M5 (capability、エンジン側) 実装済み。** headless のエンジンが、ローカル manifest から Rust 製の部品 (wasm) と
> C 版 / Rust 版のファームウェア (wasm または native) を組み立て、スイングアップ → 倒立維持まで動きます。
> 部品が TCP / ファイルを使う場合は manifest の宣言 → 同意 (`--consent-file`) を経て WASI preview 2 として付与されます。
> サーバ配信・Electron シェル (M2〜M4) は別途進行中。設計は [`Docs/`](Docs/README.md)、進捗は [`Docs/08-decisions.md`](Docs/08-decisions.md)。

[`../NetworkedSil`](../NetworkedSil/README.md) と同じ台車型倒立振子を、
**クライアント側で組み立てるエミュレータ**として実現します。

- **サーバアプリ**: アクセスすると構成情報 (manifest) と部品 (WASM) をダウンロードできる配信サーバ + 管理画面
- **クライアントアプリ**: Chromium (UI シェル) + Wasmtime (エミュレータエンジン) のデスクトップアプリ。Windows / macOS / Linux、x64 / arm64。
  シェルは**普通のブラウザとして**サーバのポータルを開き、利用者がカタログから構成を選んで「起動」を押すと、
  エンジンが構成情報と部品 WASM を取り寄せてエミュレータ環境を組み立てる。Chrome / Edge から `ipemu://` リンクで起動することもできる
- **仮想 MCU**: エミュレータの中の CPU 部品。その中で走る**制御プログラム (ファームウェア) は外部から注入**する。
  配布形は WASM (全 OS 共通)。同じ C / Rust ソースをネイティブ共有ライブラリにもビルドでき、
  Windows では Visual Studio で F5 / アタッチしてブレークポイントを置ける
- **許可制の OS アクセス**: 部品やファームウェアが TCP・ファイル・OS 機能を使う場合は、
  manifest で宣言 → 管理者が承認・署名 → クライアントで利用者が初回に同意、の 3 段を通る

![構成図](Docs/images/architecture.svg)

## クイックスタート (M1: headless)
前提: Rust (stable、`wasm32-unknown-unknown` ターゲット)、`wasm-tools`、`wit-bindgen-cli`、clang。
```bash
cd projects/InvertedPendulumSil/WasmEmuSil
rustup target add wasm32-unknown-unknown
cargo install --locked wasm-tools wit-bindgen-cli

./scripts/build-components.sh          # 部品 4 種 → build/components/*.wasm
./scripts/build-firmware.sh            # C 版 / Rust 版 FW → build/firmware/*.wasm, *.so|dylib|dll

# 組み立てて 20 秒回す (既定: C 版 FW の wasm 形)。--firmware で差し替え
cargo run -q -p ipemu-engine -- run --manifest dev/cartpole.manifest.json --no-signature-check --run-seconds 20
cargo run -q -p ipemu-engine -- run --manifest dev/cartpole.manifest.json --no-signature-check \
    --firmware balance=native:build/firmware/fw-balance-c.so --record out.csv

cargo run -q -p ipemu-engine -- validate dev/cartpole.manifest.json --no-signature-check   # サーバと同じ検証
cargo run -q -p ipemu-engine -- inspect build/components/plant-cartpole.wasm                # describe() を表示
cargo run -q -p ipemu-engine -- lqr --length 0.6 --ts 0.005                                  # FW 用 LQR ゲイン

./scripts/check.sh                     # fmt / clippy / 全ビルド / 全テスト / manifest 検証 (PR の前に通す)
```

### capability と同意 (M5)
`dev/cartpole-tcp.manifest.json` は上の構成に、振子角を 100 ms ごとに TCP で送りファイルにも追記する部品 `tcp-reporter` を足したものです。
部品が使う `wasi:sockets` / `wasi:filesystem` は manifest の `capabilities` に宣言され (未宣言なら V7 で拒否)、
初回は利用者の同意が要ります。headless では同意ファイルで渡します。
```bash
nc -lk 127.0.0.1 9000 &                                                     # 受け側 (任意)
cargo run -q -p ipemu-engine -- consent list --manifest dev/cartpole-tcp.manifest.json --no-signature-check
#   → 同意が要る鍵 (sha256:capability) と --consent-file の雛形が出る。雛形を consent.json に保存して:
cargo run -q -p ipemu-engine -- run --manifest dev/cartpole-tcp.manifest.json --no-signature-check \
    --run-seconds 5 --consent-file consent.json --data-dir ./build/data
#   同意ファイルなしなら終了コード 3 (未同意)、拒否があれば 4。ファイルは ./build/data/ipemu/data/logs/theta.csv にできる
#   validate は同意なしで通るが、capability は deny-all で付与されるので I/O は一切起きない
cargo run -q -p ipemu-engine -- inspect build/components/tcp-reporter.wasm   # import 一覧と必要な宣言の目安
```
`dev/cartpole.manifest.json` は NetworkedSil の既定パラメータ (振子 0.6 m) で、0.5 s 後にスイングアップを自動開始し、
約 6 s で倒立領域に入って LQR に切り替わります。`--record` で 10 ms ごとのスナップショットを CSV に出せます。

## 何が変わるか (NetworkedSil との違い)
| 観点 | NetworkedSil | WasmEmuSil |
| --- | --- | --- |
| 制御ループの場所 | バックエンド。通信路がループの内側 | クライアント内の仮想 MCU。通信路はループの外側 |
| サーバの役割 | コントローラそのもの | 構成情報と部品の配信・管理 |
| 時間 | 実時間 (ブラウザの Stopwatch とサーバの時計) | **仮想時間**。固定ステップで決定的。実時間へのペーシングは任意 |
| 制御プログラム | C# (`ControllerCore`) がサーバに固定 | C / Rust ソースを WASM (配布) とネイティブ (デバッグ) にビルドして注入 |
| デバッグ | ブレークで止めるとウォッチドッグが鳴る | ブレークで止めると**仮想世界ごと止まる**。再開しても矛盾しない |
| 部品の差し替え | ソース変更と再デプロイ | 構成情報の編集と部品 WASM の登録。クライアント本体は更新不要 |
| 対象の広がり | 倒立振子専用 | クライアントは対象を知らない。部品・ファームウェア・UI・構成はすべて配信物で、別の機構も登録だけで動く |
| 起動の入口 | ブラウザで URL を開く | シェル (または通常のブラウザ) でサーバのポータルを開き、カタログから「起動」 |
| 実装言語 | C# (.NET 10) | エンジン / サーバ / 部品: Rust、UI: TypeScript、ファームウェア: C / C++ / Rust |

## ドキュメント
| ドキュメント | 内容 |
| --- | --- |
| [Docs/01-concept.md](Docs/01-concept.md) | 目的、設計原則、NetworkedSil からの位置づけ、スコープ外 |
| [Docs/02-architecture.md](Docs/02-architecture.md) | 全体構成 (構成図)、起動シーケンス、プロセス / スレッドモデル、仮想時間 |
| [Docs/03-component-model.md](Docs/03-component-model.md) | 部品のインターフェース (WIT)、ポートと配線、スケジューラ、決定性 |
| [Docs/04-manifest.md](Docs/04-manifest.md) | 構成情報 (manifest) の仕様、検証規則、版と署名、capability 宣言 |
| [Docs/05-firmware.md](Docs/05-firmware.md) | 仮想 MCU と HAL、WASM / ネイティブの 2 形、Visual Studio / LLDB でのデバッグ |
| [Docs/06-server-app.md](Docs/06-server-app.md) | 配信 API、管理画面、保存、認証・署名・承認フロー |
| [Docs/07-client-app.md](Docs/07-client-app.md) | Electron シェル、Rust エンジン、IPC 契約、capability の実施、キャッシュとオフライン |
| [Docs/08-decisions.md](Docs/08-decisions.md) | 技術選定の比較 (トレードオフ)、未決事項、マイルストーン |
| [Docs/09-verification.md](Docs/09-verification.md) | 検証方針: 性質テスト、ゴールデントレース、決定性、適合性テスト |
| [schemas/emulator-manifest.schema.json](schemas/emulator-manifest.schema.json) | 構成情報の JSON Schema と [サンプル](schemas/examples/cartpole-lqr.manifest.json) |
| [abi/wit/](abi/wit/) / [abi/ip_hal.h](abi/ip_hal.h) | 部品・ファームウェアの WIT 定義と、ネイティブ形用の C ヘッダ草案 |

## ディレクトリ構成 (✅ = M1 で実装済み)
```text
WasmEmuSil/
  Cargo.toml                 ✅ ワークスペース
  abi/wit/                   ✅ ip:emu パッケージの WIT (部品・ファームウェア・ホスト拡張)。deps/ に WASI 0.2.12 (M5)
  abi/ip_hal.h               ✅ ファームウェア HAL の C ヘッダ (wasm / native 共通)
  schemas/                   ✅ manifest の JSON Schema とサンプル
  dev/cartpole.manifest.json ✅ ローカル開発用 manifest (path 参照・署名なし)
  crates/
    ip-model/                ✅ 物理モデル・線形化・離散 LQR・スイングアップ・速度推定 (no_std, libm)
    ipemu-core/              ✅ manifest 解釈と検証 (V1〜V12)、capability 型・import 抽出・capabilitiesHash (JCS)、同意の型、配線グラフ、スケジューラ
    ipemu-wasm/              ✅ Wasmtime で部品 / FW を実体化、capability に応じた WASI p2 の付与 (wasi.rs)、native FW ローダ
    ipemu-engine/            ✅ headless CLI (run / validate / inspect / consent / lqr)、セッション組み立て、同意ストア
    ip-part-sdk/             ✅ 部品を Rust で書くための SDK (WIT バインディング + 入口マクロ)
    ip-hal/                  ✅ FW を Rust で書くための HAL (wasm import / native vtable) + 入口マクロ
    ip-cmath/                ✅ C 版 FW の wasm ビルド用 libm (Rust の libm を C ABI で公開)
    ipemu-ipc/                  (M2) シェル ⇄ エンジンの IPC 契約
    ipemu-server/               (M3) 配信 API + ポータル / 管理 SPA の配信 (axum)
  components/                ✅ 部品 (Rust → wasm32-unknown-unknown → wasm-tools component new)
    plant-cartpole/  motor-driver/  encoder/  comm-link/
    tcp-reporter/            ✅ (M5) wasi:sockets / wasi:filesystem を使う検証用部品
  firmware/
    fw-balance-c/            ✅ C: freestanding wasm + native。CMake と launch.vs.json (VS 用) 同梱
    fw-balance-rs/           ✅ 同じ仕様の Rust 実装 (cdylib: wasm と native)
  scripts/                   ✅ build-components.sh / build-firmware.sh / check.sh
  tests/golden/              ✅ ゴールデントレース
  apps/
    shell/                      (M2) Electron シェル: 汎用ブラウザ UI + 信頼 origin 限定のブリッジ + ipemu:// ハンドラ
    web/                        (M3/M4) サーバが配信する SPA (Vite + React): ポータルと管理画面
    ui-kit/                     (M2) 共有 UI (sim-view の移植、スコープ、トポロジ表示)
  Docs/
```
