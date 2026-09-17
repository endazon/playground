# 07. ログと診断トレース

「画面を見ているだけでは何が起きているのか分からない」を解消するためのログです。
サーバのコンソール、ブラウザの console、画面のイベントログの 3 か所に、
それぞれ役割の違うログが出ます。

## 3 つの出口

| 出口 | 見る場所 | 何が出るか | 量 |
| --- | --- | --- | --- |
| サーバのコンソール | `dotnet run` した端末 / `docker compose logs app` | コントローラ本体の内部 (状態遷移・帰還・指令・再設計・ウォッチドッグ)、制御ループの生存、Hub の受信、セッションの生成と破棄、ハートビート | 既定 (Production) では節目だけ。Development では 1 秒ごとの集計も出る |
| ブラウザの console | DevTools の Console | プラントの内部 (積分・帰還の生成・指令の受理と拒否・保護動作)、積分ループの生存、Hub の送受信、時計同期 | 同上 |
| 画面のイベントログ | ページ下部の「イベントログ」 | オペレータ向けの節目だけ (操作、状態遷移、再設計、保護動作、時計同期の初回結果) | 最大 80 行 |

画面のイベントログは**意図的に少量**に抑えています。
帰還 1 本ごとの内容や 1 秒ごとの集計まで画面に流すと、肝心の状態遷移が埋もれるためです。
「もっと細かく見たい」ときはコンソール側のレベルを下げてください。

## 診断トレースの仕組み
`Ip.Shared` はロギング基盤 (`Microsoft.Extensions.Logging`) に依存しません。
WASM でもテストでも同じコードを動かすためです。
そこで `Ip.Shared.Diagnostics.DiagnosticTrace` という小さな出口を置き、
`ControllerCore` / `VirtualPlant` / `DelayLine` がそこへ書き、
ホスト側 (`Ip.Server` の `DiagnosticTraceBridge`、`Ip.Client` の `PlantRunner`) が `ILogger` へ橋渡しします。

```text
ControllerCore.Trace  ──┐
VirtualPlant.Trace    ──┼──▶ DiagnosticTrace (発生源 + 重要度 + 本文)
DelayLine.Trace       ──┘          │
                                   ▼
        Ip.Server: DiagnosticTraceBridge ──▶ ILogger "Ip.Controller" / "Ip.Downlink" ──▶ コンソール
        Ip.Client: PlantRunner.Attach     ──▶ ILogger "Ip.Plant" / "Ip.Uplink"        ──▶ ブラウザ console
```

トレースは制御ループの内側から呼ばれるので、**購読者がいない・重要度が足りないときは補間文字列の整形すら行いません**
(`DiagnosticInterpolatedStringHandler`)。ホスト側の `ILogger.IsEnabled` の結果を
`DiagnosticTrace.MinimumLevel` に写しているため、Trace を無効にしておけば帰還 1 本ごとのコストはゼロです。
検証は `tests/Ip.Shared.Tests/DiagnosticTraceTests.cs`。

## 重要度の使い分け

| 重要度 | 何を出すか | 例 |
| --- | --- | --- |
| Trace | 帰還 1 本・指令 1 本ごとの内容。**数百行/秒** | `帰還 Seq 822 t=4728.0 dt=5.0ms x=+0.0000 (0 cnt) θ=+1.23° (14 cnt) ẋ=+0.000 θ̇=+0.021 up=45.9ms e2e=0.0ms drive=ON epoch=1` |
| Debug | 1 秒ごとの集計、キューの状態、指令の破棄、予測器の外挿量の変化 | `集計 Balance #1 t=6.7s: 帰還 200 本 (平均 5.0 ms, 最大間隔 5.0 ms), 指令 200 本 (飽和 0), x=+0.014 m (目標 +0.00), θ=+0.5° (最大 \|θ\| 3.2°), u=+0.82 V (最大 \|u\| 2.5), 上り 22.1 ms, E2E 66.6 ms, ...` |
| Information | 状態遷移、運転開始、設定変更、再設計、リセット、外乱、接続と切断 | `状態遷移 Idle → Balance (CommandId #1) ...` / `再設計 (帰還周期 5→10 ms): ... K=[-7.89, -37.63, -12.21, -7.23], 不安定極 5.32 rad/s, 理論遅延余裕 85 ms (キャッシュ, 0.0 ms)` |
| Warning | 時間スリップ、旧セッションの指令・イベント、キャッシュミスの再設計 (制御ループが止まる)、送信失敗 | `時間スリップ #1: 実時間に 1523 ms 遅れていたので 1323 ms を捨て、原点を後ろへずらした` |
| Error | FAULT、保護動作、ループの異常終了 | `転倒検知: \|θ\|=36.2° > 34°, θ̇=+4.12 rad/s, x=+0.212 m, 直前の指令 +24.0 V` |

## ログのカテゴリと絞り込み

### サーバ (`src/Ip.Server/appsettings.json`)

| カテゴリ | 出るもの |
| --- | --- |
| `Ip.Controller` | コントローラ本体の診断トレース (`ControllerCore`) |
| `Ip.Downlink` | 下り遅延線 (スパイクの注入、配送の遅れ) |
| `Ip.Server.Sessions.ControllerSession` | 制御ループの生存: 周回数、周期の実測 (平均・最大間隔)、1 周回の最大処理時間、受信キューの最大深さと破棄、送信件数 |
| `Ip.Server.Sessions.ControllerSessionManager` | セッションの生成・破棄・上限による拒否 |
| `Ip.Server.Sessions.SessionHeartbeat` | `Sil:HeartbeatSeconds` (既定 10 秒) ごとの全セッションの要約 |
| `Ip.Server.Hubs.SilHub` | Hub の受信 (接続・切断は Information、設定と操作は Debug、帰還は Trace) |
| `Ip.Server.Startup` | 起動時の設定値と待ち受けアドレス |

既定 (`appsettings.json`) は `Ip` が Information、
Development (`appsettings.Development.json`) は Debug です。
帰還 1 本ごとの内容を見たいときは環境変数で上書きします。

```bash
# コントローラの帰還・指令を 1 本ずつ出す (数百行/秒)
Logging__LogLevel__Ip.Controller=Trace dotnet run --project src/Ip.Server

# Hub の受信も 1 本ずつ
Logging__LogLevel__Ip.Server.Hubs.SilHub=Trace dotnet run --project src/Ip.Server

# ハートビートを止める
Sil__HeartbeatSeconds=0 dotnet run --project src/Ip.Server
```

Docker では `compose.yaml` の `environment` に同じキーを足します。
コンソール出力には `HH:mm:ss.fff` の時刻が付くので、帰還の到着と指令の送出の前後関係を ms 単位で追えます。

### ブラウザ (`src/Ip.Client/wwwroot/appsettings.json`)

| カテゴリ | 出るもの |
| --- | --- |
| `Ip.Plant` | プラントの診断トレース (`VirtualPlant`): 積分開始、リセット、指令の受理と拒否、保護動作、1 秒ごとの集計 |
| `Ip.Uplink` | 上り遅延線 |
| `Ip.Client.Services.PlantRunner` | 積分ループの生存 (周回数・最大間隔)、Hub の接続状態、送受信の件数、時計同期の各標本 |
| `Ip.Client.Startup` | 起動時の環境とログレベル |

Blazor WASM は `wwwroot/appsettings.json` と `wwwroot/appsettings.{環境}.json` を読みます。
`Ip.Server` がホストするので、サーバが Development なら `appsettings.Development.json` (Debug) が効きます。
帰還 1 本ごとの内容を見たいときは `appsettings.Development.json` の `Ip.Plant` を `Trace` にしてください
(ブラウザの console は 1 行ごとのコストが大きいので、常用はしないこと)。

## ログで分かること (読み方)

### 「動いているか」を 1 行で判断する
1 秒ごとの集計 (Debug) を見ます。

- サーバ `Ip.Controller` の `集計 ...`: 帰還が周期どおり届いているか (`帰還 200 本 (平均 5.0 ms, 最大間隔 5.0 ms)`)、指令を出しているか、飽和していないか、推定した θ と x
- サーバ `ControllerSession` の `ループ集計 ...`: 制御ループの実測周期 (`平均 4.0 ms` = OS のタイマ粒度)、1 周回の最大処理時間、受信キューの深さ
- ブラウザ `Ip.Plant` の `集計 sim t=...`: 積分が実時間に追従しているか (`実時間との遅れ 最大 ... ms`)、指令が届いて適用されているか (`指令 適用 200/拒否 0`)
- ブラウザ `PlantRunner` の `ループ集計 ...`: 積分ループの周回間隔 (ブラウザのタイマ粒度)、帰還の生成数と送信数の一致

### 「なぜ FAULT になったか」を追う
Error 行に理由と当時の状態が入っています。

```text
fail: Ip.Controller[0] [J5rq…] 転倒検知: |θ|=36.2° > 34°, θ̇=+4.12 rad/s, x=+0.212 m, 直前の指令 +24.0 V
fail: Ip.Controller[0] [J5rq…] FAULT: 転倒検知 |θ|=36°
info: Ip.Controller[0] [J5rq…] AbortCommand #1 を送出 (転倒検知 |θ|=36°): 電圧 0 へ
fail: Ip.Controller[0] [J5rq…] 状態遷移 Balance → Fault (転倒検知 |θ|=36°) CommandId #1, 指令 1047 本, 帰還 1825 本, ...
```

その直前の `集計` 行を見れば、E2E 遅延がどこまで伸びていたか、指令が飽和し始めていたかが分かります。
ウォッチドッグの場合は発火の前に
`帰還が途絶えている: 最後の帰還から 160 ms (ウォッチドッグ 300 ms の半分を超過)` (Debug) が 1 度出ます。

### 「運転開始直後に転倒する」を見分ける
運転開始の直後には次の 2 行が出ます。

```text
info: 運転開始 Balance: CommandId #1, 開始時刻ガード 4723 ms (プラント時計), 周期 5 ms, 予測器 OFF, ウォッチドッグ 300 ms, ...
info: 運転開始後の最初の帰還: Seq 822, 開始から 5 ms (プラント時計), 上り遅延 45.9 ms
```

「倒立から開始」でリセット直後の計測 (`Epoch` の変化と `推定器を初期化: ... θ=1.2°`) が
最初の帰還より**後**に来ていたら、リセット前の吊り下げ状態 (θ=180°) で運転を始めてしまっています。
ブラウザが重くて帰還の送出が遅れているときに起きます (ブラウザ側の `積分ループが N ms 止まっていた` を確認)。

### 通信の詰まりを見る
- サーバ `受信キューが満杯 (2048 件): 古いメッセージを破棄している` (Warning): 制御ループが受信に追いついていない
- サーバ `制御ループの 1 周回に 45.2 ms かかった` (Warning): 再設計 (キャッシュミス) や GC で制御が止まった
- `再設計 (...): ... (新規計算, 38.1 ms — 制御ループを止めた)` (Warning): 未知の組合せの初回設計
- `Ip.Downlink` / `Ip.Uplink` の `再送スパイクを注入: +150 ms → VoltageCommand は 155.0 ms 後に到達` (Debug)
- 遅延線の集計 `配送遅れ最大 7.8 ms`: 到達時刻から実際の配送までの遅れ。Flush を呼ぶループの周期粒度がそのまま現れる

## ハートビート
`SessionHeartbeat` が `Sil:HeartbeatSeconds` ごとに全セッションの要約を Information で出します。
UI を開かずに「どの接続がどの状態で回っているか」が分かります。

```text
info: Ip.Server.Sessions.SessionHeartbeat[0] ハートビート: セッション 1/64 (生成累計 3, 拒否累計 0, 異常累計 0)
info: Ip.Server.Sessions.SessionHeartbeat[0]   J5rq…: 稼働 42 s, Balance #1 帰還 8231 本 (周期 5.0 ms, 上り 22.1 ms, E2E 66.6 ms), u=+0.82 V, x=+0.014 m, θ=+0.5°, 周回 10412, 受信キュー 3 件 (破棄 0), 送信キュー 0 件, 下り [投入 8200 / 配送 8200 / 滞留 0 件, スパイク 0, ...]
```

セッションが無いときは Debug に落とすので、Production の待機中は静かです。
`/healthz` も `sessions` に加えて `totalCreated` / `totalRejected` / `totalFaulted` を返します。
