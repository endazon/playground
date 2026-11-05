//! アプリケーション状態 (ハンドラ間で共有)。

use crate::auth::LoginThrottle;
use crate::config::Config;
use crate::store::ArtifactStore;
use ed25519_dalek::SigningKey;
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct Inner {
    pub config: Config,
    pub pool: SqlitePool,
    pub store: ArtifactStore,
    pub signing_key: SigningKey,
    pub key_id: String,
    pub throttle: LoginThrottle,
}

#[derive(Clone)]
pub struct AppState(pub Arc<Inner>);

impl std::ops::Deref for AppState {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.0
    }
}

impl AppState {
    pub fn public_key_entry(&self) -> ipemu_sign::PublicKeyEntry {
        ipemu_sign::PublicKeyEntry {
            key_id: self.key_id.clone(),
            alg: "ed25519".into(),
            public_key: hex::encode(self.signing_key.verifying_key().to_bytes()),
        }
    }
}
