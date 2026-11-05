//! ipemu-ipc — シェル (Electron) ⇄ エンジン (ipemu-engine) の IPC 契約。
//!
//! - 文字列フレーム: JSON-RPC 2.0 の要求 / 応答 / 通知 ([`rpc`])
//! - バイナリフレーム: 先頭 1 バイトがチャネル種別、残りが MessagePack ([`frame`])
//!
//! ここにある型が契約のすべてで、`cargo test -p ipemu-ipc` で `bindings/` に TypeScript 型を書き出す
//! (`scripts/gen-ipc-types.sh`)。シェルを Electron 以外に替えても、この契約を満たせば動く。
//! メソッド一覧は `Docs/07-client-app.md` の表と 1 対 1 に対応する。

pub mod error;
pub mod frame;
pub mod methods;
pub mod notify;
pub mod rpc;

pub use error::{codes, RpcError};
pub use frame::{decode_frame, encode_frame, FrameKind, SchedulerStats, ScopeFrame, StateFrame};
pub use methods::*;
pub use notify::*;
pub use rpc::{Discovery, Notification, Request, RequestId, Response};

/// 契約の版。互換性のない変更で上げ、`session.describe` の `engine.ipcVersion` に載せる。
pub const IPC_VERSION: u32 = 1;
