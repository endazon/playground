//! ipemu-engine — manifest からエミュレータを組み立てて動かす。
//!
//! M1 の範囲: ローカルの manifest (署名なし・`path` 参照) を読み、部品 (wasm) と組み込み部品を実体化し、
//! ファームウェアを装着して headless で回す。サーバからの取得・IPC・UI は後続のマイルストーン。

pub mod recorder;
pub mod session;

pub use recorder::{Recorder, TraceHash};
pub use session::{FirmwareSource, Session, SessionOptions};
