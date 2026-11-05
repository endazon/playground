# WIT 定義

パッケージ `ip:emu@0.1.0`。部品・ファームウェア・ホスト拡張の境界をここで定義します。
`wit-bindgen` で Rust / C のバインディングを生成し、エンジン側は `wasmtime::component::bindgen!` で読みます。

| ファイル | 内容 |
| --- | --- |
| `part.wit` | 部品が実装する `part` ワールドと、部品が使えるホスト機能 `host` |
| `firmware.wit` | ファームウェアが実装する `firmware` ワールドと HAL `hal` |
| `host-ext.wit` | capability として個別に付与するホスト拡張 (シリアルポートなど) |

設計の背景は [`../../Docs/03-component-model.md`](../../Docs/03-component-model.md) と
[`../../Docs/05-firmware.md`](../../Docs/05-firmware.md) を参照。

## WASI (M5)
| ファイル | 内容 |
| --- | --- |
| `capabilities.wit` | エンジンが capability として提供する WASI p2 の範囲 (`wasi-capabilities` world)、エンジン側 bindgen 用の world |
| `deps/*.wit` | WASI 0.2.12 の WIT (wasmtime-wasi 50 に同梱のものをそのまま置く)。`wasi:cli` は意図的に含めない |

部品 / ファームウェアが WASI を使うときは `wasi-capabilities` world を `wit-bindgen` で生成して import し、
manifest の `capabilities` に宣言する。宣言のない import はエンジンが link しないので実体化で失敗する
([`../../Docs/07-client-app.md`](../../Docs/07-client-app.md))。
