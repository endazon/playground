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

## 実装 (M3/M4)
実体は [`../crates/ipemu-server`](../crates/ipemu-server) (Rust / axum / sqlx SQLite / argon2id / ed25519-dalek) と
[`../apps/web`](../apps/web) (Vite + React + TypeScript、TanStack Query、React Hook Form、CodeMirror 6)。
署名まわりは [`../crates/ipemu-sign`](../crates/ipemu-sign) にまとめ、サーバ (署名) とエンジン (検証) が同じクレートを使う。

### 設計からの差分と確定事項
| 項目 | 実装 | 備考 |
| --- | --- | --- |
| 認証 | **Bearer のみ** (`sess_` ログインセッション / `lt_` チケット交換トークン / `ipk_` API キー)。Cookie は使わない | D36。設計案の Cookie + CSRF から変えた理由: ポータル → シェル → エンジンへ渡す資格情報が同じトークンで済み、CSRF が構造的に消える。代償として SPA が sessionStorage にトークンを持つため XSS 耐性は HttpOnly Cookie に劣る。**緩和策**: (1) ログインセッションの寿命は 12 時間 (`session_ttl_secs`、再ログインで更新)、(2) SPA と API に厳格な CSP (`default-src 'self'; script-src 'self'` で inline script 禁止。`style-src` の `'unsafe-inline'` は CodeMirror の動的スタイルのため)、(3) 破壊的な管理操作 (承認・公開・公開範囲・既定 revision・ユーザ・API キー) はロール検査に加えて**ログインから 1 時間以内のセッション** (`Principal::require_fresh_login`、`403 reauth_required`) を要求し、古いトークンでは再ログインを促す |
| `/api/auth` | `POST login` / `POST logout` / `GET me` / `POST password` | 初回ログインは `mustChangePassword` を返し、SPA が変更画面へ誘導 |
| ログイン抑止 | ユーザ名ごとに連続失敗 5 回で 30 秒ロック (メモリ上)。ユーザ不在時もダミーのハッシュ検証で応答時間を揃える | |
| 配信 API | 表のとおり。追加: `GET /api/v1/keys` (公開鍵の参考表示)、`GET /api/v1/launches/mine` (自分の起動履歴) | `keys` は**信頼の根拠ではない** (信頼鍵はクライアント同梱 / 設定) |
| manifest 取得 | `GET manifests/{id}?rev=n\|default` は `ETag` (= 本文 sha256)、`X-Ipemu-Launch-Id` (起動履歴の ID)、`X-Ipemu-Signed`、`X-Ipemu-Revision` を返す。取得のたびに `launches` 行を作る | editor 以上は未公開 revision を署名なしで取れる (`X-Ipemu-Signed: false`)。エンジンは `--no-signature-check` が必要 |
| アーティファクト | `ETag: "<sha256>"`, `Cache-Control: private, max-age=31536000, immutable`, `Content-Digest: sha-256=:…:`、`If-None-Match` → 304。`application/wasm` / `application/gzip` (UI) | 認証必須なので `private`。CDN に載せるなら公開範囲の設計を足す |
| チケット | `POST launch-tickets {manifestId, revision?}` (利用者のみ) → `{ticket, expiresAt, launchUrl}`。`launchUrl = ipemu://launch?server=<IPEMU_PUBLIC_URL>&manifest=…&rev=n\|default&ticket=tk_…`。交換は認証なし・60 秒・1 回 (`UPDATE … WHERE used_at IS NULL` の原子性で単回性を担保) | 2 回目以降と失効は `410 Gone` |
| telemetry | `{launchId?, manifestId, revision, event, detail}`。`event` が `stopped/fault/error/aborted` なら起動履歴の終了を埋める | 自分の起動だけ更新できる |
| hello | `{clientId?, engineVersion, witVersions[], os, arch}` を `(clientId, 主体)` で upsert。応答に公開鍵一覧を添える | |
| 管理 API | `/api/admin/{artifacts,manifests,validate,users,groups,api-keys,audit,launches,clients}` | ロール検査は extractor (`AdminAuth`) + ハンドラ内の `require` |
| 部品登録 | multipart (`name`, `version`, `kind`, `file`)。import は `ipemu_core::imports::component_imports` (エンジンの V7 / V8 と同じ抽出。型だけの instance import は区別して保存)、export 名は `wasmparser` で読んで world を推定。実体化は **import に対応する wasi:* を deny-all 形 (`WasiGrant::deny_all_for`、D34) で link** して行い、part は describe()、firmware は world の適合を確認する。describe / init 中に接続・名前解決・ファイル操作はすべて拒否される | 同名同版は 409。UI は tar.gz の magic だけ検査 |
| 下書き保存 | 本文の `id` / `revision` をパスに揃え、`approval` / `signature` を落とし、`artifacts` を登録済みアーティファクトで補完 (`sha256` / `size` / `kind` / `url`、`path` は除去)。保存のたびに検証して `validation` に残す | 本文が別の sha256 を主張していれば 422 |
| 検証 | V1 (スキーマ) → `ipemu_core::validate` (V2〜V12)。V7 / V8 は `DescResolver::imports` に登録時の import 一覧を渡して core 側の規則で照合する (サーバ独自の規則は持たない) | 検証結果に部品の記述 (`descs`) を含め、トポロジ図の描画に使う |
| 状態遷移 | `draft` → (承認) `approved` → (公開) `published`。承認済み / 公開済みは編集不可で、新しい revision を作る。公開済みは削除不可 (巻き戻しは既定 revision の変更) | |
| 承認 | approver 以上、かつ**最終編集者と別人** (admin でも例外なし)。承認時に再検証し、`capabilitiesHash` を記録 | |
| 公開 | `approval` (`capabilitiesHash` は `ipemu_core::capabilities_hash`) を本文に付け、`ipemu_core::jcs` で正規化して Ed25519 署名。`signed_body` と `keyId` を保存し、既定 revision が未設定なら既定にする | 署名後の全文もスキーマで再検証 |
| 公開範囲 | `all` / `groups` (ユーザの `groups` と照合)。API キーは `allowedManifests` (`*` で全部) | 範囲外は 404 |
| ユーザ | ロールは集合 (`user/viewer/editor/approver/admin`)。自分の admin を外す / 自分を無効化する操作は拒否。無効化とパスワード再設定でセッションを全部消す | |
| 監査 | すべての変更操作 (`artifact.register`, `manifest.*`, `user.*`, `api_key.*`, `auth.login`, …) を `audit` に記録 | |
| 保存 | `IPEMU_STORE/artifacts/sha256/ab/cd/<hex>` (一時ファイル → rename)、`IPEMU_DB` (既定 `<store>/db.sqlite`、WAL)。マイグレーションは `migrations/` を起動時に適用 | |
| 鍵 | `IPEMU_SIGNING_KEY` が指す JSON (`{keyId, alg, seed}`、seed は raw 32 バイト hex)。未設定なら起動ごとに一時鍵 (警告)。`ipemu-server keygen --out dev/keys` で生成 ([`../dev/README.md`](../dev/README.md)) | PEM ではなく hex (D37) |
| SPA 配信 | `IPEMU_WEB_DIR` (既定 `apps/web/dist` があればそれ)。`/api` 以外は `index.html` にフォールバック。`assets/` は immutable | `X-Content-Type-Options: nosniff`, `Referrer-Policy: same-origin` |
| 起動 | `ipemu-server serve` (既定) / `ipemu-server keygen`。ログは tracing JSON (`IPEMU_LOG_FORMAT=text` でテキスト) | `Dockerfile` + `compose.yaml` (ループバック公開、cap_drop ALL、read_only、リソース上限) |

### SPA (`apps/web`)
- 1 つの SPA。`/login`、ポータル (`/` カタログ、`/m/:id` 詳細 = トポロジ図 + パラメータ要約 + 要求 capability + **起動**、`/me/launches`、`/me/password`)、
  管理 (`/admin/{artifacts,manifests,manifests/:id,manifests/:id/rev/:rev,approvals,users,clients,audit}`)。ルートはロールで出し分け (`viewer` 以上で管理、`admin` でユーザ)。
- 起動ボタン (`src/launch.ts`): `window.ipemu.launch({server, manifestId, revision, sessionToken})` があればそれ (M2 のシェルと合わせた引数)、
  なければ `POST /api/v1/launch-tickets` → `ipemu://launch?…` へ遷移。2.5 秒経ってもページが見えたままなら未インストールの案内を出す。
- トポロジ図 (`TopologyView`) は配線の向きから列を決める簡易レイアウト (閉路は DFS で切る)。M2 の `apps/ui-kit` に移す前提で React + SVG だけに依存。
- 単体テスト (vitest + jsdom): 起動 URL とブリッジ / チケットの分岐、ロール判定、カタログ検索、トポロジのレイアウトと描画。

### 結合テスト
- `crates/ipemu-server/tests/api.rs`: 登録 → 検証 → 承認 → 公開 → カタログ → 取得 → 署名検証、公開範囲外の 404、チケットの単回性と失効、
  ETag / If-None-Match、API キーの範囲と失効、ログイン抑止、壊れた manifest の拒否。
- `crates/ipemu-engine/tests/bootstrap.rs`: サーバを in-process で立て、`dev/cartpole.manifest.json` の部品と FW を登録・公開し、
  エンジンが API キーで取得して**倒立維持**まで動く。サーバを止めてもキャッシュから同じ軌道で起動する。改竄 / 未知鍵 / 無効資格情報 / 使用済みチケットの拒否。
