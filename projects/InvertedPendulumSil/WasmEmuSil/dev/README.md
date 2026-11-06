# dev/ — ローカル開発用

| ファイル | 内容 |
| --- | --- |
| `cartpole.manifest.json` | ローカルビルドの部品 / FW を `path` で参照する manifest (署名なし。`--no-signature-check` で起動) |
| `cartpole-netdelay.manifest.json` | M6: MCU と駆動系の間の信号をすべて `comm-link` に通した構成 (片道 40 ms = 往復 80 ms、倒立付近から balance、2 s ごとの外乱)。NetworkedSil の遅延実験との比較用 ([Docs/10](../Docs/10-delay-comparison.md)) |
| `keys/` (git 管理外) | `ipemu-server keygen` が作る署名鍵。**秘密鍵はコミットしない** |

## 署名鍵の作り方 (M3)
```bash
cd projects/InvertedPendulumSil/WasmEmuSil
cargo run -q -p ipemu-server -- keygen --out dev/keys --key-id dev-2026
#   dev/keys/signing.json : {"keyId":"dev-2026","alg":"ed25519","seed":"<32 バイト hex>"}  (0600。サーバだけが読む)
#   dev/keys/public.json  : {"keys":[{"keyId":"dev-2026","alg":"ed25519","publicKey":"<32 バイト hex>"}]}  (クライアントに配る)
```
- 形式は **raw 32 バイトの hex を JSON に包んだもの** (PEM ではない)。クライアント同梱の
  `crates/ipemu-engine/keys/trusted-keys.json` に `public.json` の内容を入れてビルドすると、その鍵で署名された manifest だけを受け入れる。
- 開発中はビルドし直さずに、エンジンへ `--trusted-key dev-2026=<hex>` / `IPEMU_TRUSTED_KEYS=dev-2026=<hex>` / `IPEMU_TRUSTED_KEYS_FILE=dev/keys/public.json` で渡せる。
- `IPEMU_SIGNING_KEY` を設定せずにサーバを起動すると**起動ごとに一時鍵**で署名する (警告ログに公開鍵が出る)。公開済み manifest は再起動後に検証できなくなるので、評価以外では鍵ファイルを使うこと。

## サーバをローカルで動かして配信まで試す
```bash
./scripts/build-components.sh && ./scripts/build-firmware.sh
cargo run -q -p ipemu-server -- keygen --out dev/keys
IPEMU_STORE=dev/store IPEMU_SIGNING_KEY=dev/keys/signing.json IPEMU_ADMIN_BOOTSTRAP_PASSWORD=admin-pass-123 \
  IPEMU_WEB_DIR=apps/web/dist IPEMU_LOG_FORMAT=text cargo run -q -p ipemu-server -- serve
#   別ターミナル: (cd apps/web && pnpm install && pnpm build)  または  pnpm dev (5173 → 8080 にプロキシ)
```
ブラウザで <http://127.0.0.1:8080> を開き `admin` でログイン → 管理 → 部品に `build/components/*.wasm` (part) と
`build/firmware/*.wasm` (firmware) を登録 → 構成を作成し `dev/cartpole.manifest.json` の内容を貼り付けて保存 (サーバが `path` を
`url` / `sha256` に置き換える) → 別の approver ユーザで承認 → 公開 → ポータルのカタログに出る。
headless で起動するには管理画面の「クライアント」で API キーを発行して:
```bash
PUB=$(python3 -c "import json;print(json.load(open('dev/keys/public.json'))['keys'][0]['publicKey'])")
cargo run -q -p ipemu-engine -- catalog --server http://127.0.0.1:8080 --api-key ipk_...
cargo run -q -p ipemu-engine -- run --server http://127.0.0.1:8080 --api-key ipk_... --launch cartpole --trusted-key dev-2026=$PUB --run-seconds 20
cargo run -q -p ipemu-engine -- catalog --local        # キャッシュ済み構成 (サーバを止めても --offline で起動できる)
```

## 配布クライアント (zip) をローカルのサーバに繋ぐ
同梱の信頼鍵 `crates/ipemu-engine/keys/trusted-keys.json` は**空**なので、配布クライアントは
既定でどの署名も信頼しません。ローカルのサーバ (`keygen` で作った `dev-2026` などで署名) に
繋ぐときは、**アプリを起動する環境**に信頼鍵を渡します。シェルは `spawn` に `env` を渡さないので、
エンジンはアプリの環境変数をそのまま引き継ぎます。

```powershell
# 鍵ファイルを指す (鍵を作り直しても追随するのでこちらが楽)
setx IPEMU_TRUSTED_KEYS_FILE C:\...\WasmEmuSil\dev\keys\public.json
# または keyId=hex を直接
setx IPEMU_TRUSTED_KEYS dev-2026=$(node -p "require('./dev/keys/public.json').keys[0].publicKey")
```

`setx` は新しいプロセスにしか効きません。確実にやるなら端末から渡して起動します:

```powershell
$env:IPEMU_TRUSTED_KEYS_FILE = "C:\...\WasmEmuSil\dev\keys\public.json"
& "C:\...\dist\ipemu-client-0.1.0-windows-x64\ipemu.exe"
```

渡し忘れると、起動時に
`信頼済み公開鍵が 1 つもありません (keyId \`dev-2026\`)` で失敗します。
配布物に焼き込む場合は、ビルド前に `crates/ipemu-engine/keys/trusted-keys.json` へ
`dev/keys/public.json` の内容を入れます (本番の手順はこちら)。
