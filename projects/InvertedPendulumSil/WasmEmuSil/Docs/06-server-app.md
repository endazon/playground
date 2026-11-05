# 06. サーバアプリ (配信 + 管理画面)

## 役割
1. **ポータル**: 利用者がログインし、カタログから構成を選んで「起動」する入口。シェルでも通常のブラウザでも開ける
2. **配信**: 選ばれた manifest と、そこから参照されるアーティファクト (部品 / ファームウェア / UI バンドル) を返す
3. **管理**: 部品の登録、manifest の編集・検証・承認・公開・公開範囲、ユーザとロール、監査
4. **収集**: クライアントからの実行要約 (テレメトリ)。既定 on、クライアント設定で off にできる

制御には一切関与しません。サーバが落ちていても、キャッシュ済みのクライアントは動き続けます。

## 構成
```text
ipemu-server (Rust / axum / tokio)
  ├ api/v1         配信 API (セッショントークン、または headless 用クライアント API キー)
  ├ api/admin      管理 API (セッション Cookie、CSRF 対策、ロール検査)
  ├ /portal /admin 1 つの SPA (Vite + React + TypeScript) をロールで出し分けて配信
  ├ store/
  │   artifacts/   内容アドレス: <root>/sha256/ab/cd/<hex>  (イミュータブル)
  │   db.sqlite    sqlx: artifacts, manifests, manifest_revisions, visibility, users, sessions, api_keys, launches, approvals, telemetry, audit
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
| `GET` | `/api/v1/catalog` | ログイン中の利用者に公開された構成の一覧 `{id, name, description, defaultRevision, revisions[], requiresCapabilities}` |
| `POST` | `/api/v1/launch-tickets` | 外部ブラウザからの `ipemu://` 起動用の**短命・単回**チケットを発行 (ポータルの「起動」が呼ぶ)。エンジンは `POST /api/v1/launch-tickets/{ticket}/exchange` で短命トークンに替える |
| `GET` | `/api/v1/manifests/{id}?rev=n\|default` | 署名付き manifest。`ETag` は revision。公開範囲外なら 404 |
| `GET` | `/api/v1/artifacts/{name}/{version}` | バイナリ。`ETag: "<sha256>"`、`Cache-Control: immutable`、`Content-Digest`。`If-None-Match` 対応 |
| `GET` | `/api/v1/artifacts/by-hash/{sha256}` | 同上。キャッシュ修復用 |
| `POST` | `/api/v1/telemetry` | 実行要約 (開始 / 終了、FAULT 理由、オーバーラン数、capability 使用回数)。既定 on、クライアント設定で off |
| `POST` | `/api/v1/clients/hello` | エンジン起動時の生存通知 (`engineVersion, witVersions[], os, arch`)。管理画面の「クライアント」に出す |
| `GET` | `/healthz` | ヘルスチェック |

- 認証は `Authorization: Bearer <token>`。トークンは**利用者のセッション** (ポータルからシェル経由でエンジンに渡る) か、
  headless / CI 用に管理画面で発行する**クライアント API キー**のどちらか。どちらで起動したかは `launches` に残る。
- 外部ブラウザ (`ipemu://`) 経由ではブラウザの Cookie をアプリに渡せないため、単回チケット → 短命トークンの交換を挟む。
  チケットは URL に載るので 60 秒で失効し、1 回しか交換できない。
- 配信はすべて**読み取り専用**で、アーティファクトはハッシュで同定されるため CDN / プロキシキャッシュに載せられます。
- レート制限と転送量上限を持ちます (部品 1 個あたり既定 32MB)。

## ポータル (利用者向け)
| 画面 | できること |
| --- | --- |
| ログイン | ローカルアカウント。初回はパスワード変更を強制 |
| カタログ | 公開範囲内の構成を一覧。名前・説明・要求 capability・既定 revision。検索とタグ |
| 構成の詳細 | トポロジ図 (部品と配線)、パラメータの要約、必要な capability と承認者・理由 |
| **起動** | シェル内なら `window.ipemu.launch(...)`、通常のブラウザなら `ipemu://launch?...` へ遷移。クライアント未インストールなら zip の案内を出す |
| 自分の起動履歴 | いつ・どの端末で・どの構成を起動し、どう終わったか (telemetry) |

## 管理画面 (管理者向け)
| 画面 | できること |
| --- | --- |
| 部品 (Artifacts) | wasm のアップロード → サーバが WIT を抽出し、ワールド・ポート (`describe` を Wasmtime で実行して取得)・import する `wasi:*` を表示。同名同版の再登録は拒否 (イミュータブル) |
| 構成 (Manifests) | 一覧 / 下書き作成 / JSON エディタ (CodeMirror 6 + JSON Schema 補完) / **トポロジ図** (部品と配線の可視化、クライアント UI と同じ描画部品) / 検証結果の表示 / 承認 / 公開 / 巻き戻し |
| capability 承認 | 下書きが宣言する capability の一覧 (instance ごと)。承認者・日時・理由を記録。承認は本文ハッシュに紐づく |
| 公開範囲 | 構成ごとに「全利用者」または「グループ」。カタログの既定 revision の切替 (巻き戻し) |
| ユーザ | アカウントとロール、グループ、headless 用クライアント API キーの発行と失効 |
| クライアント | hello してきたエンジン一覧 (版・OS・アーキ・最終接続)、起動中の構成 (telemetry から) |
| 監査 | すべての変更操作 (誰が・いつ・何を) と、起動・終了・FAULT の履歴 |

### 画面フレームワーク
Vite + **React** + TypeScript。ポータルと管理画面は `apps/web` の 1 つの SPA で、ロールに応じてルートを出し分けます。
状態取得は TanStack Query、フォームは React Hook Form、JSON エディタは CodeMirror 6。
シミュレータ UI (配信される UI バンドル) と同じ構成にし、トポロジ図やスコープの描画部品を `apps/ui-kit` で共有します。
NetworkedSil の `sim-view.js` (Canvas / three.js) はフレームワーク非依存のまま移植します。
Svelte / Vue も候補でしたが、「2 つの UI で同じ部品を使う」「人手と情報量」を優先しました ([08-decisions.md](08-decisions.md))。

## 認証・認可
| 主体 | 方式 | 権限 |
| --- | --- | --- |
| 利用者・管理者 | ローカルアカウント (argon2id) + セッション Cookie。OIDC は拡張点として残し v1 では作らない | ロール: `user` (ポータルと起動) / `viewer` (管理画面の閲覧) / `editor` (下書き) / `approver` (承認・公開・公開範囲) / `admin` (鍵・ユーザ) |
| エンジン (利用者起動) | 利用者のセッショントークン (シェル経由)、または単回チケットから得た短命トークン | その利用者に公開された構成の取得と telemetry 送信 |
| エンジン (headless / CI) | クライアント API キー | 発行時に指定した構成の取得と telemetry 送信 |

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
- 設定は環境変数 (`IPEMU_STORE`, `IPEMU_DB`, `IPEMU_SIGNING_KEY`, `IPEMU_ADMIN_BOOTSTRAP_PASSWORD`, `IPEMU_PUBLIC_URL`)。
  `IPEMU_PUBLIC_URL` は `ipemu://` リンクと、シェルが信頼する origin の照合に使う。
- CI は置かない。`scripts/check.sh` (cargo fmt / clippy / test、pnpm lint / test、markdownlint) を手元と手順書で回す。
- ログは構造化 (tracing + JSON)。NetworkedSil の `07-logging.md` と同じく「動いているか」が追える粒度を目標にします。
