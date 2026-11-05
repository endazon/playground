//! アーティファクトの内容アドレス保存: `<store>/artifacts/sha256/ab/cd/<hex>`。イミュータブルで削除しない。

use crate::util;
use anyhow::Context;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn new(store_dir: &Path) -> anyhow::Result<Self> {
        let root = store_dir.join("artifacts");
        std::fs::create_dir_all(&root)
            .with_context(|| format!("{} を作れません", root.display()))?;
        Ok(Self { root })
    }

    pub fn path_for(&self, sha256: &str) -> PathBuf {
        self.root
            .join("sha256")
            .join(&sha256[0..2])
            .join(&sha256[2..4])
            .join(sha256)
    }

    /// 書き込み: sha256 を返す。同じ内容が既にあれば何もしない。
    pub fn put(&self, bytes: &[u8]) -> anyhow::Result<String> {
        let sha = util::sha256_hex(bytes);
        let path = self.path_for(&sha);
        if path.exists() {
            return Ok(sha);
        }
        let parent = path.parent().unwrap();
        std::fs::create_dir_all(parent)?;
        // 途中で落ちても壊れた本体が残らないよう、一時ファイルに書いてから rename する
        let tmp = parent.join(format!(".{}.{}.tmp", sha, std::process::id()));
        std::fs::write(&tmp, bytes)?;
        match std::fs::rename(&tmp, &path) {
            Ok(()) => {}
            Err(e) if path.exists() => {
                let _ = std::fs::remove_file(&tmp);
                let _ = e;
            }
            Err(e) => return Err(e.into()),
        }
        Ok(sha)
    }

    pub fn get(&self, sha256: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let path = self.path_for(sha256);
        match std::fs::read(&path) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
