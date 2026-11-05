//! セッションの capability 計画: 宣言 (manifest) とアーティファクトの sha256 を結び付け、
//! 同意の問い合わせ ([`ConsentRequest`]) と実施 ([`WasiGrant`]) の両方を作る。

use ipemu_core::capability::Capabilities;
use ipemu_core::consent::{consent_key, ConsentRequest};
use ipemu_wasm::WasiGrant;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// クライアントのデータディレクトリ (同意ストアと fs capability の実体を置く)。
#[derive(Debug, Clone)]
pub struct DataDirs {
    pub root: PathBuf,
}

impl DataDirs {
    /// 優先順: 明示指定 > `IPEMU_DATA_DIR` > OS の既定 (Linux: `$XDG_DATA_HOME` or `~/.local/share`,
    /// macOS: `~/Library/Application Support`, Windows: `%APPDATA%`)。
    pub fn resolve(explicit: Option<&Path>) -> Self {
        let root = explicit
            .map(Path::to_path_buf)
            .or_else(|| std::env::var_os("IPEMU_DATA_DIR").map(PathBuf::from))
            .unwrap_or_else(default_data_dir);
        Self { root }
    }

    /// `<data dir>/ipemu/consent.json`
    pub fn consent_file(&self) -> PathBuf {
        self.root.join("ipemu").join("consent.json")
    }

    /// `fs` capability の実体: `<data dir>/ipemu/data/<path>`
    pub fn fs_root(&self) -> PathBuf {
        self.root.join("ipemu").join("data")
    }
}

fn default_data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    } else if cfg!(target_os = "macos") {
        home().join("Library").join("Application Support")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".local").join("share"))
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// capability を要求する instance 1 つ分。
#[derive(Debug, Clone)]
pub struct PlanEntry {
    pub instance: String,
    /// `name@version`、またはローカル上書きのパス
    pub artifact: String,
    pub artifact_sha256: String,
    pub capabilities: Capabilities,
}

/// セッション全体の計画。宣言のない instance は含まない。
#[derive(Debug, Clone, Default)]
pub struct CapabilityPlan {
    pub entries: Vec<PlanEntry>,
}

impl CapabilityPlan {
    pub fn push(&mut self, e: PlanEntry) {
        if !e.capabilities.is_empty() {
            self.entries.push(e);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 同意が必要な (artifact, capability) の一覧。同じ鍵は 1 件にまとめ、instance を併記する。
    pub fn requests(&self, manifest_id: &str, revision: u32) -> Vec<ConsentRequest> {
        let mut by_key: BTreeMap<String, ConsentRequest> = BTreeMap::new();
        let mut order: Vec<String> = Vec::new();
        for e in &self.entries {
            for cap in e.capabilities.items() {
                let key = consent_key(&e.artifact_sha256, &cap.to_string());
                match by_key.get_mut(&key) {
                    Some(r) => r.instances.push(e.instance.clone()),
                    None => {
                        order.push(key.clone());
                        by_key.insert(
                            key.clone(),
                            ConsentRequest {
                                request_id: key,
                                instances: vec![e.instance.clone()],
                                artifact: e.artifact.clone(),
                                artifact_sha256: e.artifact_sha256.clone(),
                                capability: cap.to_string(),
                                reason: format!(
                                    "部品 `{}` ({}) が{}",
                                    e.instance,
                                    e.artifact,
                                    cap.describe()
                                ),
                                manifest_id: manifest_id.to_owned(),
                                revision,
                            },
                        );
                    }
                }
            }
        }
        order
            .into_iter()
            .filter_map(|k| by_key.remove(&k))
            .collect()
    }

    pub fn entry(&self, instance: &str) -> Option<&PlanEntry> {
        self.entries.iter().find(|e| e.instance == instance)
    }

    /// instance に付与する WasiGrant (宣言がなければ空 = サンドボックス)。
    pub fn grant_for(&self, instance: &str, dirs: &DataDirs) -> anyhow::Result<WasiGrant> {
        match self.entry(instance) {
            Some(e) => WasiGrant::from_capabilities(&e.capabilities, &dirs.fs_root()),
            None => Ok(WasiGrant::default()),
        }
    }
}
