//! 通知 (エンジン → シェル)。文字列フレームで送る低頻度のもの。高頻度の `state` / `scope` は [`crate::frame`]。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// `event` の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EventKind {
    /// 部品 / FW の `log`
    Log,
    /// 実行状態の遷移 (idle / running / fault)。`data` に RunStatus
    Run,
    /// step の失敗。`message` に理由
    Fault,
    /// ファームウェアの装着 / ホット差替 / 監視の検知
    Firmware,
    /// パラメータ変更 (部品の再 init)
    Param,
    /// セッションの作成 / 置き換え / 破棄。`data` に SessionDescription (破棄は null)
    Session,
    /// capability の使用
    Capability,
    /// 実時間ペーシングの時間スリップ
    Slip,
    /// エンジン自身の警告 (設定、監視ファイルの読めなさ など)
    Engine,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventNotice {
    #[ts(type = "number")]
    pub t_ns: i64,
    pub kind: EventKind,
    pub level: Level,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub instance: Option<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub data: Option<serde_json::Value>,
}

/// 未同意の capability 1 件 (`ipemu_core::consent::ConsentRequest` と同形。M5)。
/// シェルは専用ウィンドウで表示し、`consent.answer {requestId, grant, remember}` で返す。
/// 同じ (artifact, capability) を複数の instance が使う場合は 1 件にまとまり `instances` に併記される。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConsentRequest {
    /// 回答で参照する id。同意ストアの鍵 (`<sha256>:<capability>`) と同じ文字列
    pub request_id: String,
    /// この capability を要求する instance (先頭が代表)
    pub instances: Vec<String>,
    /// アーティファクト名 (`name@version`、ローカル上書きならパス)
    pub artifact: String,
    pub artifact_sha256: String,
    /// 例 `net.tcpConnect:192.168.10.5:502`
    pub capability: String,
    pub reason: String,
    pub manifest_id: String,
    pub revision: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BootstrapStage {
    Manifest,
    Signature,
    Artifacts,
    Compile,
    Validate,
    Consent,
    Instantiate,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BootstrapProgress {
    pub stage: BootstrapStage,
    pub message: String,
    /// 0..1。不明なら None
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fraction: Option<f64>,
    /// キャッシュ構成でのオフライン起動
    pub offline: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "method", content = "params")]
#[ts(export)]
pub enum NotificationBody {
    #[serde(rename = "event")]
    Event(EventNotice),
    #[serde(rename = "consent.required")]
    ConsentRequired(ConsentRequest),
    #[serde(rename = "bootstrap.progress")]
    BootstrapProgress(BootstrapProgress),
}
