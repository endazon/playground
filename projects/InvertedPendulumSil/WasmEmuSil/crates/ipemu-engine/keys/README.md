# 同梱する信頼済み公開鍵

`trusted-keys.json` はエンジンにコンパイル時に埋め込まれ (`include_str!`)、サーバが manifest に付ける
Ed25519 署名の検証に使います。形式は `ipemu-server keygen` が出力する `public.json` と同じです。

```json
{ "keys": [ { "keyId": "srv-2026", "alg": "ed25519", "publicKey": "<raw 32 バイトの hex>" } ] }
```

- 本番のサーバ鍵を作ったら、その `public.json` の内容をここに追記して**クライアント本体をビルドし直します**
  (D-: 鍵の更新 = 本体の更新。KMS / 配布時の鍵取得は v1 では持たない)。
- 開発中は埋め込まずに、`IPEMU_TRUSTED_KEYS="dev-2026=<hex>"` か `--trusted-key dev-2026=<hex>`、
  または `IPEMU_TRUSTED_KEYS_FILE=dev/keys/public.json` で渡せます。
- 秘密鍵 (`signing.json`) は決してリポジトリに入れないでください (`dev/keys/` は .gitignore 済み)。
