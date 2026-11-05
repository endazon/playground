//! JSON-RPC のエラー。標準コードに加え、エンジン固有のコードを -32000 番台に置く。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// エラーコード。TS 側は `codes` を参照して分岐する (文字列比較をしない)。
pub mod codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    /// エンジン内部の失敗 (manifest 検証、実体化、step の失敗など)。`message` に理由
    pub const ENGINE_ERROR: i32 = -32000;
    /// セッションがない (先に `session.launch` が要る)
    pub const NO_SESSION: i32 = -32001;
    /// 操作は受理したがエンジンの再起動が必要 (native FW など)。応答の `data.restartRequired = true`
    pub const RESTART_REQUIRED: i32 = -32002;
    /// 後続マイルストーンの機能 (サーバ配信 = M3、capability = M5)
    pub const NOT_IMPLEMENTED: i32 = -32003;
    /// トークン不一致
    pub const UNAUTHORIZED: i32 = -32004;
    /// 部品 / ポート / slot / requestId が見つからない
    pub const NOT_FOUND: i32 = -32005;
    /// 現在の実行状態では受け付けない操作 (例: FAULT 中の step)
    pub const INVALID_STATE: i32 = -32006;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, thiserror::Error)]
#[ts(export)]
#[error("rpc error {code}: {message}")]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub data: Option<serde_json::Value>,
}

impl RpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
    pub fn engine(e: impl std::fmt::Display) -> Self {
        Self::new(codes::ENGINE_ERROR, e.to_string())
    }
    pub fn no_session() -> Self {
        Self::new(
            codes::NO_SESSION,
            "セッションがありません (先に session.launch を送ってください)",
        )
    }
    pub fn invalid_params(e: impl std::fmt::Display) -> Self {
        Self::new(codes::INVALID_PARAMS, e.to_string())
    }
    pub fn not_found(what: impl std::fmt::Display) -> Self {
        Self::new(codes::NOT_FOUND, what.to_string())
    }
    pub fn not_implemented(what: impl std::fmt::Display) -> Self {
        Self::new(codes::NOT_IMPLEMENTED, what.to_string())
    }
    pub fn invalid_state(what: impl std::fmt::Display) -> Self {
        Self::new(codes::INVALID_STATE, what.to_string())
    }
    pub fn method_not_found(m: &str) -> Self {
        Self::new(codes::METHOD_NOT_FOUND, format!("未知のメソッド `{m}`"))
    }
}

impl From<anyhow::Error> for RpcError {
    fn from(e: anyhow::Error) -> Self {
        RpcError::engine(format!("{e:#}"))
    }
}
