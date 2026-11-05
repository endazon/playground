# WIT 定義 (草案)

パッケージ `ip:emu@0.1.0`。部品・ファームウェア・ホスト拡張の境界をここで定義します。
`wit-bindgen` で Rust / C のバインディングを生成し、エンジン側は `wasmtime::component::bindgen!` で読みます。

| ファイル | 内容 |
| --- | --- |
| `part.wit` | 部品が実装する `part` ワールドと、部品が使えるホスト機能 `host` |
| `firmware.wit` | ファームウェアが実装する `firmware` ワールドと HAL `hal` |
| `host-ext.wit` | capability として個別に付与するホスト拡張 (シリアルポートなど) |

設計の背景は [`../../Docs/03-component-model.md`](../../Docs/03-component-model.md) と
[`../../Docs/05-firmware.md`](../../Docs/05-firmware.md) を参照。
