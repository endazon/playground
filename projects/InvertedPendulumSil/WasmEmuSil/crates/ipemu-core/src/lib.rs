//! ipemu-core — 構成情報 (manifest) の解釈・検証、配線グラフ、スケジューラ、組み込み部品。
//!
//! サーバ (登録・公開時の検証) とクライアント (起動時の検証と実行) が同じクレートを使う。
//! Wasmtime にも OS にも依存しない。部品は [`Part`] トレイト、ファームウェアは
//! [`hal::Firmware`] トレイトで抽象化し、実体は `ipemu-wasm` が与える。

pub mod builtins;
pub mod capability;
pub mod consent;
pub mod desc;
pub mod engine;
pub mod graph;
pub mod hal;
#[cfg(feature = "imports")]
pub mod imports;
pub mod jcs;
pub mod manifest;
pub mod validate;

pub use capability::{capabilities_hash, Capabilities, Capability, ImportInfo};
pub use consent::{consent_key, ConsentAnswer, ConsentRequest};
pub use desc::{PartDesc, PortDesc, PortKind};
pub use engine::{Engine, Part, StepError};
pub use manifest::Manifest;
pub use validate::{Diagnostics, Severity};
