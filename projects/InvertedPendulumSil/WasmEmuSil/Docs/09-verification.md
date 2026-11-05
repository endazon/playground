# 09. 検証方針

## 方針
外部の基準値 (C# 実装や実測) に依存せず、**Rust の中で閉じて**正しさを示します。
CI は置かず (裁定済み)、`scripts/check.sh` / `scripts/check.ps1` が `cargo fmt --check` / `clippy` / `cargo test` /
`pnpm lint` / `pnpm test` / `markdownlint-cli2` を一括で回します。PR を出す前に手元で通すのが手順です。

## 層
| 層 | 対象 | 手段 | 走る場所 |
| --- | --- | --- | --- |
| L1 性質 | `ip-model` (物理)、LQR | 下表の不変量・性質テスト (`proptest`) | `cargo test` |
| L2 回帰 | 部品 wasm、ファームウェア wasm | ゴールデントレース (ハッシュ + 許容誤差) | `cargo test` (wasm をビルドして Wasmtime で実行) |
| L3 適合性 | 任意の部品 wasm | `ip:emu/part` の契約 (describe と実挙動の一致、reset の冪等、snapshot/restore の往復) | `ipemu-engine conformance <wasm>`、サーバのアップロード時にも実行 |
| L4 決定性 | エンジン全体 | 同じ manifest + 操作列を 2 回 / 別 OS・別アーキで走らせてトレースのハッシュが一致 | `cargo test` (同一機)。OS / アーキ間は手元の 2 台以上で `scripts/check.sh --golden-compare` を走らせて記録 |
| L5 結合 | エンジン + ファームウェア | headless で閉ループ: スイングアップ → 倒立維持 → 外乱 → 復帰、FAULT 経路 | `cargo test` (NetworkedSil の `SilHarness` 相当) |
| L6 契約 | IPC、配信 API | `ipemu-ipc` の型からの TS 生成が最新であること、API のスナップショットテスト | `cargo test` / `pnpm test` |
| L7 E2E | シェル + エンジン + サーバ | Playwright (Electron 対応) でポータルにログイン → 起動 → 同意 → 実行 → 表示、および `ipemu://` 起動を確認 | 手元 (`scripts/e2e.sh`) |
| L8 FW 同等性 | C 版と Rust 版の fw-balance | 同じ manifest・同じ外乱で両者のトレースが許容誤差内 (wasm 形同士はビット一致を目標) | `cargo test` |

## M1 で実装したテスト
| 層 | テスト | 場所 |
| --- | --- | --- |
| L1 | エネルギー保存、平衡の不変、鏡像対称、RK4 の 4 次収束、線形化の整合、ZOH の半群性、リカッチ残差と閉ループ減衰、非線形プラントでの倒立維持、スイングアップのエネルギー到達、速度推定、wrap の性質 (proptest) | `crates/ip-model/tests/properties.rs` |
| 検証規則 | latency-0 閉路の拒否 / latency-1 閉路の許容、不正ポート、入力の重複配線、sha256 欠落、実行順、latency の転記規則、rateDivider、仮想 MCU の tick と実行時間モデル、スキーマ | `crates/ipemu-core/tests/core.rs` |
| L2 | ゴールデントレース (`tests/golden/cartpole-c.csv`、100 ms ごとの θ / x / u。ビット一致 → 許容誤差の 2 段) | `crates/ipemu-engine/tests/closed_loop.rs` |
| L4 | 同じ manifest を 2 回走らせてハッシュ一致 | 同上 |
| L5 | C wasm / Rust wasm / C native / Rust native の 4 形態でスイングアップ → 倒立維持。吊り下げから LQR を始めると転倒検知 → FAULT → ドライブ無効 | 同上 |
| L8 | C 版と Rust 版の軌道一致 (現状ビット一致) | 同上 |

## M6 で追加したテスト
| 層 | テスト | 場所 |
| --- | --- | --- |
| L4 | fuel モデルで同じ manifest を 2 回走らせ、tick ごとの `mcu.exec-ns` の列が一致 (C 版 / Rust 版) | `crates/ipemu-engine/tests/fuel_timing.rs` |
| 性質 | `exec-ns = round(fuel × nsPerFuel)` (端数の出る係数を含む)。fuel は実行時間モデルによらない | 同上 |
| 性質 | exec-ns が tick 周期を超える係数でオーバーラン数 = 周期超えの tick 数、PWM の変化が記録した fuel から計算した時刻に遅れて出る | 同上 |
| L5 | 無限ループする wasm (WAT) が `maxFuelPerTick` で毎回同じ tick に trap → FAULT、init の暴走は `maxFuelPerInit` で装着失敗。実物の FW も予算不足なら同じ位置で `step` が失敗 | 同上 |
| L5 | native 形を fuel モデルで装着すると警告 1 回、exec-ns は 0 | 同上、`crates/ipemu-core/tests/core.rs` |
| 単体 | 名前解決: 止まる解決器でも期限内に `temporary-resolver-failure`、同じ名前の再試行でスレッドを増やさず後で回収、キャッシュの再利用 | `crates/ipemu-wasm/src/wasi.rs` |

fuel の絶対値はコンパイラの版で変わるため、テストは値を固定せず関係だけを確かめます (較正表は [05](05-firmware.md#nsperfuel-の較正))。

## L1 性質テストの中身
| 性質 | 確認すること |
| --- | --- |
| エネルギー保存 | 摩擦 0・電圧 0 で、力学的エネルギーの変動が RK4 の次数相当 (ステップ 1ms で 10 秒間に相対 1e-6 以下) |
| 平衡の不変 | 倒立点・垂下点に静止させて電圧 0 なら、状態が数値誤差の範囲で動かない |
| 対称性 | (x, θ, u) → (−x, −θ, −u) の軌道が鏡像になる |
| 収束次数 | ステップを半分にしたとき、基準 (1/8 ステップ) との誤差が約 1/16 になる (RK4 = 4 次) |
| 線形化の整合 | 倒立点近傍の微小振幅で、非線形モデルと線形化モデル (A, B) の応答差が振幅の 2 乗オーダー |
| 離散化 | ZOH 離散化 (行列指数) が、微小 Δt で `I + AΔt` に一致し、Δt を 2 分割した積と一致する |
| LQR | 解いた P が離散 Riccati 方程式を満たす (残差ノルム < 1e-9)、閉ループ `A − BK` の固有値が単位円内 |
| 遅延余裕 | NetworkedSil の `DelayMarginAnalyzer` と同じ理論 (遅延を状態拡張して二分探索) を Rust に移植し、同じパラメータで同じ余裕 [ms] を出す (移植時 1 回の手動突合せで一致を確認し、以後はゴールデン化) |
| 保護 | motor-driver: 指令タイムアウト 500ms で電圧 0、無効中の指令拒否、E-STOP 後は再初期化まで有効化不可 |
| エンコーダ | 量子化誤差が ±1 パルス以内、方向反転時に計数が飛ばない |

## ゴールデントレース
- `tests/golden/<scenario>.csv` に、仮想時刻・主要ポート値を 10ms 間隔で記録したものをコミットします。
- 比較は 2 段: (1) 全行のハッシュ一致 (決定性)、(2) 不一致なら列ごとの許容誤差で判定し、差分の最大値と時刻を報告。
- 更新は `cargo test -- --update-golden` で明示的に行い、PR の差分としてレビューします。
- **移植時の 1 回だけ**、NetworkedSil の `SilHarness` で同じシナリオを C# で回し、結果を `Docs/porting-check.md` に記録します (CI には入れません)。

## 決定性 (L4) の判定範囲
| 形 | 保証 |
| --- | --- |
| 部品 wasm + ファームウェア wasm | OS / CPU をまたいでビット一致 |
| ファームウェア native | 同一ビルド・同一マシンでの再現のみ。OS をまたいだ一致は保証しない (FMA / x87 の差) |

## ファームウェアの単体テスト
`abi/ip_hal.h` の native バックエンドをモック (`tests/hal_mock.c`) に差し替えれば、
ファームウェアはエンジンなしでテストできます。状態機械の遷移、ゲイン計算、保護の判定はここで押さえます。
