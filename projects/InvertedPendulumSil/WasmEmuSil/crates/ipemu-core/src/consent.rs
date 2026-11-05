//! 同意 (consent) の型。エンジン (`ipemu-engine::consent`) が保存と問い合わせを行い、
//! シェル (M2) の IPC `consent.required` / `consent.answer` がこの型を運ぶ。
//!
//! 鍵は `sha256(artifact) + ":" + capability 文字列`。部品を更新すると鍵が変わり再同意になる
//! (別のコードに同じ許可を引き継がない。08-decisions.md)。

use serde::{Deserialize, Serialize};

/// 同意ストアの鍵。
pub fn consent_key(artifact_sha256: &str, capability: &str) -> String {
    format!("{}:{capability}", artifact_sha256.to_ascii_lowercase())
}

/// 鍵を `(sha256, capability)` に分解する。
pub fn split_consent_key(key: &str) -> Option<(&str, &str)> {
    let (sha, cap) = key.split_once(':')?;
    if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) || cap.is_empty() {
        return None;
    }
    Some((sha, cap))
}

/// 未同意の capability 1 件。同じ (artifact, capability) を複数の instance が使う場合は 1 件にまとめる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentRequest {
    /// 回答で参照する id。同意ストアの鍵と同じ文字列
    pub request_id: String,
    /// この capability を要求する instance (先頭が代表)
    pub instances: Vec<String>,
    /// アーティファクト名 (`name@version`、ローカル上書きならパス)
    pub artifact: String,
    pub artifact_sha256: String,
    /// capability 文字列 (例 `net.tcpConnect:192.168.10.5:502`)
    pub capability: String,
    /// 利用者向けの説明
    pub reason: String,
    pub manifest_id: String,
    pub revision: u32,
}

impl ConsentRequest {
    pub fn key(&self) -> &str {
        &self.request_id
    }
}

/// 同意ダイアログの回答。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentAnswer {
    pub request_id: String,
    pub grant: bool,
    /// true なら同意ストアに保存する (「記憶する」)。false は今回だけ
    #[serde(default)]
    pub remember: bool,
}

/// 同意ストアの 1 エントリ (`<data dir>/ipemu/consent.json` の値)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentEntry {
    pub granted: bool,
    /// RFC 3339 (UTC)
    pub at: String,
    pub manifest_id: String,
    pub revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remember_until: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_format() {
        let sha = "ab".repeat(32);
        let k = consent_key(&sha.to_ascii_uppercase(), "net.tcpConnect:127.0.0.1:9000");
        assert_eq!(k, format!("{sha}:net.tcpConnect:127.0.0.1:9000"));
        assert_eq!(
            split_consent_key(&k),
            Some((sha.as_str(), "net.tcpConnect:127.0.0.1:9000"))
        );
        assert!(split_consent_key("zz:clock").is_none());
        assert!(split_consent_key(&format!("{sha}:")).is_none());
    }

    #[test]
    fn request_json_shape() {
        let r = ConsentRequest {
            request_id: "k".into(),
            instances: vec!["mcu".into()],
            artifact: "fw@1.0.0".into(),
            artifact_sha256: "00".repeat(32),
            capability: "clock".into(),
            reason: "r".into(),
            manifest_id: "m".into(),
            revision: 1,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["requestId"], "k");
        assert_eq!(v["artifactSha256"], "00".repeat(32));
        let a: ConsentAnswer = serde_json::from_str(r#"{"requestId":"k","grant":true}"#).unwrap();
        assert!(a.grant && !a.remember);
    }
}
