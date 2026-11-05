//! ipemu-engine — manifest からエミュレータを組み立てて動かす。
//!
//! M1: ローカルの manifest (署名なし・`path` 参照) を読み、部品 (wasm) と組み込み部品を実体化し、
//! ファームウェアを装着して headless で回す。
//! M2: `serve` で IPC サーバ (ループバック WebSocket) を立て、スケジューラ専用スレッド上の制御器
//! ([`control`]) が実時間ペーシング ([`pacing`]) とホット差替を行う。サーバからの取得 (bootstrap) は M3。

pub mod capability;
pub mod consent;
pub mod consent_ipc;
pub mod control;
pub mod ipc_server;
pub mod pacing;
pub mod recorder;
pub mod session;
pub mod triple_buffer;

pub use capability::{CapabilityPlan, DataDirs};
pub use consent::{ConsentOutcome, ConsentProvider, ConsentStore, FileConsent, NoConsent};
pub use recorder::{Recorder, TraceHash};
pub use session::{
    FirmwareSource, FirmwareSwap, LaunchError, ParamChange, Session, SessionOptions,
};
