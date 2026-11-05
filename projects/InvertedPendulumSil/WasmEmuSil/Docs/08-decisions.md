# 08. 技術選定と未決事項

## 決定事項 (裁定済み)
| # | 項目 | 決定 | 却下した案と理由 |
| --- | --- | --- | --- |
| D1 | ディレクトリ名 | `NetworkedSil` (既存) / `WasmEmuSil` (新規) | 旧名のまま 1 段下げる案: 2 つの形の違いが名前に出ない |
| D2 | クライアントの対象 OS | Windows / macOS / Linux | Windows 専用: 利用者の環境を縛る。ファームウェアも全 OS で同じ wasm を動かす |
| D3 | エンジンの言語 | **Rust** + wasmtime crate | .NET + wasmtime-dotnet: Component Model / WASI p2 対応が [PR 段階](https://github.com/bytecodealliance/wasmtime-dotnet/pull/360)で、許可制アクセスを自前実装することになる |
| D4 | 部品のインターフェース | **Component Model (WIT)** | フラット C ABI: .NET ホストなら必要だったが Rust では不要。バイナリから import を読めない |
| D5 | 部品の言語 | **Rust** (`cargo component`) | C/C++: 可能だが安全性で劣る。C# (wasi ワークロード): ランタイム同梱で数 MB・起動が遅い・ツールチェーンが実験的 |
| D6 | ファームウェアの言語 | **C / C++ / Rust** (HAL 境界は C ヘッダと WIT の 2 表現) | C# managed DLL: wasm への経路が実験的で、実機移植性がない |
| D7 | ファームウェアの配布形 | **wasm が正**。ネイティブ共有ライブラリはデバッグ専用の第 2 形 | ネイティブも正式配布: OS ごとの署名・配布が要り、サンドボックスが効かない |
| D8 | UI シェル | **Electron** で開始。IPC 契約を固定して差し替え可能に | 下表 |
| D9 | 許可制 | manifest 宣言 + 管理者承認 (署名) + クライアントで初回同意 | 承認のみ: 利用者が拒否できない。ローカル設定のみ: 配布管理と切り離される |
| D10 | サーバの言語 | **Rust (axum)** + TypeScript の管理 SPA | ASP.NET Core + Blazor: manifest 検証を C# で二重実装するか Rust 製 CLI を呼ぶ必要がある |
| D11 | 検証 (オラクル) | Rust 内で閉じた性質検証 + 自前ゴールデントレース。C# との突合せは移植時 1 回だけ手動 | C# を CI で実行: .NET SDK が CI に要り、基準値の更新が属人化する |
| D12 | UI フレームワーク | **Vite + React + TypeScript** (ポータル / 管理 SPA とシミュレータ UI で共通) | Svelte: 軽いが共有部品のエコシステム (CodeMirror / three.js 連携) の情報量で劣る。Vue: 同等だが人手の都合で React |
| D13 | 起動の入口 | **サーバのポータル**でカタログから「起動」。シェル内はブリッジ、外部ブラウザは `ipemu://` | エンジンが割当を取る方式: 利用者が構成を選ぶ場面がなく、キオスク向け |
| D14 | シェルの性格 | **汎用ブラウザ** (任意のサイトを閲覧可)。ブリッジは信頼 origin のみ | 信頼 origin 限定: 安全だが「普通のブラウザとして使える」要件を満たさない |
| D15 | ポータル認証 | 利用者ログイン (管理者と同じアカウント基盤、`user` ロール)。エンジンはセッショントークンを使う。API キーは headless 用 | 認証なし: 誰が起動したかが残らない。端末 API キーのみ: 共有端末で監査が曖昧 |
| D16 | 同時実行 | v1 は **1 セッション**。別構成の起動は切り替え | 最初から複数: 実時間ペーシングの干渉と UI の複雑さ |
| D17 | 本体の配布 | **未署名 zip** の社内配布。electron-builder は署名可能な構成にしておく | 最初からコード署名: 証明書費用と鍵運用 |
| D18 | テレメトリ | **既定 on**、要約のみ、クライアント設定で off | 既定 off: 管理画面の「クライアント」情報が乏しい。v1 から外す: 後から入れると契約変更になる |
| D19 | 管理者認証 | **ローカルアカウント (argon2id) のみ**。OIDC は拡張点 | 最初から OIDC: 評価環境でも IdP が要る |
| D20 | 対象アーキ | **x64 + arm64** を 3 OS すべてで | x64 のみ: Apple Silicon で Rosetta 依存 |
| D21 | comm-link | **v1 (M1) に含める** | M6 送り: NetworkedSil の遅延実験との比較が遅れる |
| D22 | FW の初回実装 | **C と Rust の 2 本** (同じ仕様、同じゴールデン) | C のみ: HAL 境界が C 寄りになっていても気づけない |
| D23 | FW リビルドの反映 | **wasm はホット差替 (状態保持)、native はエンジン再起動** | 再起動のみ: wasm の利点を捨てる。全部ホット: native のアンロードが信頼できない |
| D24 | CI | **置かない**。`scripts/check.*` のローカル検証と手順書 | GitHub Actions: 今は運用コストを払わない判断。NetworkedSil と同じ |
| D25 | 部品 / FW の wasm ターゲット (M1 で確定) | **`wasm32-unknown-unknown` + `wasm-tools component new`** | `cargo component` / `wasm32-wasip2`: std が `wasi:cli` 等を import し、WASI なしの原則と V7 に反する |
| D26 | C 版 FW の libc / libm (M1 で確定) | **freestanding + `ip-cmath` (Rust libm の C ABI) + 極小 libc** | wasi-sdk: wasi-libc が WASI を import する。外部ツールチェーンの取得も要る。Rust libm を共有すると C / Rust の wasm で数値が一致する |
| D27 | 指令タイムアウトの表現 (M1 で確定) | **ハートビート信号** (gpio-out1 の反転) を motor-driver が監視 | 指令値の更新検知: 同じ値が続く正常運転と区別できない |
| D28 | FW cfg の形 (M1 で確定) | **フラットな JSON** + 極小パーサ (C / Rust 同規則) | 入れ子 JSON + serde: C 版に JSON ライブラリが要り、MCU 向けの制約から外れる |
| D29 | サーバの認証方式 (M3 で確定) | **Bearer トークンのみ** (ログインセッション / チケット交換トークン / API キー)。Cookie は使わない | セッション Cookie + CSRF トークン: 設計案だったが、同じトークンをシェル経由でエンジンに渡す経路と二重になる。Bearer に統一すると CSRF 対策が不要で、SPA・シェル・エンジンの扱いが揃う |
| D30 | 公開鍵の配布形式 (M3 で確定) | **raw 32 バイトの hex を JSON に包む** (`{keyId, alg, publicKey}`)。クライアントに `trusted-keys.json` として埋め込み、開発時は環境変数 / CLI で追加 | PEM (PKCS#8 / SPKI): 標準的だが ASN.1 の解釈が要り、keyId を別に持つ必要がある。hex は目視で照合しやすい |
| D31 | `capabilitiesHash` の定義 (M3 で確定) | capabilities を持つ部品の `[{instance, capabilities}]` (manifest 順) を JCS 正規化した sha256 | capabilities 値だけの配列: 同じ宣言を別の部品に付け替える改変を検出できない |
| D32 | 本文の版採番と不変性 (M4 で確定) | revision はサーバ採番。承認済み / 公開済みは**編集不可**で新しい下書きを作る。公開済みは削除不可 | 承認後の編集で下書きに戻す: 履歴が追いにくく、承認の紐付けが曖昧になる |
| D33 | V7 / V8 の実装場所 (M3 で確定) | サーバ (アップロード時に保存した wasi import と宣言の照合)。クライアントの再照合は M5 | ipemu-core に WIT パーサを入れる: クライアントの依存が増える。サーバは wasmparser だけで import 名を読める |
| D34 | 編集者と承認者の分離の実装 (M4 で確定) | revision の**最終編集者 (ユーザ名)** と承認者が同一なら admin でも拒否 | ロールの排他 (editor と approver を兼ねられない): 小規模運用で人が足りない。本文単位の分離で十分 |

### D8 の調査結果: クロスプラットフォームで Chromium を同梱する手段
| 方式 | Chromium の出所 | 成熟度 | Rust エンジンとの関係 | 評価 |
| --- | --- | --- | --- | --- |
| **Electron** | 同梱 (Node + Chromium) | 最高 | 別プロセス (IPC) | 開発リスク最小。配布 ~150MB が難点 |
| [Electrobun](https://electrobun.dev/electrobun/) (`defaultRenderer: "cef"`) | 既定は OS webview、[CEF を同梱可](https://docs.electrobun.dev/electrobun/apis/bundling-cef/) | 低〜中 ([v1 は 2026 年初](https://news.ycombinator.com/item?id=47069650)) | 別プロセス | 軽量。実績が浅く CEF モードの不具合リスク。**第 1 候補の差し替え先** |
| Qt WebEngine (Qt 6) | 同梱 (Qt の Chromium) | 高 | 別プロセス | C++。SignalSimulator の Qt 資産が活きるが、UI を TS で書く方針と二重になる |
| [tauri-apps/cef-rs](https://github.com/tauri-apps/cef-rs) | 同梱 CEF | 低 ([cef crate](https://crates.io/crates/cef) は CEF 152 追従中、未完成の評価) | **単一プロセス可** | 理想形。成熟したら移行を検討 |
| NW.js | 同梱 | 中 (停滞) | 別プロセス | Electron に対する利点がない |
| Tauri / wry, Photino, Neutralino, Avalonia + CefGlue | OS webview または .NET 描画 | — | — | mac / Linux で Chromium にならない。除外 |

## 設計上の判断 (裁定不要と判断したもの)
| 項目 | 判断 | 理由 |
| --- | --- | --- |
| シェルとエンジンの IPC | ループバック WebSocket + JSON-RPC (操作) + MessagePack (ストリーム) | stdio パイプより複数接続 (CLI / テスト) が容易。gRPC は Chromium 側で重い |
| 部品の粒度 | plant / motor-driver / encoder / comm-link を分ける | 差し替えと挿入を manifest でできる単位にする |
| 配線の既定 latency | 1 ステップ | 実行順に依存しない。閉路を無条件に許せる |
| スケジューラのスレッド | 単一 | Wasmtime の Store と決定性。並列化は部品数が増えてから |
| 仮想 MCU の実行時間モデル | zero / fixed / fuel | 実機の計算遅れを模擬しつつ、既定は理想で比較しやすく |
| UI バンドルの配布 | アーティファクト (manifest の `ui.bundle`) | 部品に合わせた UI をクライアント本体の更新なしで配る |
| 同意の鍵 | artifact sha256 + capability | 別のコードに許可を引き継がない |
| 署名鍵の世代 | `keyId` で管理、公開鍵はクライアント同梱 | v1 では鍵更新 = 本体更新。KMS は後 |
| DB | SQLite (sqlx) | 小規模運用。抽象を保って PostgreSQL へ移行可能に |

## 未決事項
Q1〜Q11 はすべて裁定済み (D13〜D24 に反映)。実装中に新たに出た判断はこの表に追記し、裁定を仰ぎます。

| # | 項目 | 選択肢 | 影響 |
| --- | --- | --- | --- |
| (なし) | | | |

## マイルストーン (承認後)
| M | 内容 | 完了条件 |
| --- | --- | --- |
| M0 | 設計 (本 PR) | 設計資料の承認 |
| M1 ✅ | エンジン核 | ローカル manifest から plant / driver / encoder / comm-link (Rust 部品) + `fw-balance` (C 版と Rust 版、各 wasm と native) を組み立て、headless で倒立維持。決定性・性質・FW 同等性テストが `scripts/check.sh` で通る。**完了** (ip-model 12 / ipemu-core 8 / 結合 7 テスト) |
| M2 | シェル | Electron の汎用ブラウザ UI + 信頼 origin ブリッジ + `ipemu://` ハンドラ + シミュレータ UI (sim-view 移植)。IPC 契約の TS 型生成。wasm ホット差替 / native 再起動。VS の F5 手順がドキュメントどおりに動く |
| M3 ✅ | サーバ配信 + ポータル | ログイン、カタログ、起動 (ブリッジ / `ipemu://` チケット)、manifests / artifacts API、署名、エンジンの bootstrap とキャッシュ、ローカルカタログでのオフライン起動、telemetry。**完了** (`crates/ipemu-server`, `crates/ipemu-sign`, `crates/ipemu-engine/src/{bootstrap,cache}.rs`, `apps/web` のポータル。結合テスト: サーバ API 4 / bootstrap 2 (サーバから取得して倒立維持 → オフラインで同じ軌道)) |
| M4 ✅ | 管理画面 | 部品登録 (WIT 抽出)、manifest 編集と検証、承認 / 公開 / 公開範囲、ユーザとロール、API キー、監査。**完了** (`/api/admin` と `apps/web` の管理画面。capability 付き部品の describe と V7 の実物照合は M5 と合わせる) |
| M5 | capability | WASI p2 の sockets / filesystem を宣言 → 承認 → 同意で付与。同意 UI。拒否時の挙動 |
| M6 | 拡張 | fuel ベースの実行時間モデル、NetworkedSil の遅延実験との比較記録 (comm-link を使う)、6 プラットフォームの zip ビルド |
