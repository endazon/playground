//! ipemu-sign — manifest の署名まわり。サーバ (署名) とエンジン (検証) が同じ規則を使う。
//!
//! - 本文の正規化は RFC 8785 (JCS) で、実装は `ipemu_core::jcs` (D31 / D38: capabilitiesHash と同じもの。二重実装しない)。
//!   `signature` を除いた manifest 全体を正規化して Ed25519 で署名する
//! - `approval.capabilitiesHash` は `ipemu_core::capabilities_hash` ([`capabilities_hash`] はその Value 版の入口)
//! - 鍵の配布形式は **raw 32 バイトの hex** を JSON に包んだもの ([`PublicKeyEntry`] / [`SigningKeyFile`])。
//!   PEM (PKCS#8) は ASN.1 の解釈が要り、クライアントに同梱する値としては hex の方が検査しやすいため (D37)

use base64::Engine as _;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ipemu_core::Manifest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("JSON の正規化に失敗: {0}")]
    Canonicalize(String),
    #[error("signature がありません")]
    MissingSignature,
    #[error("signature の形式が不正: {0}")]
    MalformedSignature(String),
    #[error("alg `{0}` は未対応 (ed25519 のみ)")]
    UnsupportedAlg(String),
    #[error("keyId `{0}` は信頼済み公開鍵にありません")]
    UnknownKey(String),
    #[error("署名が一致しません (keyId `{0}`)")]
    BadSignature(String),
    #[error("approval がありません")]
    MissingApproval,
    #[error("approval.capabilitiesHash が本文と一致しません (manifest {expected}, 実際 {actual})")]
    CapabilitiesHashMismatch { expected: String, actual: String },
    #[error("鍵の形式が不正: {0}")]
    MalformedKey(String),
}

/// RFC 8785 で正規化したバイト列 (`ipemu_core::jcs` に委譲)。
pub fn canonicalize(value: &Value) -> Result<Vec<u8>, SignError> {
    Ok(ipemu_core::jcs::canonicalize(value).into_bytes())
}

/// `approval.capabilitiesHash` (`ipemu_core::capabilities_hash` の定義: `{instance: Capabilities}` を型経由で
/// 正規化 → JCS → sha256。D31)。JSON 値から呼ぶための入口で、manifest として読めない本文はエラーにする。
pub fn capabilities_hash(manifest: &Value) -> Result<String, SignError> {
    let m: Manifest = serde_json::from_value(manifest.clone())
        .map_err(|e| SignError::Canonicalize(format!("manifest として読めません: {e}")))?;
    Ok(ipemu_core::capabilities_hash(&m))
}

/// 署名対象 (signature を除いた本文) の正規化バイト列。
pub fn signing_input(manifest: &Value) -> Result<Vec<u8>, SignError> {
    let mut body = manifest.clone();
    if let Some(o) = body.as_object_mut() {
        o.remove("signature");
    }
    canonicalize(&body)
}

/// 本文の sha256 (ETag や承認の紐付けに使う)。signature を除いて計算する。
pub fn body_sha256(manifest: &Value) -> Result<String, SignError> {
    Ok(hex::encode(Sha256::digest(signing_input(manifest)?)))
}

/// manifest に `signature` を付けて返す。既存の signature は捨てる。
pub fn sign_manifest(manifest: &Value, key: &SigningKey, key_id: &str) -> Result<Value, SignError> {
    let input = signing_input(manifest)?;
    let sig: Signature = key.sign(&input);
    let mut out = manifest.clone();
    let o = out
        .as_object_mut()
        .ok_or_else(|| SignError::Canonicalize("manifest がオブジェクトではありません".into()))?;
    o.insert(
        "signature".into(),
        serde_json::json!({
            "alg": "ed25519",
            "keyId": key_id,
            "value": base64::engine::general_purpose::STANDARD.encode(sig.to_bytes()),
        }),
    );
    Ok(out)
}

/// 検証に成功したときの情報。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub key_id: String,
    pub capabilities_hash: String,
}

/// 信頼済み公開鍵の集合 (keyId → 公開鍵)。クライアントに同梱 / 設定で渡す。
#[derive(Debug, Clone, Default)]
pub struct TrustedKeys {
    keys: BTreeMap<String, VerifyingKey>,
}

impl TrustedKeys {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&mut self, key_id: impl Into<String>, key: VerifyingKey) {
        self.keys.insert(key_id.into(), key);
    }
    pub fn insert_hex(&mut self, key_id: &str, public_hex: &str) -> Result<(), SignError> {
        self.insert(key_id, parse_public_hex(public_hex)?);
        Ok(())
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    pub fn len(&self) -> usize {
        self.keys.len()
    }
    pub fn key_ids(&self) -> impl Iterator<Item = &str> {
        self.keys.keys().map(String::as_str)
    }
    pub fn get(&self, key_id: &str) -> Option<&VerifyingKey> {
        self.keys.get(key_id)
    }
    /// `keyId=hex[,keyId=hex...]` (環境変数 / CLI 用)。
    pub fn parse_list(s: &str) -> Result<Self, SignError> {
        let mut t = Self::new();
        for item in s.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (id, hexs) = item
                .split_once('=')
                .ok_or_else(|| SignError::MalformedKey(format!("`{item}` は keyId=hex の形式")))?;
            t.insert_hex(id.trim(), hexs.trim())?;
        }
        Ok(t)
    }
    /// `{"keys":[{"keyId":..,"publicKey":..}]}` または `[...]` の JSON。
    pub fn from_json(json: &str) -> Result<Self, SignError> {
        let v: Value =
            serde_json::from_str(json).map_err(|e| SignError::MalformedKey(e.to_string()))?;
        let arr = match &v {
            Value::Array(a) => a.clone(),
            Value::Object(o) => o
                .get("keys")
                .and_then(|k| k.as_array())
                .cloned()
                .unwrap_or_default(),
            _ => return Err(SignError::MalformedKey("keys の形式が不正".into())),
        };
        let mut t = Self::new();
        for e in arr {
            let e: PublicKeyEntry =
                serde_json::from_value(e).map_err(|e| SignError::MalformedKey(e.to_string()))?;
            t.insert_hex(&e.key_id, &e.public_key)?;
        }
        Ok(t)
    }
    pub fn to_entries(&self) -> Vec<PublicKeyEntry> {
        self.keys
            .iter()
            .map(|(id, k)| PublicKeyEntry {
                key_id: id.clone(),
                alg: "ed25519".into(),
                public_key: hex::encode(k.to_bytes()),
            })
            .collect()
    }
}

/// 署名を検証し、approval.capabilitiesHash が本文と一致することを確かめる (V12)。
pub fn verify_manifest(manifest: &Value, trusted: &TrustedKeys) -> Result<Verified, SignError> {
    let sig = manifest
        .get("signature")
        .ok_or(SignError::MissingSignature)?;
    let alg = sig.get("alg").and_then(|v| v.as_str()).unwrap_or("");
    if alg != "ed25519" {
        return Err(SignError::UnsupportedAlg(alg.to_owned()));
    }
    let key_id = sig
        .get("keyId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SignError::MalformedSignature("keyId がありません".into()))?;
    let value = sig
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SignError::MalformedSignature("value がありません".into()))?;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|e| SignError::MalformedSignature(e.to_string()))?;
    let signature =
        Signature::from_slice(&raw).map_err(|e| SignError::MalformedSignature(e.to_string()))?;
    let key = trusted
        .get(key_id)
        .ok_or_else(|| SignError::UnknownKey(key_id.to_owned()))?;
    let input = signing_input(manifest)?;
    key.verify(&input, &signature)
        .map_err(|_| SignError::BadSignature(key_id.to_owned()))?;

    // 署名が正しくても、承認したときの capability と本文が食い違っていれば拒否する
    let expected = manifest
        .get("approval")
        .and_then(|a| a.get("capabilitiesHash"))
        .and_then(|v| v.as_str())
        .ok_or(SignError::MissingApproval)?;
    let actual = capabilities_hash(manifest)?;
    if expected != actual {
        return Err(SignError::CapabilitiesHashMismatch {
            expected: expected.to_owned(),
            actual,
        });
    }
    Ok(Verified {
        key_id: key_id.to_owned(),
        capabilities_hash: actual,
    })
}

/// 公開鍵 1 件 (クライアントに配る形)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicKeyEntry {
    pub key_id: String,
    #[serde(default = "ed25519_str")]
    pub alg: String,
    /// raw 32 バイトの hex
    pub public_key: String,
}

fn ed25519_str() -> String {
    "ed25519".into()
}

/// 署名鍵ファイル (`IPEMU_SIGNING_KEY` が指す JSON)。seed は raw 32 バイトの hex。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningKeyFile {
    pub key_id: String,
    #[serde(default = "ed25519_str")]
    pub alg: String,
    pub seed: String,
}

impl SigningKeyFile {
    pub fn generate(key_id: &str) -> Self {
        let key = SigningKey::generate(&mut rand_core::OsRng);
        Self {
            key_id: key_id.to_owned(),
            alg: "ed25519".into(),
            seed: hex::encode(key.to_bytes()),
        }
    }
    pub fn parse(json: &str) -> Result<Self, SignError> {
        let f: Self =
            serde_json::from_str(json).map_err(|e| SignError::MalformedKey(e.to_string()))?;
        if f.alg != "ed25519" {
            return Err(SignError::UnsupportedAlg(f.alg));
        }
        f.signing_key()?;
        Ok(f)
    }
    pub fn signing_key(&self) -> Result<SigningKey, SignError> {
        let raw = hex::decode(&self.seed).map_err(|e| SignError::MalformedKey(e.to_string()))?;
        let arr: [u8; 32] = raw
            .try_into()
            .map_err(|_| SignError::MalformedKey("seed は 32 バイトの hex".into()))?;
        Ok(SigningKey::from_bytes(&arr))
    }
    pub fn public_entry(&self) -> Result<PublicKeyEntry, SignError> {
        Ok(PublicKeyEntry {
            key_id: self.key_id.clone(),
            alg: "ed25519".into(),
            public_key: hex::encode(self.signing_key()?.verifying_key().to_bytes()),
        })
    }
}

pub fn parse_public_hex(s: &str) -> Result<VerifyingKey, SignError> {
    let raw = hex::decode(s.trim()).map_err(|e| SignError::MalformedKey(e.to_string()))?;
    let arr: [u8; 32] = raw
        .try_into()
        .map_err(|_| SignError::MalformedKey("公開鍵は 32 バイトの hex".into()))?;
    VerifyingKey::from_bytes(&arr).map_err(|e| SignError::MalformedKey(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "manifestVersion": 1, "id": "x", "revision": 2, "name": "n",
            "scheduler": {"baseStepNs": 1000000},
            "artifacts": {"p@1.0.0": {"url": "/api/v1/artifacts/p/1.0.0", "sha256": "0".repeat(64)}},
            "components": [
                {"instance": "b", "kind": "builtin:snapshot", "params": {}},
                {"instance": "a", "artifact": "p@1.0.0", "capabilities": {"net": {"tcpConnect": ["h:1"]}}}
            ],
            "wires": []
        })
    }

    #[test]
    fn jcs_sorts_keys_and_is_stable() {
        let a =
            canonicalize(&json!({"b": 1, "a": [1.5, "x"], "c": {"z": null, "y": true}})).unwrap();
        assert_eq!(
            String::from_utf8(a).unwrap(),
            r#"{"a":[1.5,"x"],"b":1,"c":{"y":true,"z":null}}"#
        );
    }

    #[test]
    fn sign_verify_roundtrip_and_tamper_detection() {
        let kf = SigningKeyFile::generate("t-1");
        let key = kf.signing_key().unwrap();
        let mut body = sample();
        let ch = capabilities_hash(&body).unwrap();
        body["approval"] = json!({"approvedBy": "u", "approvedAt": "2026-01-01T00:00:00Z", "capabilitiesHash": ch});
        let signed = sign_manifest(&body, &key, "t-1").unwrap();

        let mut trusted = TrustedKeys::new();
        trusted
            .insert_hex("t-1", &kf.public_entry().unwrap().public_key)
            .unwrap();
        let v = verify_manifest(&signed, &trusted).unwrap();
        assert_eq!(v.key_id, "t-1");

        // 本文の改変
        let mut t = signed.clone();
        t["name"] = json!("evil");
        assert!(matches!(
            verify_manifest(&t, &trusted),
            Err(SignError::BadSignature(_))
        ));

        // 未知の鍵
        let other =
            TrustedKeys::parse_list(&format!("t-2={}", kf.public_entry().unwrap().public_key))
                .unwrap();
        assert!(matches!(
            verify_manifest(&signed, &other),
            Err(SignError::UnknownKey(_))
        ));

        // capability だけ書き換えて署名し直しても、approval のハッシュと食い違えば拒否
        let mut c = body.clone();
        c["components"][1]["capabilities"]["net"]["tcpConnect"] = json!(["evil:9"]);
        let resigned = sign_manifest(&c, &key, "t-1").unwrap();
        assert!(matches!(
            verify_manifest(&resigned, &trusted),
            Err(SignError::CapabilitiesHashMismatch { .. })
        ));
    }

    #[test]
    fn capabilities_hash_matches_core_and_binds_instance() {
        let a = sample();
        let m: Manifest = serde_json::from_value(a.clone()).unwrap();
        assert_eq!(
            capabilities_hash(&a).unwrap(),
            ipemu_core::capabilities_hash(&m)
        );
        let mut b = sample();
        b["name"] = json!("other");
        assert_eq!(
            capabilities_hash(&a).unwrap(),
            capabilities_hash(&b).unwrap()
        );
        // 既定値の明示 (clock: false) では変わらない (型経由の正規化)
        let mut d = sample();
        d["components"][1]["capabilities"]["clock"] = json!(false);
        assert_eq!(
            capabilities_hash(&a).unwrap(),
            capabilities_hash(&d).unwrap()
        );
        let mut c = sample();
        c["components"][1]["instance"] = json!("z");
        assert_ne!(
            capabilities_hash(&a).unwrap(),
            capabilities_hash(&c).unwrap()
        );
    }

    #[test]
    fn trusted_keys_json_roundtrip() {
        let kf = SigningKeyFile::generate("k");
        let entries = vec![kf.public_entry().unwrap()];
        let json = serde_json::to_string(&json!({"keys": entries})).unwrap();
        let t = TrustedKeys::from_json(&json).unwrap();
        assert_eq!(t.to_entries(), entries);
    }
}
