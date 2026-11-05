//! ipemu-wasm — Wasmtime で部品 (`ip:emu/part-world`) とファームウェア (`ip:emu/firmware`) を動かす。
//! ネイティブ形のファームウェア (.so / .dll / .dylib) も同じ `Firmware` トレイトで扱う。

pub mod firmware_native;
pub mod firmware_wasm;
pub mod part;
pub mod runtime;

pub use firmware_native::NativeFirmware;
pub use firmware_wasm::WasmFirmware;
pub use part::WasmPart;
pub use runtime::Runtime;
pub use wasmtime;
