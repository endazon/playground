# 10. NetworkedSil の遅延実験との比較 (M6)

NetworkedSil は、コントローラをサーバに・プラントをクライアントに置き、その間の通信遅延をループに入れた構成です。
その設計メモ ([NetworkedSil/Docs/03-delay-margin.md](../../NetworkedSil/Docs/03-delay-margin.md)) には
(a) 理論遅延余裕の表と、(b) 仮想時間の結合テストで測った「倒立を維持できる / 倒れる」の境目があります。
ここでは同じ比較を WasmEmuSil で再現します。

- (a) は C# の `DelayMarginAnalyzer` を Rust (`ip_model::delay_margin`) に移植して同じ表を出す
- (b) は `comm-link` を**制御ループの中** (仮想 MCU ↔ 駆動系) に入れた manifest を、部品 wasm + ファームウェア wasm の全エミュレーションで回して境目を求める

検証は Rust の中で閉じます (C# は実行しない。NetworkedSil の数字は設計メモとテストに書かれた値をそのまま使う)。

## 結論
> 振子 0.6 m・帰還周期 5 ms・外乱ありで、**全エミュレーションの境目は往復 88 ms 維持 / 90 ms 転倒**。
> 理論遅延余裕 85 ms (NetworkedSil の表、Rust 移植も同値) と 3〜5 ms の差で整合し、
> NetworkedSil の実測 (往復 80 ms 維持 / 90 ms 転倒) とも同じ判定になる。C 版と Rust 版の FW で結果は同一。
> 理論遅延余裕の表 15 マスは Rust 移植で**すべて完全一致** (許容 ±2 ms に対して誤差 0)。

部品に分けた構成 (エンコーダの量子化・PWM ドライバ・MCU 上のファームウェア・1 ms の基本ステップ) にしても、
NetworkedSil が 1 つのハーネスで示した「遅延余裕を超えると倒れる」境目はほぼ動きません。
ジッタと再送スパイクは境目を下げ、その量は NetworkedSil の「ジッタは E2E 遅延に現れる」という観察と同程度です (後述)。

## 1. 理論遅延余裕 (Rust 移植)
`crates/ip-model/src/delay_margin.rs` は `DelayMarginAnalyzer.cs` と同じ構造・定数・判定です。

- 線形化モデル (A, B) を 1 ms で ZOH 離散化し、周期 Ts ごとのサンプル、差分 + 1 次 LPF (25 Hz) の速度推定、
  ZOH の電圧保持、一定遅延 (FIFO) を含む閉ループを 24 s 模擬する。初期値は θ = 0.01 rad
- |θ| ≥ 1e3 (または非有限) なら発散、後半 8 s の |θ| のピークが中盤 8 s のピークの 98% を超えていれば「減衰していない」として不安定
- 0〜600 ms を二分探索して、安定な最大の遅延 [ms] を返す
- ゲインは NetworkedSil の `ControllerCore.CreateDesign` と同じ経路: 周期 Ts で ZOH 離散化したモデルに対する離散 LQR
  (重み Q = diag(40, 60, 1, 0.5), R = 0.6)。`ip_model::lqr::design_gains` がこれと同じなので、
  `delay_margin::design` はそれを呼ぶ。dev manifest の FW ゲイン (`k0..k3`) も同じ設計 (`ipemu-engine lqr --ts 0.005`)
- 丸めは C# の `Math.Round` (偶数丸め) に合わせて `libm::rint` を使う。電圧の飽和・摩擦の非線形・量子化は含まない (C# 版も含まない)

速度推定 LPF 25 Hz。単位は ms。左が NetworkedSil の表 (C# の出力)、右が Rust 移植の出力。

| 振子長 | 不安定極 | 1ms | 5ms | 20ms | 50ms | 100ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0.3 m (NetworkedSil) | 7.5 rad/s | 58 | 57 | 52 | 41 | 35 |
| 0.3 m (Rust) | 7.5 rad/s | 58 | 57 | 52 | 41 | 35 |
| 0.6 m (NetworkedSil) | 5.3 rad/s | 86 | 85 | 80 | 68 | 59 |
| 0.6 m (Rust) | 5.3 rad/s | 86 | 85 | 80 | 68 | 59 |
| 1.0 m (NetworkedSil) | 4.1 rad/s | 110 | 109 | 105 | 95 | 86 |
| 1.0 m (Rust) | 4.1 rad/s | 110 | 109 | 105 | 95 | 86 |

`crates/ip-model/tests/delay_margin.rs` が NetworkedSil の `DelayMarginTests.cs` と同じ許容誤差 (±2 ms) で表を検算し、
単調性 (振子が短いほど・周期が長いほど余裕が減る) と、遅延を 5 ms 刻みで振ったときに安定 / 不安定の境目が 1 つだけ
(二分探索の前提) であることも確かめます。

## 2. 全エミュレーション (comm-link を制御ループに入れる)
### 構成
`dev/cartpole-netdelay.manifest.json`。`dev/cartpole.manifest.json` と同じ部品・パラメータに、仮想 MCU (制御側) と
駆動系 (現場側) の間の信号を**すべて** `comm-link` に通したものです (D61)。

```text
 現場側 (NetworkedSil のクライアント = プラント)      通信路 (comm-link)              制御側 (サーバ)

 plant ─x,θ (lat 1)─▶ enc-cart  ─count (lat 0)─▶ [up-enc-cart]    ─(lat 1)─▶ mcu.enc0
                      enc-pend  ─count (lat 0)─▶ [up-enc-pend]    ─(lat 1)─▶ mcu.enc1
                      driver.fault        ─────▶ [up-drv-fault]   ─(lat 1)─▶ mcu.gpio-in0
                      driver.drive-enabled ────▶ [up-drv-enabled] ─(lat 1)─▶ mcu.gpio-in1
                                                                              │ fw-balance (C / Rust, wasm)
 plant ◀─voltage (lat 0)─ driver.duty      ◀─(lat 1)─ [dn-pwm]       ◀─(lat 0)─ mcu.pwm0
                          driver.enable    ◀─(lat 1)─ [dn-enable]    ◀─(lat 0)─ mcu.gpio-out0
                          driver.heartbeat ◀─(lat 1)─ [dn-heartbeat] ◀─(lat 0)─ mcu.gpio-out1
 plant.x ─(lat 1)─▶ driver.position (メカリミットは現場側で直結)
```

- 上り 4 本・下り 3 本の `comm-link`。同じ向きのリンクは同じ seed (上り 11・下り 23) と同じパラメータなので、
  ジッタ・スパイクの乱数列を共有し、1 つのメッセージとして同じ遅延で届く
- 全リンク `sampleEveryMs: 1`、送信側の配線は latency 0、リンクの出力から先は latency 1。
  これで**片道遅延がちょうど `delayMs`** (1 ms 以上) になる (D60)。manifest の既定は片道 40 ms = 往復 80 ms
- ドライバの指令タイムアウト (`commandTimeoutMs`) は NetworkedSil の `CommandTimeoutMs` と同じ 500 ms。
  ハートビートが下りを通るので、「通信越しの指令が途絶えたら電圧を切る」の意味になる。スパイク (+150 ms) では切れない
- プラントは倒立付近 θ = 0.02 rad から始め (NetworkedSil の `NearUpright`)、FW は 60 ms 後に balance を自動開始する。
  片道 60 ms 以下なら、制御側は本物の計測が届いてから始まる (届く前に始めると comm-link の初期値 0 カウントを計測とみなし、
  本物が届いた瞬間に速度推定が跳ねる)
- 外乱は `plant-cartpole` の `pushes` (D62): 2.03 s から 2 s ごとに振子軸へ 0.3 Nm × 50 ms、向きを交互
  (NetworkedSil の `Push` と同じ大きさ・長さ・時刻)

### 時刻の意味の確認
仮想時間 1 ms ステップで、片道 10 ms・FW の自動開始 30 ms のときに観測した値 (デバッグ用に一時的に出力して確認):

| 事象 | 時刻 |
| --- | --- |
| エンコーダが t = 0 のカウントを出す | ステップ 0 |
| MCU がそのカウントを読める | t = 10 ms (片道 10 ms) |
| MCU が制御 tick (t = 34 ms) で PWM を出す | t = 34 ms |
| ドライバがその PWM で電圧を出し、プラントが [44, 45] ms をその電圧で積分 | t = 44 ms (片道 10 ms) |

計測から適用までが往復 = 2 × `delayMs` ちょうどで、NetworkedSil の `SilHarness` (計測時刻を起点に片道ずつ遅らせ、
到着したステップで計算・適用) と同じです。理論解析の「遅延 d: 時刻 s の計測が s + d から効く」とも同じ意味です。

### 条件の対応
| 項目 | NetworkedSil (`IntegrationTests.cs`) | WasmEmuSil (`net_delay.rs`) |
| --- | --- | --- |
| 振子・パラメータ | 既定 (0.6 m) | 同じ (`dev/cartpole.manifest.json` と同値) |
| 帰還周期 | 5 ms (プラントが 5 ms ごとに帰還を送る) | 5 ms (MCU の `controlHz` 200、MCU 自身の時計で制御) |
| 制御則 | 離散 LQR + 差分 + LPF 25 Hz の速度推定、±24 V で飽和 | 同じ (fw-balance。ゲインも同じ設計) |
| 遅延 | 上り下りとも片道 = RTT / 2 | 同じ (全リンク `delayMs` = RTT / 2) |
| 計測 | プラントでエンコーダ量子化 (2000 / 4096 count/rev) | `encoder` 部品で同じ量子化 |
| 駆動 | プラントが電圧指令を ZOH で保持 | MCU が PWM デューティ (f32) を出し、`motor-driver` が ×24 V |
| 開始 | 倒立付近 θ = 0.02 から 30 ms 後に開始、t = 1 ms からの計測を使う。最初の指令の適用は 2D + 1 ms | 倒立付近 θ = 0.02、FW は 60 ms で開始。最初の指令の適用は 60 + D (+0〜4) ms |
| 外乱 | 2.03 s から 2 s ごとに 0.3 Nm × 50 ms、交互 | 同じ |
| 長さ | 開始から 12 s | 12 s |
| 転倒の判定 | 制御側が FAULT (推定 \|θ̂\| > 0.6 rad・ソフトリミット・プラントの保護通知) | FW の FAULT (同じ条件)・**真の** \|θ\| ≥ 0.6 rad・メカストッパ接触のどれか (D63) |
| 基本ステップ | 1 ms (ハーネスのロックステップ) | 1 ms (`baseStepNs`) |

### 結果 (振子 0.6 m・周期 5 ms・外乱あり)
| 求め方 | 値 |
| --- | --- |
| NetworkedSil の理論遅延余裕 (C#) | 85 ms |
| Rust 移植の理論遅延余裕 (`ip_model::delay_margin`) | 85 ms |
| **WasmEmuSil 全エミュレーションの境目** (C 版 FW / Rust 版 FW、RTT 2 ms 分解能) | **88 ms 維持 / 90 ms 転倒** (両 FW で同一) |
| NetworkedSil の実測 (仮想時間の結合テスト、10 ms 刻み) | 80 ms 維持 / 90 ms 転倒 |

全エミュレーションの RTT ごとの結果 (C 版 FW と Rust 版 FW で全点同じ。転倒は判定が立った時刻と理由):

| RTT | 結果 | 備考 |
| ---: | --- | --- |
| 10 ms | 維持 | max\|θ\| = 0.107 rad |
| 20 ms | 維持 | 0.115 rad |
| 40 ms | 維持 | 0.144 rad |
| 60 ms | 維持 | 0.215 rad |
| 80 ms | 維持 | 0.325 rad |
| 84 / 86 / 88 ms | 維持 | 二分探索 |
| 90 ms | 転倒 | t = 4.37 s (\|θ\| ≥ 0.6、2 回目の外乱の後) |
| 100 ms | 転倒 | t = 2.77 s (1 回目の外乱の後) |
| 110 ms | 転倒 | t = 2.75 s |
| 120 ms | 転倒 | t = 2.35 s (メカストッパ) |

「遅延を増やすと助かる」点はありません (格子 RTT 10〜120 ms の全点で、最初に倒れた点より上はすべて転倒)。
FW の自動開始を「最初の計測が届いた直後」(max(30 ms, 片道)) にしても RTT 84 / 86 / 88 ms 維持・90 ms 転倒で、境目は変わりません
(開始の違い 60 + D と 2D + 1 は結果に効いていない)。

### 差の説明
- **理論 85 ms と全エミュレーション 88 ms の差 (3 ms)**: 理論解析は「24 s のうち後半のピークが中盤の 98% を超える
  = 減衰しない」を不安定とする厳しい判定で、境目のすぐ上は**ゆっくり増える振動**です。全エミュレーションの判定は
  「12 s 以内に 0.6 rad に届く」なので、増え方が遅い 86〜88 ms はまだ倒れません。90 ms でも倒れるのは 2 回目の外乱の後 (4.37 s) で、
  境目の近くほど倒れるまでの時間が長くなります。この差は遅延のモデル化の違いではなく、判定の違いです
- **NetworkedSil の実測 80 ms との差**: NetworkedSil の結合テストは 10 ms 刻み (`MeasuredBoundary` も片道 5 ms 刻み) なので、
  「80 維持 / 90 転倒」は境目が 80〜90 ms のどこかにあるという主張です。WasmEmuSil の 88 / 90 はこの範囲に入っており、矛盾しません
  (C# は実行していないので、NetworkedSil の 82〜88 ms の結果は分かりません)
- **エンコーダの量子化**: NetworkedSil と WasmEmuSil は同じ分解能で丸めるので、両者の差にはなりません。
  理論解析は量子化を含みませんが、理論との差 (3 ms) は上の判定の違いで説明でき、量子化の寄与はそれより小さいと見ています
  (量子化だけを外した切り分けはしていません)
- **ドライバ**: PWM デューティを f32 で運び ×24 V するので、電圧に 1e-6 V 程度の丸めが入ります。境目には効きません。
  ドライバの保護 (ハートビート断 500 ms・メカリミット 0.501 m) が転倒の判定より先に働いた試行はありません
  (ジッタ・スパイクなしの転倒理由はすべて \|θ\| ≥ 0.6 rad かメカストッパ接触)
- **MCU 上のファームウェア**: NetworkedSil の制御側は帰還の到着で計算する (イベント駆動) のに対し、
  fw-balance は MCU の時計で 5 ms ごとに計算する (時間駆動)。comm-link を 1 ms ごとにサンプルするので、
  MCU が読む計測はちょうど片道遅延だけ古く、両者は同じ「5 ms ごとの計測を RTT 後に適用」になります (D60)。
  MCU の実行時間モデル (fuel、1 tick 約 1.4 µs) はステップ (1 ms) より短く、遅延に加わりません
- **1 ms の基本ステップ**: 両者とも 1 ms なので遅延は整数 ms です。comm-link は「ステップ末までに着いたものをそのステップで出す」ため、
  `delayMs` が 0 でも片道 1 ms (= 1 ステップ) 遅れます。RTT 0 の比較はできません (この比較では RTT ≥ 2 ms)

## 3. ジッタと再送スパイク (観察)
遅延が乱数で揺れると、境目は 1 回の乱数列 (seed 固定) で決まる観察値になります。テストでは断定せず、ここに記録だけします。
値は「ジッタ・スパイクを足したときに維持できた最大の基準 RTT (2 × `delayMs`)」で、格子 RTT 10 ms 刻み + 二分探索 (2 ms) です。

ジッタは各リンクで一様 [0, J) ms を足し、追い越し禁止 (先に出たものより早く着かない) です。スパイクは +150 ms (TCP の再送相当)。
`sampleEveryMs` 1 ms (常時のテストと同じ) と 5 ms (NetworkedSil の 1 通 / 5 ms と同じメッセージ頻度) の両方を示します。
5 ms のときは MCU の制御 tick との位相差で、片道ごとに 1〜5 ms の余分な遅れが乗ります (D60)。
スパイクは 1 秒あたりの回数がそろうように、1 サンプルあたりの確率を変えています。

| 条件 | 境目 (sampleEvery 1 ms) | 境目 (sampleEvery 5 ms) |
| --- | ---: | ---: |
| なし | 88 ms | — |
| ジッタ 5 ms | 80 ms | 78 ms |
| ジッタ 10 ms | 72 ms | 72 ms |
| ジッタ 20 ms | 56 ms | 60 ms |
| スパイク 0.1 回/s | 72 ms | 78 ms |
| スパイク 0.5 回/s | 74 ms | 48 ms |
| スパイク 2 回/s | 40 ms | 全点で転倒 (RTT 10 ms でもソフトリミット) |

- ジッタ J = 5 / 10 / 20 ms で境目は 8〜10 / 16 / 28〜32 ms 下がります (上り下りのジッタ幅の合計 2J よりやや小さい)。
  追い越し禁止のため、届く値の遅れは一様分布の平均 J/2 より大きくなります。NetworkedSil の `Jitter_ShowsUpInTheMeasuredEndToEndDelay`
  (ジッタ 20 ms で E2E 遅延が 10〜30 ms 増える) と同じ傾向です
- スパイクは 1 回で 150 ms ぶん計測が止まり、その後まとめて届きます。境目は「スパイクがいつ外乱と重なったか」で大きく動くので、
  1 つの seed の結果は傾向しか示しません。2 回/s (NetworkedSil の `SpikePercent` 1% / 5 ms 周期と同じ頻度) では
  5 ms のときどの遅延でも倒れ、NetworkedSil の `DelaySpikes_PushTheLoopPastItsMargin` (スパイク 100%) と同じく
  「スパイクが頻繁なら余裕を超える」になります

## 再生成
```bash
./scripts/build-components.sh && ./scripts/build-firmware.sh
cargo test -p ip-model --test delay_margin -- --nocapture print_table                                # 1 の表 (数秒)
cargo test --release -p ipemu-engine --test net_delay -- --ignored --nocapture print_delay_report     # 2 と 3 の表 (release で約 20 s)
cargo run -q -p ipemu-engine -- run --manifest dev/cartpole-netdelay.manifest.json --no-signature-check --run-seconds 12
```
常時のテスト (`cargo test`) は軽い版です: `delay_margin.rs` の 6 件 (約 2 s) と、`net_delay.rs` の 2 件
(C 版 FW: RTT 60 / 80 ms 維持・100 / 120 ms 転倒・境目を二分探索して理論 ± 15 ms、Rust 版 FW: 80 ms 維持・100 ms 転倒。
debug ビルドで約 30〜40 s)。境目のすぐ上の 90 ms は、境目からの余裕が 2 ms しかないので常時のテストでは断定していません
(結果は決定的ですが、モデルの小さな変更で入れ替わりうるため)。
