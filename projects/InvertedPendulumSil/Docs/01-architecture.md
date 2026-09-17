# 01. アーキテクチャ

## 全体像
制御ループの**内側**に通信路が入っているのが本構成の特徴です。
プラント (仮想ハードウェア) はブラウザ、コントローラはサーバにあり、
両者は SignalR (MessagePack) だけでつながっています。

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

## プロジェクトの分割
| プロジェクト | 種別 | 役割 |
| --- | --- | --- |
| `src/Ip.Shared` | classlib | 物理モデル、線形化、離散 LQR、遅延余裕計算、プロトコル、遅延線、プラント、コントローラ本体 |
| `src/Ip.Server` | ASP.NET Core | SignalR Hub と、接続ごとのコントローラ制御ループ。Blazor WASM クライアントのホストも兼ねる |
| `src/Ip.Client` | Blazor WASM | プラントの実時間ホストと UI。描画は JS モジュールに委譲 |
| `tests/Ip.Shared.Tests` | xUnit | 数値・物理・保護動作の単体テストと、**仮想時間**の結合テスト |
| `tests/Ip.Server.Tests` | xUnit | 実際の SignalR + MessagePack を通した**実時間**の結合テスト |
| `e2e` | Playwright | 実 Chromium でのブラウザ結合テスト |

### なぜ制御ロジックとプラントを `Ip.Shared` に置くのか
アプリ (実時間) とテスト (仮想時間) が**同じコード**を動かすためです。
テスト用に簡略化したモデルを別に持つと、
「テストでは通ったが実機構成では別物だった」という乖離が必ず起きます。

`Ip.Shared` は実時間にも UI にも依存しません。時刻は引数として外から渡され、
`MonotonicClock` は実時間ホスト側でのみ使われます。
この分離があるので、仮想時間のハーネス (`tests/Ip.Shared.Tests/SilHarness.cs`) は
プラント・遅延線・コントローラをロックステップで回し、再現性のある実験ができます。

## `Ip.Shared` の内訳
| 名前空間 | ファイル | 中身 |
| --- | --- | --- |
| `Model` | `PendulumParameters`, `CartPoleDynamics`, `CartLimits`, `PlantState` | 台車+振子の非線形モデル、DC モータ (逆起電力)、物理パラメータ、機械的制限 |
| `Numerics` | `Matrix`, `DiscreteLqr` | 行列演算・行列指数 (ZOH 離散化)、離散時間 LQR の Riccati 反復 |
| `Control` | `LinearizedModel`, `ControllerCore`, `SwingUpController`, `VelocityEstimator`, `DelayMarginAnalyzer` | 倒立点まわりの線形化、状態機械と制御則、エネルギー法スイングアップ、差分+LPF の速度推定、遅延余裕の二分探索 |
| `Protocol` | `Messages`, `ClockSynchronizer` | 上下りメッセージ定義、受信値の矯正 (`Sanitize`)、NTP 方式のクロック同期 |
| `Simulation` | `VirtualPlant`, `DelayLine` | 仮想ハードウェア (エンコーダ量子化・ドライブ保護含む)、遅延・ジッタ・再送スパイクのエミュレータ |

`ControllerCore` は本プロジェクトで最も大きい型 (470 行) で、
状態機械・状態推定・LQR 再設計・予測器・監視をまとめて持ちます。
ロックを一切持たないのは、後述のとおり呼び出しが 1 本の制御ループに直列化されているためです。

## サーバ側の実行モデル
```text
SignalR Hub (SilHub)
  └ 受信したメッセージを Channel に積むだけ (重い処理はしない)
        ↓
ControllerSession (接続ごとに 1 インスタンス)
  └ 単一のループが Channel を読み、ControllerCore を呼び、指令を送り返す
        ↑
ControllerSessionManager  … 接続とセッションの対応、上限 (Sil:MaxSessions) の管理、解放
```
Hub のメソッドで直接制御計算をすると、その呼び出しスレッドが占有されて
**他の接続の受信まで遅れます**。受信と計算を Channel で切り離すことで、
接続どうしの干渉をなくし、同時に `ControllerCore` からロックを追い出しています。

1 接続 = 独立した 1 シミュレーションであり、接続間で状態は共有されません。

## クライアント側の実行モデル
```text
PlantRunner (.NET / WASM)
  ├ Stopwatch で経過時間を測り、必要なステップ数だけ 1ms 固定ステップ RK4 を進める
  ├ 帰還周期ごとに EncoderFeedback を送信 / MotionEvent は即時送信
  └ 50Hz で状態を JS へ push
        ↓
sim-view.js (JS モジュール)
  └ requestAnimationFrame で 2D / 3D / スコープを描画するだけ
```
描画を JS の rAF に逃がしているため、描画が重くなっても
制御ループもプラントの積分周期も乱れません。
詳細な理由は [05-design-notes.md](05-design-notes.md) を参照してください。

## 設定
`src/Ip.Server/appsettings.json` の `Sil` セクション。

| キー | 既定 | 意味 |
| --- | --- | --- |
| `CrossOriginIsolation` | `false` | `true` で COOP/COEP ヘッダを配信する (`SharedArrayBuffer` を使う場合) |
| `MaxSessions` | `64` | 同時に保持するコントローラセッションの上限 |
