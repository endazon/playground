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
| D12 | UI フレームワーク | **Vite + React + TypeScript** (管理 SPA とシェル renderer で共通) | Svelte: 軽いが共有部品のエコシステム (CodeMirror / three.js 連携) の情報量で劣る。Vue: 同等だが人手の都合で React |

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

## 未決事項 (実装前に裁定が欲しいもの)
| # | 項目 | 選択肢 | 影響 |
| --- | --- | --- | --- |
| Q1 | クライアント本体の配布と署名 | (a) 未署名 zip / (b) OS ごとのコード署名 (Authenticode / notarization) | 配布経路と費用。v1 は (a) で、社内配布を想定 |
| Q2 | テレメトリの既定 | (a) 既定 on (要約のみ) / (b) 既定 off | 管理画面の「クライアント」タブの情報量 |
| Q3 | 管理者認証 | (a) ローカルアカウントのみ / (b) 最初から OIDC | 運用環境の IdP 有無 |
| Q4 | 対象アーキテクチャ | x64 のみ / + arm64 (Apple Silicon、Raspberry Pi) | Electron / エンジンのビルドマトリクス |
| Q5 | `comm-link` 部品の v1 同梱 | 含める / M6 まで送る | NetworkedSil の遅延実験との比較をいつ取るか |
| Q6 | ファームウェアの最初の実装 | C のみ / C + Rust の 2 本 | HAL 境界の 2 言語での検証を初回から行うか |

## マイルストーン (承認後)
| M | 内容 | 完了条件 |
| --- | --- | --- |
| M0 | 設計 (本 PR) | 設計資料の承認 |
| M1 | エンジン核 | ローカル manifest から plant / driver / encoder (Rust 部品) + `fw-balance` (C、wasm と native) を組み立て、headless で倒立維持。決定性テストと性質テストが通る |
| M2 | シェル | Electron + sim-view 移植。IPC 契約の TS 型生成。VS の F5 デバッグ手順がドキュメントどおりに動く |
| M3 | サーバ配信 | hello / manifests / artifacts API、署名、クライアントの bootstrap とキャッシュ、オフライン起動 |
| M4 | 管理 SPA | 部品登録 (WIT 抽出)、manifest 編集と検証、承認 / 公開、割当、監査 |
| M5 | capability | WASI p2 の sockets / filesystem を宣言 → 承認 → 同意で付与。同意 UI。拒否時の挙動 |
| M6 | 拡張 | fuel ベースの実行時間モデル、`comm-link`、NetworkedSil の遅延実験との比較記録 |
