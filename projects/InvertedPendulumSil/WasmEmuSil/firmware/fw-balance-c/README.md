# fw-balance-c

倒立振子の制御ファームウェア (C 版)。`src/fw_balance.c` は HAL (`abi/ip_hal.h`) だけに依存し、
同じソースから次の 2 形をビルドします。

| 形 | 作り方 | 用途 |
| --- | --- | --- |
| wasm (配布形・正) | `./build.sh` → `build/firmware/fw-balance-c.wasm` | 全 OS / アーキで同一。エンジンが Wasmtime で実行し fuel で実行時間を見積もる |
| native (デバッグ形) | `./build.sh` または CMake → `fw-balance-c.{so,dylib,dll}` | Visual Studio / LLDB でブレークポイントを置く |

## wasm 形のビルドの中身
wasi-sdk を使わず **freestanding** でビルドします (libc を持ち込まない = WASI を import しない)。

1. `wit-bindgen c abi/wit --world firmware` の生成物 (`gen/`、コミット済み。WIT を変えたら再生成)
2. `clang --target=wasm32-unknown-unknown -nostdlib -ffreestanding` で `src/*.c` と `gen/firmware.c` をコンパイル
3. 数学関数は `crates/ip-cmath` (Rust の `libm` crate を C ABI で公開した staticlib) をリンク。
   Rust 版ファームウェアと同じ実装なので、wasm 同士で結果が一致する
4. `malloc` / `memcpy` 等は `src/libc_min.c` の極小実装 (バンプアロケータ)
5. `gen/firmware_component_type.o` をリンクしてから `wasm-tools component new` でコンポーネントにする

## Visual Studio でのデバッグ (Windows)
1. `IPEMU_ENGINE` 環境変数に `ipemu-engine.exe` のパスを設定する
2. Visual Studio でこのフォルダを開く (CMake プロジェクトとして認識される)
3. `src/fw_balance.c` にブレークポイントを置き、起動項目「ipemu-engine (native firmware, headless)」で F5

ブレークで止まっている間は仮想時間も止まるので、再開しても保護は鳴りません。
