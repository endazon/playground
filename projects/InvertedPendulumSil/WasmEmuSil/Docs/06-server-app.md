# 06. サーバアプリ (配信 + 管理画面)

## 役割
1. **配信**: クライアントに割当てた manifest と、そこから参照されるアーティファクト (部品 / ファームウェア / UI バンドル) を返す
2. **管理**: 部品の登録、manifest の編集・検証・承認・公開、capability の承認、クライアントへの割当、監査
3. **収集 (任意)**: クライアントからの実行要約 (テレメトリ) を受け取る

制御には一切関与しません。サーバが落ちていても、キャッシュ済みのクライアントは動き続けます。

## 構成
```text
ipemu-server (Rust / axum / tokio)
  ├ api/v1         配信 API (クライアント API キー)
  ├ admin/api      管理 API (セッション Cookie、CSRF 対策)
  ├ admin/         管理 SPA の静的配信 (Vite + React + TypeScript のビルド成果物)
  ├ store/
  │   artifacts/   内容アドレス: <root>/sha256/ab/cd/<hex>  (イミュータブル)
  │   db.sqlite    sqlx: artifacts, manifests, manifest_revisions, clients, assignments, approvals, audit
  └ keys/          Ed25519 署名鍵 (ファイル権限 0600、将来は KMS / HSM)
TLS 終端はリバースプロキシ (Caddy / nginx) に任せ、compose.yaml に同梱する
```

### なぜ Rust (axum) か
manifest 検証 (V1〜V11) と wasm の WIT 抽出を、クライアントと**同じ `ipemu-core` クレートで**行うためです。
サーバで通った構成がクライアントで落ちる事故を構造的に防げます。
ASP.NET Core 案との比較は [08-decisions.md](08-decisions.md)。

## 配信 API
| メソッド | パス | 内容 |
| --- | --- | --- |
| `POST` | `/api/v1/clients/hello` | クライアント登録・生存通知。`{clientId, engineVersion, witVersions[], os, arch}` → `{manifestId, revision, flags, serverTime}` |
| `GET` | `/api/v1/manifests/{id}?rev=n\|latest` | 署名付き manifest。`ETag` は revision |
| `GET` | `/api/v1/artifacts/{name}/{version}` | バイナリ。`ETag: "<sha256>"`、`Cache-Control: immutable`、`Content-Digest`。`If-None-Match` 対応 |
| `GET` | `/api/v1/artifacts/by-hash/{sha256}` | 同上。キャッシュ修復用 |
| `POST` | `/api/v1/telemetry` | 実行要約 (開始 / 終了、FAULT 理由、オーバーラン数、capability 使用回数)。無効化可能 |
| `GET` | `/healthz` | ヘルスチェック |

- 認証はクライアントごとの API キー (`Authorization: Bearer`)。キーは管理画面で発行・失効。
- 配信はすべて**読み取り専用**で、アーティファクトはハッシュで同定されるため CDN / プロキシキャッシュに載せられます。
- レート制限と転送量上限を持ちます (部品 1 個あたり既定 32MB)。

## 管理画面 (SPA)
| 画面 | できること |
| --- | --- |
| 部品 (Artifacts) | wasm のアップロード → サーバが WIT を抽出し、ワールド・ポート (`describe` を Wasmtime で実行して取得)・import する `wasi:*` を表示。同名同版の再登録は拒否 (イミュータブル) |
| 構成 (Manifests) | 一覧 / 下書き作成 / JSON エディタ (CodeMirror 6 + JSON Schema 補完) / **トポロジ図** (部品と配線の可視化、クライアント UI と同じ描画部品) / 検証結果の表示 / 承認 / 公開 / 巻き戻し |
| capability 承認 | 下書きが宣言する capability の一覧 (instance ごと)。承認者・日時・理由を記録。承認は本文ハッシュに紐づく |
| クライアント | 登録済みクライアント、最終 hello、エンジン版、割当 manifest (id + revision / latest)、API キーの発行と失効 |
| 監査 | すべての変更操作 (誰が・いつ・何を) と、テレメトリの要約 |

### 画面フレームワーク
Vite + **React** + TypeScript。状態取得は TanStack Query、フォームは React Hook Form、JSON エディタは CodeMirror 6。
クライアント UI (Electron renderer) と同じ構成にし、トポロジ図やスコープの描画部品を `apps/ui-kit` で共有します。
NetworkedSil の `sim-view.js` (Canvas / three.js) はフレームワーク非依存のまま移植します。
Svelte / Vue も候補でしたが、「2 つの UI で同じ部品を使う」「人手と情報量」を優先しました ([08-decisions.md](08-decisions.md))。

## 認証・認可
| 主体 | 方式 | 権限 |
| --- | --- | --- |
| 管理者 | ローカルアカウント (argon2id) + セッション Cookie。将来 OIDC | ロール: `viewer` / `editor` (下書き) / `approver` (承認・公開) / `admin` (鍵・ユーザ) |
| クライアント | API キー | 配信 API の読み取りと telemetry 送信のみ |

**編集と承認は別ロール**にします。capability の承認は「クライアントで何が動くか」を決める操作なので、
編集した本人が承認できる構成では承認の意味がなくなります。

## 署名
- manifest 本文 (JSON を RFC 8785 で正規化) に Ed25519 で署名し、`signature` に付けます。
- 公開鍵はクライアントに同梱し、`keyId` で世代管理します。鍵の更新はクライアント本体の更新を伴います (v1 の制約)。
- アーティファクトは manifest 内の sha256 で間接的に署名されます。個別署名は持ちません。

## 保存とバックアップ
- アーティファクトは内容アドレスで重複せず、削除しません (manifest から参照されなくなっても残す。巻き戻しのため)。
- SQLite はテスト・小規模運用向けです。同時書き込みが増えたら PostgreSQL に切り替えられるよう `sqlx` の抽象で書きます。
- バックアップは `store/` ディレクトリの丸ごとコピーで完結させます。

## 運用
- Docker イメージ 1 つ + リバースプロキシ。`compose.yaml` は NetworkedSil と同じ流儀 (ループバック公開、cap_drop、リソース上限)。
- 設定は環境変数 (`IPEMU_STORE`, `IPEMU_DB`, `IPEMU_SIGNING_KEY`, `IPEMU_ADMIN_BOOTSTRAP_PASSWORD`)。
- ログは構造化 (tracing + JSON)。NetworkedSil の `07-logging.md` と同じく「動いているか」が追える粒度を目標にします。
