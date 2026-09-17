# 06. 設計モック (実装の出発点)

## これは何か
[`reference/design-mock.html`](reference/design-mock.html) は、本プロジェクトの実装に先立って作られた
**単一 HTML の設計モック**です。ブラウザで開くだけで動きます (外部サーバ不要)。

本実装はこのモックが示した設計を .NET 10 プロジェクトとして作り直したものなので、
モックは「なぜこの構成なのか」を追うための一次資料として残しています。
実装の正は `src/` であり、モックは**凍結された参照**です。差異があれば実装が正しいものとします。

```bash
# 開くだけ
open Docs/reference/design-mock.html     # macOS
xdg-open Docs/reference/design-mock.html # Linux
```

> モックは three.js を cdnjs から読み込みます。遮断環境では 3D ビューだけがフォールバック表示になり、
> 側面図とスコープは動きます。本実装が three.js を同梱している理由はまさにこれで、
> 詳細は [05-design-notes.md](05-design-notes.md) を参照してください。

## モックが持っていて、本実装のドキュメントには無いもの
- **前回設計 (搬送機シミュレータ) との差分表** — 下り指令が「動作開始時に 1 回の `MoveCommand`」から
  「帰還ごとの `VoltageCommand`」へ変わった経緯、帰還周期を固定から可変にした理由、
  `epoch` (リセット世代) を追加した理由。
- **Blazor WASM へ移植する際の注意** — `WasmEnableThreads` と COOP/COEP、
  `[JSImport]/[JSExport]` がメインスレッド専用であること、
  .NET 10 のマルチスレッド WASM で `System.Threading.Timer` が落ちる報告。
  本実装が「固定ステップループを Timer に依存させない」「`SharedArrayBuffer` を使わない」
  という判断に至った出所です。
- **状態機械の遷移表** — IDLE / SWINGUP / BALANCE / FAULT の遷移条件
  (`|θ| < 0.3rad` かつ `|θ̇| < 4rad/s` で BALANCE へ、`|θ| > 0.6rad` / `|x| > 0.45m` で FAULT へ)。
  本実装では `src/Ip.Shared/Control/ControllerCore.cs` がこの表どおりに動きます。

## モックと本実装の違い
モックは「設計が成立するかを手早く確かめる」ためのもので、実行基盤が本実装と異なります。

| 項目 | 設計モック | 本実装 |
| --- | --- | --- |
| 言語 | JavaScript (単一 HTML) | C# / .NET 10 |
| プラントの置き場 | Web Worker | Blazor WASM (ブラウザ) |
| コントローラの置き場 | もう 1 つの Web Worker | ASP.NET Core (サーバ) |
| 両者をつなぐもの | `MessageChannel` + 遅延線で通信を模擬 | **実際の SignalR (MessagePack)** + 遅延線エミュレータ |
| 状態の受け渡し | `SharedArrayBuffer` (seqlock)、不可なら `postMessage` | 50Hz の状態 push (SAB は不使用) |
| Worker が使えない場合 | メインスレッドで同じ関数を実行するフォールバック | 該当なし |
| three.js | cdnjs から読み込み | リポジトリに同梱 |
| テスト | Node 上の実時間結合テスト | xUnit (単体 / 仮想時間の結合 / 実時間の結合) + Playwright |
| 実測の遅延境界 | 往復 **約 70ms** 維持 / 約 90ms 転倒 | 往復 **80ms** 維持 / 90ms 転倒 |

理論遅延余裕の表 (振子長 × 帰還周期) は**モックと本実装で完全に一致**します
(0.6m・5ms で 85ms など)。同じアルゴリズムを移植し、`DelayMarginTests` が ±2ms で検証しているためです。

### 実測の境界が 70ms → 80ms に動いた理由
モックの実測は**実時間**で動く Node のハーネスによるもので、
測定そのものがタイミングの揺らぎを含みます。本実装の 80ms は
**仮想時間のロックステップ**ハーネス (`tests/Ip.Shared.Tests/SilHarness.cs`) によるもので、
同じ条件なら必ず同じ結果になります。理論値 85ms に対しては、後者のほうが整合しています。

この「量的な主張は仮想時間のテストが持つ」という方針そのものが、
モックを実装へ移す過程で得た結論です ([04-build-and-test.md](04-build-and-test.md) 参照)。

## モックのままにしてある記述
モックの設計メモには、実装時点で変わった記述が一部残っています。読み替えてください。

| モックの記述 | 現在 |
| --- | --- |
| 「※モックでは遅延線で模擬」(SignalR Hub の箇所) | 本実装では実際の SignalR Hub が動く |
| 「[Worker] 1ms 固定ステップ RK4 → SharedArrayBuffer (seqlock)」 | 本実装は Blazor WASM の `PlantRunner` が 1ms 固定ステップで回し、50Hz で JS へ push |
| 「Node 上の実時間結合テストで往復約 70ms」 | 仮想時間の結合テストで往復 80ms ([03-delay-margin.md](03-delay-margin.md)) |
| 「setTimeout の最小間隔 (約4ms) の制約」 | 本実装では `Task.Delay(1)` の粒度として同じ制約が残る ([05-design-notes.md](05-design-notes.md)) |
