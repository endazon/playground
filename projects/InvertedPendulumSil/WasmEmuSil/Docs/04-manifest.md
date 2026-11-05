# 04. 構成情報 (manifest)

## 役割
manifest は「どの部品を、どのパラメータで、どう配線し、どの周期で回し、どの capability を許すか」を記した JSON です。
サーバが版管理・承認・署名し、クライアントは署名を検証してから組み立てます。
スキーマは [`../schemas/emulator-manifest.schema.json`](../schemas/emulator-manifest.schema.json)、
サンプルは [`../schemas/examples/cartpole-lqr.manifest.json`](../schemas/examples/cartpole-lqr.manifest.json) です。

## 構造
```jsonc
{
  "manifestVersion": 1,
  "id": "cartpole-lqr-default",          // 安定な識別子。カタログと公開範囲はこの id 単位
  "revision": 3,                          // 公開のたびに +1。クライアントは id + revision を記録する
  "name": "台車型倒立振子 / LQR 既定構成",
  "scheduler": { "baseStepNs": 1000000, "pacing": "realtime", "maxSlipMs": 200, "snapshotDivider": 20 },
  "artifacts": {                          // 名前 → 取得先と内容ハッシュ。部品もファームウェアも UI もここ
    "plant-cartpole@1.2.0": { "sha256": "…", "url": "/api/v1/artifacts/plant-cartpole/1.2.0", "size": 184320 }
    // ローカル開発用 manifest では "path": "../build/components/plant-cartpole.wasm" と sha256 省略が許される
    // (エンジンの --no-signature-check。配信用では V2 がエラーにする)
  },
  "components": [
    { "instance": "plant", "artifact": "plant-cartpole@1.2.0", "params": { "pendulumLength": 1.0 } },
    { "instance": "mcu", "kind": "builtin:cpu-virtual",
      "params": { "tickHz": 1000, "timingModel": { "kind": "fuel", "nsPerFuel": 2.0 } },
      "firmware": { "slot": "balance", "world": "ip:emu/firmware@0.1.0",
                    "default": "fw-balance@0.4.1", "allowOverride": ["wasm", "native"] },
      "capabilities": { "net": { "tcpConnect": ["192.168.10.5:502"] } } }
  ],
  "wires": [ { "from": "plant.theta", "to": "enc-pend.angle" }, { "from": "mcu.pwm0", "to": "driver.duty", "latency": 0 } ],
  "ui": { "bundle": "ui-cartpole@2.0.0", "views": ["2d", "3d", "scope"] },
  "approval": { "approvedBy": "admin@example", "approvedAt": "2026-11-05T09:00:00Z", "capabilitiesHash": "…" },
  "signature": { "alg": "ed25519", "keyId": "srv-2026", "value": "base64…" }
}
```

## 検証規則
検証は `ipemu-core::manifest::validate` に集約し、**サーバ (登録・公開時) とクライアント (起動時) で同じコード**を走らせます。
結果は `Error` (拒否) と `Warning` (許可するが表示) の 2 段です。

| # | 規則 | 段 |
| --- | --- | --- |
| V1 | JSON Schema に適合する | Error |
| V2 | `components[].artifact` が `artifacts` に存在し、`sha256` が 64 桁 hex (ローカル開発モードでは欠落を Warning に緩める)、`url` か `path` がある | Error |
| V3 | 各部品の WIT ワールドが `ip:emu/part` (または `firmware`) を実装している | Error |
| V4 | 配線の `from` / `to` が存在するポートで、方向 (出力 → 入力) が正しい | Error |
| V5 | 同じ入力ポートに複数の配線がない | Error |
| V6 | `latency: 0` の配線だけで閉路がない | Error |
| V7 | 部品の import する `wasi:*` が、その instance の `capabilities` で宣言されている | Error |
| V8 | `capabilities` で宣言したものが部品の import に存在しない (過剰宣言) | Warning |
| V9 | 単位 (`unit`) が配線の両端で一致しない | Warning |
| V10 | `wasi:clocks` / `wasi:random` を与える部品がある (決定性の保証外) | Warning |
| V11 | `rateDivider` と `tickHz` が `baseStepNs` の整数倍で表現できる | Error |
| V12 | `signature` が公開鍵で検証でき (クライアントのみ)、`approval.capabilitiesHash` が capability 宣言と一致する (`approval` があれば常に。M5 で実装) | Error |

V3 / V7 / V8 は wasm バイナリの WIT を読む必要があるため、サーバはアップロード時に WIT を抽出して
メタデータに保存し、manifest 検証ではそれを参照します。クライアントはコンパイル時に実物と再照合します。
import の抽出は `ipemu_core::imports::component_imports(&[u8])` (M5。下の「実装 (M5)」参照)。

## capability 宣言
```jsonc
"capabilities": {
  "net":   { "tcpConnect": ["host:port", "*.example.local:8080"], "tcpListen": ["127.0.0.1:9000"], "udp": [] },
  "fs":    [ { "path": "logs", "mode": "rw" } ],          // クライアントの data dir 配下の相対パスに限る
  "clock": false,                                          // true で wasi:clocks を与える (決定性の保証外)
  "random": false,
  "hostExt": ["serial-port"]                               // ip:emu/host-ext の個別インターフェース
}
```
- 宣言は **instance 単位**です。同じ部品でも用途により許可が違うからです。
- `fs.path` は絶対パスを禁止し、クライアントのデータディレクトリ配下に閉じます。
- 宣言は `approval.capabilitiesHash` に含めて署名されるため、承認後に書き換えると V12 で落ちます。
- クライアント側の同意は `(artifact sha256, capability 文字列)` を鍵に記憶します。部品を更新すると再同意になります。
  これは意図した挙動です (別のコードに同じ許可を引き継がない)。

### 実装 (M5)
- 宣言は `ipemu_core::capability::Capabilities` (serde) として manifest に型付きで載る (`components[].capabilities`)。
  `net.tcpConnect / tcpListen / udp` の要素は `HostPort` (IP リテラル・ホスト名・`*.` サブドメイン) として解釈時に検査する。
  `fs[].path` は相対パスで `..` を含まないこと (`FsGrant::is_confined_path`)。`hostExt` は `KNOWN_HOST_EXT` (現在 `serial-port`) のみ。
- **capability 文字列** (同意の鍵・UI 表示・`--consent-file` に使う 1 件ずつの表現):
  `net.tcpConnect:<host:port>` / `net.tcpListen:<host:port>` / `net.udp:<host:port>` / `fs:<ro|rw>:<path>` / `clock` / `random` / `hostExt:<name>`。
  `Capabilities::items()` が宣言順に展開する。
- **import の抽出** (`ipemu_core::imports::component_imports`): `wasmparser` でコンポーネントの最上位 import 名を読む
  (`wasm-tools component wit` の import 行と同じ集合)。入れ子のモジュール / コンポーネントは見ない。
  関数もリソースも持たない型だけの instance import (例: filesystem が使う `wasi:clocks/wall-clock` の `datetime`) は
  `typesOnly = true` とし、Wasmtime も Linker 定義を要求しないので V7 / V8 の対象外。
- **V7 / V8** は `Capabilities::check_imports(&[ImportInfo])` に集約した。対応表:

  | import | 必要な宣言 |
  | --- | --- |
  | `wasi:sockets/tcp*` | `net.tcpConnect` か `net.tcpListen` |
  | `wasi:sockets/udp*` | `net.udp` |
  | `wasi:sockets/network`, `instance-network`, `ip-name-lookup` | `net` のいずれか |
  | `wasi:filesystem/*` | `fs` |
  | `wasi:clocks/*` (関数を持つもの) | `clock: true` |
  | `wasi:random/*` | `random: true` |
  | `wasi:io/*` | `net` / `fs` / `clock` のいずれか (土台として付与) |
  | `ip:emu/<name>` (host-ext.wit) | `hostExt: ["<name>"]` |
  | `wasi:cli/*`、その他の `wasi:*`、未知の名前 | **常に Error** (提供しない) |

  V8 (過剰宣言) は `net` / `fs` / `clock` / `random` / `hostExt` の各宣言に対応する import がないときに Warning。
  `validate` の `DescResolver` に `imports()` が増え、`None` を返す文脈 (閉包だけの簡易 resolver) では V7 / V8 を省略する。
  サーバは登録時に抽出した import を保存して同じ関数を呼べばよい。
- **`approval.capabilitiesHash`** = `sha256(JCS({ "<instance>": <capabilities>, … }))` (hex 小文字)。
  - 対象は宣言が空でない instance のみ。値は `Capabilities` の serde 出力 (既定値のフィールドは省く) なので、
    `"clock": false` の有無のような表記差では変わらない。instance 名の順序にも依存しない。
  - JCS (RFC 8785) は `ipemu_core::jcs::canonicalize(&serde_json::Value) -> String` に実装した
    (キーは UTF-16 コード単位順、数値は ES6 表記)。manifest 署名 (M3) の入力正規化にも同じ関数を使える。
  - 計算は `ipemu_core::capability::capabilities_hash(&Manifest) -> String`、正規形の取得は `capabilities_document`。
    **サーバ (M3/M4) は承認時にこの関数で値を埋める。** クライアントは `validate` の V12 (ハッシュ部分) で再計算して照合し、
    不一致なら Error。署名 (Ed25519) の検証は V12 の残り半分で、M3 のブートストラップ側で行う。
- `approval` は `ipemu_core::capability::Approval` として型付き (`approvedBy / approvedAt / reason? / capabilitiesHash`)。

## 版と公開範囲
```text
manifest (id)  ──┬── revision 1  (公開済み)
                 ├── revision 2  (公開済み・カタログの既定)   ← 利用者がポータルで選んで起動
                 └── revision 3  (下書き → 検証 → 承認 → 公開)
```
- 下書きは何度でも保存でき、検証の結果が画面に出ます。
- 承認は capability を含む本文に対して行い、承認後の本文変更は新しい下書きになります。
- 公開で署名が付き、カタログに載ります。巻き戻しは「カタログの既定を古い revision に戻す」操作です。
- **公開範囲** (全利用者 / 特定グループ) と **カタログでの既定 revision** は manifest 本文ではなくサーバ側のメタデータです。
  本文に入れると署名のやり直しになるためです。
- 利用者はカタログから `(id, revision)` を選びます。通常は既定 revision だけを見せ、
  `editor` 以上のロールには過去 revision や未公開の下書きも「検証用に起動」できるようにします (署名はない旨を UI に表示)。
- クライアントは複数の `(id, revision)` をキャッシュに持てますが、同時に動かすのは 1 つです。

## 上書き (開発用)
ファームウェアの差し替えなど、開発者がローカルで manifest の一部を上書きできます。
実行中の差し替え (`firmware.load`) は wasm 形ならプラントの状態を保ったままホットリロード、
native 形はエンジン再起動 (プラントも初期状態) になります ([05-firmware.md](05-firmware.md))。

```bash
ipemu-engine --manifest ./dev/cartpole.manifest.json \
             --firmware balance=native:./build/fw_balance.dll \
             --param plant.pendulumLength=0.6 \
             --no-signature-check     # ローカルファイルの manifest のときだけ許す
```
上書きが効いていることは UI のヘッダに常時表示します (配布構成と開発構成の取り違えを防ぐため)。
