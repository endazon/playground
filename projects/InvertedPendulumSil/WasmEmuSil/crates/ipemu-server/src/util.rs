//! 小物: 時刻、乱数トークン、ハッシュ。

use rand::RngCore;
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub fn now_str() -> String {
    fmt(now())
}

pub fn fmt(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).expect("rfc3339")
}

pub fn after_secs(secs: i64) -> String {
    fmt(now() + Duration::seconds(secs))
}

pub fn parse(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}

pub fn is_expired(expires_at: &str) -> bool {
    match parse(expires_at) {
        Some(t) => t <= now(),
        None => true,
    }
}

/// URL に載せられる 32 バイトの乱数トークン (base64url, padding なし)。
pub fn random_token() -> String {
    use base64::Engine as _;
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// トークンは平文で保存しない。sha256 の hex を鍵にする。
pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn json_vec(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}
