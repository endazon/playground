//! ipemu-engine — manifest からエミュレータを組み立てて動かす。
//!
//! M1: ローカルの manifest (署名なし・`path` 参照) を読み、部品 (wasm) と組み込み部品を実体化し、
//! ファームウェアを装着して headless で回す。
//! M3: `bootstrap` がサーバから署名付き manifest とアーティファクトを取り寄せ、キャッシュ経由で同じ `Session` に渡す。
//! M5: capability の計画と同意。IPC・UI は M2。

pub mod bootstrap;
pub mod cache;
pub mod capability;
pub mod consent;
pub mod recorder;
pub mod session;

pub use bootstrap::{bootstrap, BootstrapOptions, Bootstrapped, Credential, Telemetry};
pub use cache::{Cache, LocalCatalog};
pub use capability::{CapabilityPlan, DataDirs};
pub use consent::{ConsentOutcome, ConsentProvider, ConsentStore, FileConsent, NoConsent};
pub use recorder::{Recorder, TraceHash};
pub use session::{ArtifactResolver, FirmwareSource, LaunchError, Session, SessionOptions};
