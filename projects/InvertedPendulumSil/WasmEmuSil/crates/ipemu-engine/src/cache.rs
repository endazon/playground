//! クライアント側キャッシュ (07-client-app.md「キャッシュとオフライン」)。
//!
//! ```text
//! <cache dir>/ipemu/
//!   sha256/ab/cd/<hex>                         アーティファクト本体 (イミュータブル)
//!   manifests/<server>/<id>/<revision>.json    署名付き manifest
//!   catalog.json                               ローカルカタログ (キャッシュ済み構成と最終起動日時)
//! ```

use anyhow::Context;
use ipemu_core::manifest::is_valid_manifest_id;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

/// ローカルカタログの 1 項目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalCatalogEntry {
    pub server: String,
    pub manifest_id: String,
    pub revision: u32,
    pub name: String,
    /// サーバで「既定」だった revision か (rev=default で起動したときに真)
    #[serde(default)]
    pub is_default: bool,
    pub last_launched_at: String,
    #[serde(default)]
    pub signed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalCatalog {
    #[serde(default)]
    pub entries: Vec<LocalCatalogEntry>,
}

/// URL をディレクトリ名にできる形へ (`https://ipemu.example:8443` → `ipemu.example_8443`)。
pub fn server_key(server: &str) -> String {
    let s = server
        .trim()
        .trim_end_matches('/')
        .split("://")
        .last()
        .unwrap_or(server);
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "server".into()
    } else {
        out
    }
}

/// manifest id をパスに使う前の検査 (D46)。`../x` などでキャッシュの外を指させない。
pub fn check_manifest_id(manifest_id: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        is_valid_manifest_id(manifest_id),
        "manifest id `{}` が不正です (^[a-z0-9][a-z0-9-]{{2,63}}$)",
        manifest_id.escape_debug()
    );
    Ok(())
}

fn now_rfc3339() -> String {
    // time クレートに依存せず秒精度の UTC を出す (ログ用途なので十分)
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let rem = secs % 86400;
    // 1970-01-01 からの日数 → 年月日 (proleptic Gregorian)
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

impl Cache {
    /// `root` は `<cache dir>/ipemu` に相当するディレクトリ。
    pub fn new(root: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)
            .with_context(|| format!("キャッシュ {} を作れません", root.display()))?;
        Ok(Self { root })
    }

    /// 既定の場所: `IPEMU_CACHE_DIR`、なければ OS のキャッシュディレクトリ配下の `ipemu`。
    pub fn default_dir() -> PathBuf {
        if let Ok(p) = std::env::var("IPEMU_CACHE_DIR") {
            return PathBuf::from(p);
        }
        if let Ok(p) = std::env::var("XDG_CACHE_HOME") {
            return PathBuf::from(p).join("ipemu");
        }
        if let Ok(p) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(p).join("ipemu").join("cache");
        }
        if let Ok(home) = std::env::var("HOME") {
            if cfg!(target_os = "macos") {
                return PathBuf::from(home).join("Library/Caches/ipemu");
            }
            return PathBuf::from(home).join(".cache/ipemu");
        }
        PathBuf::from(".ipemu-cache")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn artifact_path(&self, sha256: &str) -> PathBuf {
        self.root
            .join("sha256")
            .join(&sha256[0..2.min(sha256.len())])
            .join(&sha256[2.min(sha256.len())..4.min(sha256.len())])
            .join(sha256)
    }

    pub fn has_artifact(&self, sha256: &str) -> bool {
        sha256.len() == 64 && self.artifact_path(sha256).is_file()
    }

    /// 内容を検証して保存する。期待ハッシュと一致しなければ保存しない。
    pub fn put_artifact(&self, bytes: &[u8], expected_sha256: &str) -> anyhow::Result<PathBuf> {
        use sha2::{Digest, Sha256};
        let actual = hex::encode(Sha256::digest(bytes));
        anyhow::ensure!(
            actual == expected_sha256,
            "取得したアーティファクトの sha256 が一致しません (manifest {expected_sha256}, 実体 {actual})"
        );
        let path = self.artifact_path(&actual);
        if path.is_file() {
            return Ok(path);
        }
        let parent = path.parent().unwrap();
        std::fs::create_dir_all(parent)?;
        let tmp = parent.join(format!(".{actual}.{}.tmp", std::process::id()));
        std::fs::write(&tmp, bytes)?;
        if let Err(e) = std::fs::rename(&tmp, &path) {
            let _ = std::fs::remove_file(&tmp);
            if !path.is_file() {
                return Err(e.into());
            }
        }
        Ok(path)
    }

    /// `manifests/<server>/<id>/`。id がスキーマに適合しなければ Err (パスを作らない)。
    pub fn manifest_dir(&self, server: &str, manifest_id: &str) -> anyhow::Result<PathBuf> {
        check_manifest_id(manifest_id)?;
        Ok(self
            .root
            .join("manifests")
            .join(server_key(server))
            .join(manifest_id))
    }

    pub fn manifest_path(
        &self,
        server: &str,
        manifest_id: &str,
        revision: u32,
    ) -> anyhow::Result<PathBuf> {
        Ok(self
            .manifest_dir(server, manifest_id)?
            .join(format!("{revision}.json")))
    }

    pub fn put_manifest(
        &self,
        server: &str,
        manifest_id: &str,
        revision: u32,
        json_text: &str,
    ) -> anyhow::Result<PathBuf> {
        let path = self.manifest_path(server, manifest_id, revision)?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json_text)?;
        std::fs::rename(&tmp, &path)?;
        Ok(path)
    }

    pub fn get_manifest(
        &self,
        server: &str,
        manifest_id: &str,
        revision: u32,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        let path = self.manifest_path(server, manifest_id, revision)?;
        match std::fs::read_to_string(&path) {
            Ok(s) => {
                Ok(Some(serde_json::from_str(&s).with_context(|| {
                    format!("{} が壊れています", path.display())
                })?))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// キャッシュにある revision の一覧 (昇順)。id が不正なら空。
    pub fn cached_revisions(&self, server: &str, manifest_id: &str) -> Vec<u32> {
        let Ok(dir) = self.manifest_dir(server, manifest_id) else {
            return Vec::new();
        };
        let mut revs: Vec<u32> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter_map(|e| {
                        e.file_name()
                            .to_str()?
                            .strip_suffix(".json")?
                            .parse::<u32>()
                            .ok()
                    })
                    .collect()
            })
            .unwrap_or_default();
        revs.sort_unstable();
        revs
    }

    /// manifest が参照する全アーティファクトがキャッシュに揃っているか。足りないものの名前を返す。
    pub fn missing_artifacts(&self, manifest: &serde_json::Value) -> Vec<String> {
        let mut missing = Vec::new();
        if let Some(arts) = manifest.get("artifacts").and_then(|a| a.as_object()) {
            for (name, a) in arts {
                match a.get("sha256").and_then(|v| v.as_str()) {
                    Some(h) if self.has_artifact(h) => {}
                    _ => missing.push(name.clone()),
                }
            }
        }
        missing
    }

    pub fn catalog_path(&self) -> PathBuf {
        self.root.join("catalog.json")
    }

    pub fn load_catalog(&self) -> LocalCatalog {
        std::fs::read_to_string(self.catalog_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// 起動した構成をローカルカタログに記録する (同じ server/id/rev は上書き)。
    pub fn record_launch(
        &self,
        server: &str,
        manifest_id: &str,
        revision: u32,
        name: &str,
        is_default: bool,
        signed: bool,
    ) -> anyhow::Result<()> {
        let mut cat = self.load_catalog();
        cat.entries.retain(|e| {
            !(e.server == server && e.manifest_id == manifest_id && e.revision == revision)
        });
        if is_default {
            // 既定は 1 つ
            for e in cat.entries.iter_mut() {
                if e.server == server && e.manifest_id == manifest_id {
                    e.is_default = false;
                }
            }
        }
        cat.entries.push(LocalCatalogEntry {
            server: server.to_owned(),
            manifest_id: manifest_id.to_owned(),
            revision,
            name: name.to_owned(),
            is_default,
            last_launched_at: now_rfc3339(),
            signed,
        });
        cat.entries
            .sort_by(|a, b| b.last_launched_at.cmp(&a.last_launched_at));
        let tmp = self.catalog_path().with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&cat)?)?;
        std::fs::rename(&tmp, self.catalog_path())?;
        Ok(())
    }

    /// ローカルカタログで既定 (または最後に起動した) revision を引く。
    pub fn default_revision(&self, server: &str, manifest_id: &str) -> Option<u32> {
        let cat = self.load_catalog();
        let mine: Vec<&LocalCatalogEntry> = cat
            .entries
            .iter()
            .filter(|e| e.server == server && e.manifest_id == manifest_id)
            .collect();
        mine.iter()
            .find(|e| e.is_default)
            .or(mine.first())
            .map(|e| e.revision)
            .or_else(|| self.cached_revisions(server, manifest_id).last().copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_key_is_filesystem_safe() {
        assert_eq!(
            server_key("https://ipemu.example:8443/"),
            "ipemu.example_8443"
        );
        assert_eq!(server_key("http://127.0.0.1:8080"), "127.0.0.1_8080");
    }

    #[test]
    fn manifest_id_is_checked_before_touching_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ipemu");
        let c = Cache::new(&root).unwrap();
        let server = "http://127.0.0.1:8080";
        for bad in ["../x", "../../x", "A", "a", ""] {
            assert!(c.manifest_dir(server, bad).is_err(), "{bad:?}");
            assert!(c.manifest_path(server, bad, 1).is_err(), "{bad:?}");
            assert!(c.put_manifest(server, bad, 1, "{}").is_err(), "{bad:?}");
            assert!(c.get_manifest(server, bad, 1).is_err(), "{bad:?}");
            assert!(c.cached_revisions(server, bad).is_empty(), "{bad:?}");
            assert_eq!(c.default_revision(server, bad), None, "{bad:?}");
        }
        // キャッシュの外 (root の親) に何も書かれていない
        let outside: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(outside, ["ipemu"]);

        // 正しい id は通る
        let p = c
            .put_manifest(server, "cartpole", 3, r#"{"id":"cartpole"}"#)
            .unwrap();
        assert!(p.starts_with(&root));
        assert!(p.ends_with("manifests/127.0.0.1_8080/cartpole/3.json"));
        assert_eq!(
            c.get_manifest(server, "cartpole", 3).unwrap().unwrap()["id"],
            "cartpole"
        );
        assert_eq!(c.cached_revisions(server, "cartpole"), [3]);
    }

    #[test]
    fn rfc3339_now_has_expected_shape() {
        let s = now_rfc3339();
        assert_eq!(s.len(), 20, "{s}");
        assert!(s.ends_with('Z'));
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19723), (2024, 1, 1));
    }

    #[test]
    fn artifact_put_verifies_hash_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let c = Cache::new(dir.path()).unwrap();
        let bytes = b"hello";
        let sha = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(!c.has_artifact(sha));
        let p = c.put_artifact(bytes, sha).unwrap();
        assert!(p.ends_with("sha256/2c/f2/".to_owned() + sha));
        assert!(c.has_artifact(sha));
        c.put_artifact(bytes, sha).unwrap();
        assert!(c.put_artifact(b"other", sha).is_err());
    }
}
