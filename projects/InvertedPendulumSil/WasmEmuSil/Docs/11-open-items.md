# 11. 未決事項と残作業 (2026-11-06 時点)

設計 (M0) から M6 と監査までを終えた時点での引き継ぎ資料です。
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
| M6 | fuel 計時、A7、NetworkedSil との遅延比較、6 プラットフォームの zip | 本資料と同じ PR |

## 1. M6 (拡張) の結果
M6 は完了しました。詳細は各資料にあります。

| 項目 | 結果 | 決定番号 | 資料 |
| --- | --- | --- | --- |
| fuel 計時 | cpu-virtual の `maxFuelPerTick` / `maxFuelPerInit` で、暴走した FW を毎回同じ命令位置で trap させ FAULT にする。fuel / tick の実測 (倒立中の制御 tick: C 版 854、Rust 版 1101) と `nsPerFuel` の較正手順。native FW を fuel モデルで使うと警告 1 回 | D57, D59 | [05](05-firmware.md) |
| A7 | 許可ホストの名前解決を別スレッド + 期限 2 s + キャッシュ 60 s にした。DNS が遅いときにスケジューラが止まる時間は最大 2 s に抑えられる (ゼロにはならない) | D58 | [07](07-client-app.md) |
| 遅延比較 | 理論遅延余裕の Rust 移植は NetworkedSil の表を 15 マスすべて一致。comm-link を制御ループに入れた全エミュレーションの境目は RTT 88 ms 維持 / 90 ms 転倒 (理論 85 ms、NetworkedSil の実測 80 維持 / 90 転倒) | D60〜D63 | [10](10-delay-comparison.md) |
| 6 プラットフォームの zip | electron-builder の zip 構成と `scripts/package.sh`。6 通りともこの環境でビルドできた。実行して確かめたのは linux-x64 だけで、他は成果物のアーキ (`file`) と配置を確認 | D64〜D66 | [07 の配布](07-client-app.md#配布-m6) |

## 2. 実機が要る確認
この開発環境 (Linux x64 のコンテナ) では確かめられないものです。

| 項目 | 確認すること | 反映先 |
| --- | --- | --- |
| Q13: Visual Studio の F5 (Windows) | `firmware/fw-balance-c/.vs/launch.vs.json` で native DLL をエンジンに読ませ、ブレークポイントで止まること。**2026-11-06 に DLL のビルド → エンジンへの装着 → 倒立維持までは Windows 実機で確認済み** ([05 の「実機で確認したこと」](05-firmware.md))。残るのは**ブレークポイントで止める対話操作だけ** | 05-firmware.md の手順 |
| macOS / Windows の zip の起動 | 展開したアプリが起動し、同梱エンジン (`resources/bin`) を見つけ、`ipemu://` を初回起動で登録すること。未署名なので macOS では Gatekeeper の回避手順も確認する。配布前に各 OS 上で `scripts/package.sh --target <os>-<arch>` で作り直す (Linux からのクロスビルドは macOS SDK なしでリンクしている) | 07-client-app.md の配布 (M6) |
| windows-arm64 の MSVC ビルド | この環境では cargo-xwin が `ring` の arm64 ビルドで失敗したため、`aarch64-pc-windows-gnullvm` で代用した。Windows 上で MSVC (`aarch64-pc-windows-msvc`) で作り直して動作を確かめる | 同上 |
| arm64 実機 | Apple Silicon と Windows on Arm、Linux arm64 で、wasm FW と native FW (同じアーキでビルド) が動くこと | 同上 |
| Linux のデスクトップ | `chrome-sandbox` に setuid が付かないため `--no-sandbox` か権限の付与が要る。`ipemu://` の登録はコンテナ (デスクトップ・dbus なし) では確認できなかった | 同上 |

## 3. 小さな残作業
| 項目 | 内容 | 規模 |
| --- | --- | --- |
| Electron の E2E の再実行 | 完了。`apps/shell/e2e/smoke.mjs` (Xvfb + Playwright `_electron`、一時 userData、`xvfb-run -a node apps/shell/e2e/smoke.mjs`) に 6 シナリオを常設して全て合格した: ローカルカタログ起動と sim UI の状態受信 / 信頼 origin だけのブリッジ注入 (http は loopback かつ `allowLocalhost` のときだけ) / web タブの `app://` 遷移拒否 / `app://ui` の外部 fetch の CSP 拒否 / `ipemu://` 確認ウィンドウ / M2-1 (`allowLocalLaunch`)。実行中に M2-1 の既定値が `true` のままだったのを検出し修正した。skip したシナリオはない。Electron 本体が要るので `scripts/check.sh` には入れていない | 完了 |
| 拒否した宛先の通知 | capability で拒否した接続先は `WasmPart::take_denied_destinations()` で取り出せるが、IPC の `event` 通知には載せていない (07-client-app.md) | 小 |
| 単一インスタンスのロックの鍵 | `requestSingleInstanceLock()` がモジュール読み込み時に走り、`IPEMU_SHELL_USER_DATA` の適用より前に取られる。ロックは既定の userData に紐づくので、別の userData を指定した 2 つのシェル (E2E の同時実行など) が衝突する | 小 |
| ホーム画面から開く `app://ui` のタブ | シェル自身のホーム画面は `shell:new-tab` で `app://ui` を web 種別のタブに開けてしまい、そのタブには sim 用ではなくブリッジ用の preload が付く。Web ページからは到達できないので不具合ではないが、タブ種別と URL の対応を main で固定するとよい | 小 |

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
