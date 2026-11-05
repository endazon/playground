//! JSON-RPC 2.0 の封筒。本文 (メソッドごとの params / result) は [`crate::methods`]。

use crate::error::RpcError;
use crate::methods::RequestBody;
use crate::notify::NotificationBody;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 要求 id。数値か文字列 (JSON-RPC 2.0)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export)]
pub enum RequestId {
    Num(#[ts(type = "number")] i64),
    Str(String),
}

/// 要求 (シェル / CLI → エンジン)。`params` のない要求は `{}` を送る (省略・null も受理する)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Request {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(flatten)]
    pub body: RequestBody,
}

/// 応答。`result` と `error` は排他。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Response {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<RpcError>,
}

/// 通知 (エンジン → シェル)。id を持たない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Notification {
    pub jsonrpc: String,
    #[serde(flatten)]
    pub body: NotificationBody,
}

/// エンジンが起動直後に stdout の 1 行目へ出す接続情報。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Discovery {
    pub port: u16,
    pub token: String,
    pub pid: u32,
    pub ipc_version: u32,
    pub engine_version: String,
}

impl Request {
    pub fn new(id: impl Into<RequestId>, body: RequestBody) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id.into(),
            body,
        }
    }

    /// 生の JSON から組み立てる。`params` が省略 / null のメソッドは `{}` とみなす。
    /// 失敗の種別 (構文 / 未知メソッド / params 不正) を JSON-RPC のコードに写す。
    pub fn parse(text: &str) -> Result<Self, (Option<RequestId>, RpcError)> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| {
            (
                None,
                RpcError::new(crate::codes::PARSE_ERROR, e.to_string()),
            )
        })?;
        let id: Option<RequestId> = v
            .get("id")
            .filter(|i| !i.is_null())
            .and_then(|i| serde_json::from_value(i.clone()).ok());
        let Some(id) = id else {
            return Err((
                None,
                RpcError::new(crate::codes::INVALID_REQUEST, "id がありません"),
            ));
        };
        let method = v
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| {
                (
                    Some(id.clone()),
                    RpcError::new(crate::codes::INVALID_REQUEST, "method がありません"),
                )
            })?
            .to_owned();
        if !RequestBody::METHODS.contains(&method.as_str()) {
            return Err((Some(id), RpcError::method_not_found(&method)));
        }
        let params = match v.get("params") {
            None | Some(serde_json::Value::Null) => serde_json::json!({}),
            Some(p) => p.clone(),
        };
        let body: RequestBody =
            serde_json::from_value(serde_json::json!({ "method": method, "params": params }))
                .map_err(|e| (Some(id.clone()), RpcError::invalid_params(e)))?;
        Ok(Self {
            jsonrpc: "2.0".into(),
            id,
            body,
        })
    }
}

impl Response {
    pub fn ok<T: Serialize>(id: RequestId, result: &T) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(serde_json::to_value(result).expect("result は serialize 可能")),
            error: None,
        }
    }
    pub fn err(id: RequestId, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error),
        }
    }
    /// id が分からないエラー (構文エラー) 用。JSON-RPC では id を null にする
    pub fn err_no_id(error: RpcError) -> serde_json::Value {
        serde_json::json!({ "jsonrpc": "2.0", "id": null, "error": error })
    }
}

impl Notification {
    pub fn new(body: NotificationBody) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            body,
        }
    }
}

impl From<i64> for RequestId {
    fn from(v: i64) -> Self {
        RequestId::Num(v)
    }
}
impl From<&str> for RequestId {
    fn from(v: &str) -> Self {
        RequestId::Str(v.to_owned())
    }
}
