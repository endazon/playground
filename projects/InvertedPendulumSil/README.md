# 台車型倒立振子 SIL シミュレータ

制御ループ全体が通信路をまたぐ **純 SIL (Software In the Loop)** 構成の実験台です。
プラント (台車・振子・モータ) はブラウザ上の Blazor WebAssembly で 1ms 固定ステップで動き、
コントローラ (状態機械・LQR・スイングアップ・監視) は ASP.NET Core 側にあります。
両者をつなぐのは SignalR だけで、遅延・ジッタ・再送スパイクを注入できます。

目的は「**バックエンドで安定化ループを回す設計が、どこまで成立するのか**」を数値で確かめることです。

```text
┌─ バックエンド (ASP.NET Core) ── コントローラ ───────────┐
│  状態機械 IDLE → SWINGUP → BALANCE / FAULT               │
│  状態推定: パルス → 位置 / 差分+LPF で速度               │
│  離散LQR (帰還周期で再設計) / エネルギー法スイングアップ │
│  予測器(任意) / 転倒・ソフトリミット監視 / ウォッチドッグ│
│  接続ごとに 1 本の制御ループ (Channel で直列化)          │
└────────────────┬─────────────────────────┘
   VoltageCommand (下り) │ │ EncoderFeedback (上り)
   帰還受信ごとに1回     │ │ 周期可変 1〜100ms + MotionEvent 即時
┌────────────────┴─────────────────────────┐
│  SignalR Hub (MessagePack) + 遅延線エミュレータ           │
└────────────────┬─────────────────────────┘
┌────────────────┴─────────────────────────┐
│─ Blazor WASM ── プラント (仮想ハードウェア) ────────────│
│  1ms 固定ステップ RK4 (Stopwatch 基準の追いつき方式)      │
│     台車+振子の非線形モデル / DCモータ(逆起電力)         │
│     エンコーダ量子化 / メカストッパ                       │
│     ドライブ保護: 非常停止・リミット・指令タイムアウト    │
│               ↓ 50Hz で状態を push                       │
│  [JS モジュール] rAF → 描画のみ (2D / 3D / スコープ)      │
└──────────────────────────────────────────┘
```

## クイックスタート
以下のコマンドはすべて**このディレクトリ (`projects/InvertedPendulumSil/`) を起点**に実行します。
```bash
cd projects/InvertedPendulumSil

# .NET SDK で動かす (.NET 10 SDK が必要)
dotnet run --project src/Ip.Server     # http://localhost:5210

# Docker で動かす
docker compose up --build              # http://localhost:5210

# テスト
dotnet test                            # 単体 + 仮想時間の結合 + 実時間の結合
```
`Ip.Server` が Blazor WASM クライアントをホストするので、起動するプロセスは 1 つだけです。

動作の中身はログに出ます (サーバのコンソール、ブラウザの console、画面のイベントログ)。
Development では 1 秒ごとの集計まで出るので、状態遷移・帰還の周期・指令・遅延を数値で追えます。
帰還 1 本ごとの内容まで見たいときは `Logging__LogLevel__Ip.Controller=Trace` を付けて起動します。
詳細は [Docs/07-logging.md](Docs/07-logging.md)。

## ドキュメント
詳細は [`Docs/`](Docs/README.md) にあります。

| ドキュメント | 内容 |
| --- | --- |
| [Docs/01-architecture.md](Docs/01-architecture.md) | 全体構成、プロジェクトの分割方針、責務の置き場所 |
| [Docs/02-protocol.md](Docs/02-protocol.md) | 通信プロトコル、時刻の扱い、監視の多重化 |
| [Docs/03-delay-margin.md](Docs/03-delay-margin.md) | 遅延余裕の理論値と実測、純 SIL が成立する条件、実験手順 |
| [Docs/04-build-and-test.md](Docs/04-build-and-test.md) | ビルド・実行・テストの全手順、Docker、CI の組み方 |
| [Docs/05-design-notes.md](Docs/05-design-notes.md) | 実装上の判断とその理由、既知の制約、今後の課題 |
| [Docs/06-design-mock.md](Docs/06-design-mock.md) | 実装の出発点になった設計モック (単一 HTML) と、実装との差分 |
| [Docs/07-logging.md](Docs/07-logging.md) | ログと診断トレース: 出口 (サーバ / ブラウザ / 画面)、カテゴリと絞り込み、読み方 |
| [Docs/08-dimension-extension.md](Docs/08-dimension-extension.md) | 次元拡張 (多軸化・多リンク化・遅延の状態拡張ほか) の検討と、全案却下の記録 |

## プロジェクト構成

| プロジェクト | 役割 |
| --- | --- |
| `src/Ip.Shared` | 物理モデル・線形化・離散LQR・遅延余裕計算・プロトコル・遅延線・プラント・コントローラ本体 |
| `src/Ip.Server` | SignalR Hub と、接続ごとのコントローラ制御ループ |
| `src/Ip.Client` | Blazor WASM のプラントホストと UI (描画は JS モジュール) |
| `tests/Ip.Shared.Tests` | 数値計算・物理・保護動作の単体テストと、仮想時間の結合テスト |
| `tests/Ip.Server.Tests` | 実際の SignalR + MessagePack を通した実時間の結合テスト |
| `e2e` | Playwright によるブラウザ結合テスト |
| `Dockerfile` / `compose.yaml` | コンテナでのビルド・実行・テスト |
| `Docs` | 本プロジェクトのドキュメント (`Docs/reference/` に設計モックを凍結保存) |

## 結論 (先に)
帰還 100ms 固定の設計は、長い振子 (1.0m 前後) なら LAN 内でぎりぎり成立します。
短い振子やインターネット越しでは余裕を超えます。
実機へ移行する際は、安定化ループをドライブ (組み込み側) に置き、
バックは目標値とモード管理に専念する階層型に切り替えるべきです。
根拠は [Docs/03-delay-margin.md](Docs/03-delay-margin.md) にあります。
