# 11. 未決事項と残作業 (2026-11-05 時点)

設計 (M0) から M5 と監査までを終えた時点での引き継ぎ資料です。
次の作業者 (人でも Claude のセッションでも) がこの資料だけで再開できるように、
**残っている作業**、**実機が要る確認**、**判断を保留しているもの**、**設計で対象外にしたもの**、**再開の手順と環境の制約**を分けて書きます。
判断の経緯は [08-decisions.md](08-decisions.md)、監査の所見は [audit/2026-11-05-m1-m5-m3.md](audit/2026-11-05-m1-m5-m3.md) にあります。

## 完了しているもの
| M | 内容 | 取り込み |
| --- | --- | --- |
| M0 | 設計 | 設計資料 01〜09 |
| M1 | エンジン核 (部品 / FW / headless 実行 / 性質テスト) | develop |
| M5 | capability (WASI p2 の宣言 → 承認 → 同意) | `endazon/playground#7` |
| M3 / M4 | 配信サーバ + ポータル / 管理画面、bootstrap とキャッシュ | `endazon/playground#8` |
| M2 | Electron シェル + IPC | `endazon/playground#9` |
| 監査 | 所見 A1〜A10 と M2-1 の修正 (D50〜D56) | `endazon/playground#10` |

## 1. 進行中: M6 (拡張)
| 項目 | 内容 | 決定番号 | 状態 |
| --- | --- | --- | --- |
| fuel 計時 | tick ごとの fuel 上限 (暴走した FW を決定的に FAULT にする)、実 FW での性質テスト、`nsPerFuel` の校正手順、native FW を fuel モデルで使ったときの警告 | D57〜D59 | 作業中 |
| A7 | 許可ホストの名前解決を別スレッド + 期限 + キャッシュにし、DNS の遅延でスケジューラが止まらないようにする | D57〜D59 | 作業中 |
| 遅延比較 | NetworkedSil の遅延余裕解析 (`DelayMarginAnalyzer.cs`) の Rust 移植、comm-link を制御ループに入れた manifest、境界の掃引、比較記録 (10-delay-comparison.md) | D60〜D63 | 作業中 |
| 6 プラットフォームの zip | electron-builder の zip 構成、`scripts/package.sh`、この環境で作れる分のビルドと確認 | D64〜D66 | 作業中 |

M6 の統合時に、この表を結果で更新します。

## 2. 実機が要る確認
この開発環境 (Linux x64 のコンテナ) では確かめられないものです。

| 項目 | 確認すること | 反映先 |
| --- | --- | --- |
| Q13: Visual Studio の F5 (Windows) | `firmware/fw-balance-c/.vs/launch.vs.json` で native DLL をエンジンに読ませ、ブレークポイントで止まること。M2 では Linux で native `.so` のロードと `restartRequired` の経路まで確認済み | 05-firmware.md の手順 |
| macOS / Windows の zip の起動 | 展開したアプリが起動し、同梱エンジンを見つけ、`ipemu://` を初回起動で登録すること。未署名なので macOS では Gatekeeper の回避手順も確認する | 07-client-app.md の対象プラットフォーム |
| arm64 実機 | Apple Silicon と Windows on Arm で、wasm FW と native FW (同じアーキでビルド) が動くこと | 同上 |

## 3. 小さな残作業
| 項目 | 内容 | 規模 |
| --- | --- | --- |
| Electron の E2E の再実行 | Xvfb + Playwright (`_electron`) によるシェルの結合確認は M2 のときに手元で 1 回実施しただけ。監査修正 M2-1 でシェルの main プロセスを変えたので再確認する。M6 の統合時に実施予定 | 小 |
| 拒否した宛先の通知 | capability で拒否した接続先は `WasmPart::take_denied_destinations()` で取り出せるが、IPC の `event` 通知には載せていない (07-client-app.md) | 小 |

## 4. 判断を保留しているもの
| 項目 | 現状 | 再開の条件 |
| --- | --- | --- |
| クライアントの Chromium 機能の拡張 | 「Chromium の機能が欲しい」という要望に対し、一旦は現行の Electron シェル (汎用ブラウザ + 信頼 origin ブリッジ) のままとする裁定 | 必要な機能 (拡張機能、DevTools Protocol、独自プロトコル、ブラウザプロファイルなど) が具体化したとき。候補の比較は 08-decisions.md の D8 の調査結果にある |

## 5. 設計で対象外にしたもの (v1)
01-concept.md の「スコープ外」と同じです。やらないと決めたものなので、着手には設計の見直しが要ります。

- 命令セットレベルの CPU エミュレーション (`cpu-virtual` を差し替えられる構造だけ残している)
- 多軸・多リンクへの次元拡張
- 複数クライアント間の連携、1 クライアントでの複数シミュレータの同時実行
- クライアント本体の自己更新 (zip の差し替えで更新)
- コード署名 (Authenticode / notarization)。electron-builder は署名を足せる構成にする
- CI (D24)。検証は `scripts/check.sh` のローカル実行
- 署名鍵の KMS / HSM 化 (現在はファイル権限 0600)

## 6. 再開の手順と環境の制約
### 検証
```bash
cd projects/InvertedPendulumSil/WasmEmuSil
ELECTRON_SKIP_BINARY_DOWNLOAD=1 ./scripts/check.sh
```
fmt / clippy --tests / 部品・FW のビルド / cargo test 全体 / manifest 検証 / IPC 型の差分 / pnpm lint・test・build / markdownlint を順に実行します。
マージ前にこれが通ることを確認する運用です (CI は置かない)。

### 開発環境で分かっている制約
| 制約 | 対処 |
| --- | --- |
| github.com (Releases を含む) がプロキシで遮断される。crates.io / npm / PyPI は通る | Electron 本体は `ELECTRON_MIRROR=https://npmmirror.com/mirrors/electron/` で取得する。`check.sh` は `ELECTRON_SKIP_BINARY_DOWNLOAD=1` で本体なしに通る |
| ディスクの上限が約 39GB。debug の target は 25GB を超える (テストバイナリ 1 本が約 500MB) | 不要な target を消す。`CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0` で縮める |
| worktree をまたいで `CARGO_BUILD_BUILD_DIR` を共有すると、ワークスペース内の crate が別 worktree のソースのままビルドされる (メタデータのハッシュが同じで、dep-info が相対パスのため鮮度判定をすり抜ける) | 並行作業では worktree ごとに `RUSTFLAGS="--cfg <worktree 名>"` を付けてハッシュを分けるか、build-dir を分ける。マージ前の検証は共有なしのツリーで行う |
| CPU 4 コア / メモリ 15GB | wasmtime を含むビルドの並行は 3 本程度まで |
| 強制 push は権限で拒否される | マージ済みのブランチで続きをするときは、develop をマージしてから通常の push をする |

### ブランチの運用
- 統合先は `develop`。機能ごとにブランチを切り、PR を squash マージする。
- 決定は 08-decisions.md の D 番号で追記する。並行作業では番号の範囲を先に割り当てて衝突を防ぐ。
