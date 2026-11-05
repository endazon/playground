//! 同意 (consent) の保存と問い合わせ。
//!
//! - ストア: `<data dir>/ipemu/consent.json`。鍵は `sha256(artifact):capability` ([`ipemu_core::consent`])。
//! - 問い合わせ: [`ConsentProvider`]。シェル (M2) は IPC の `consent.required` / `consent.answer` で実装し、
//!   headless は `--consent-file` ([`FileConsent`]) か「何も答えない」([`NoConsent`]) になる。
//! - 判定: [`resolve_consent`]。拒否が 1 件でもあれば**起動不可** (部分起動しない)。

use anyhow::Context;
use ipemu_core::consent::{split_consent_key, ConsentAnswer, ConsentEntry, ConsentRequest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// 同意ストア (ファイルの読み書き)。
#[derive(Debug, Clone)]
pub struct ConsentStore {
    path: PathBuf,
    entries: BTreeMap<String, ConsentEntry>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoreFile {
    version: u32,
    #[serde(default)]
    entries: BTreeMap<String, ConsentEntry>,
}

impl ConsentStore {
    /// 無ければ空のストアとして開く。壊れたファイルはエラー (黙って捨てない)。
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let entries = match std::fs::read_to_string(path) {
            Ok(text) => {
                let f: StoreFile = serde_json::from_str(&text)
                    .with_context(|| format!("同意ストア {} が壊れています", path.display()))?;
                anyhow::ensure!(
                    f.version == 1,
                    "同意ストア {} の version {} は未対応",
                    path.display(),
                    f.version
                );
                f.entries
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => return Err(e).with_context(|| format!("{} を読めません", path.display())),
        };
        Ok(Self {
            path: path.to_path_buf(),
            entries,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 記憶された回答。`rememberUntil` を過ぎたものは無視する。
    pub fn get(&self, key: &str, now: &str) -> Option<&ConsentEntry> {
        let e = self.entries.get(key)?;
        match &e.remember_until {
            Some(until) if until.as_str() <= now => None,
            _ => Some(e),
        }
    }

    pub fn set(&mut self, key: String, entry: ConsentEntry) {
        self.entries.insert(key, entry);
    }

    pub fn remove(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    pub fn entries(&self) -> impl Iterator<Item = (&String, &ConsentEntry)> {
        self.entries.iter()
    }

    /// 一時ファイルに書いてから rename する (途中で落ちても壊れない)。
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("{} を作れません", dir.display()))?;
        }
        let f = StoreFile {
            version: 1,
            entries: self.entries.clone(),
        };
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&f)?)
            .with_context(|| format!("{} を書けません", tmp.display()))?;
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("{} へ置き換えられません", self.path.display()))?;
        Ok(())
    }
}

/// 同意を利用者に問い合わせる手段。回答がない request は「未同意」のまま残る。
pub trait ConsentProvider: Send + Sync {
    fn ask(&self, requests: &[ConsentRequest]) -> anyhow::Result<Vec<ConsentAnswer>>;
    /// 表示用の名前
    fn name(&self) -> &'static str;
}

/// 何も答えない (headless で `--consent-file` がないとき)。
#[derive(Debug, Default)]
pub struct NoConsent;

impl ConsentProvider for NoConsent {
    fn ask(&self, _: &[ConsentRequest]) -> anyhow::Result<Vec<ConsentAnswer>> {
        Ok(Vec::new())
    }
    fn name(&self) -> &'static str {
        "none"
    }
}

/// `--consent-file` の形式。
///
/// ```json
/// { "version": 1,
///   "grants": ["<sha256>:net.tcpConnect:127.0.0.1:9000", "*:fs:rw:logs"],
///   "denies": ["<sha256>:clock"] }
/// ```
/// `*:` はアーティファクトを問わない。開発時の再ビルド向けで、ローカル開発モード
/// (`--no-signature-check` = `allow_unhashed`) のときだけ受理し、それ以外はエラーにする。
/// `remember: true` なら許可を同意ストアにも書く (既定は今回だけ)。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentFile {
    #[serde(default = "one")]
    pub version: u32,
    #[serde(default)]
    pub grants: Vec<String>,
    #[serde(default)]
    pub denies: Vec<String>,
    #[serde(default)]
    pub remember: bool,
}

fn one() -> u32 {
    1
}

/// `--consent-file` からの事前同意。
#[derive(Debug)]
pub struct FileConsent {
    grants: HashSet<String>,
    denies: HashSet<String>,
    remember: bool,
}

impl FileConsent {
    /// `allow_wildcard` はローカル開発モード (`allow_unhashed`) のときだけ true にする。
    pub fn load(path: &Path, allow_wildcard: bool) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("同意ファイル {} を読めません", path.display()))?;
        let f: ConsentFile = serde_json::from_str(&text)
            .with_context(|| format!("同意ファイル {} の形式が不正", path.display()))?;
        Self::from_file(f, allow_wildcard)
            .with_context(|| format!("同意ファイル {}", path.display()))
    }

    pub fn from_file(f: ConsentFile, allow_wildcard: bool) -> anyhow::Result<Self> {
        anyhow::ensure!(f.version == 1, "version {} は未対応", f.version);
        for k in f.grants.iter().chain(&f.denies) {
            if let Some(cap) = k.strip_prefix("*:") {
                // アーティファクト不問の指定は配布用の設定で使えないよう、開発モードに限る
                anyhow::ensure!(
                    allow_wildcard,
                    "鍵 `{k}` のワイルドカード (アーティファクト不問) はローカル開発モード (--no-signature-check) でのみ使えます"
                );
                anyhow::ensure!(!cap.is_empty(), "鍵 `{k}` の capability が空です");
            } else if split_consent_key(k).is_none() {
                anyhow::bail!("鍵 `{k}` は `<sha256>:<capability>` か `*:<capability>` の形式");
            }
        }
        Ok(Self {
            grants: f
                .grants
                .into_iter()
                .map(|s| s.to_ascii_lowercase())
                .collect(),
            denies: f
                .denies
                .into_iter()
                .map(|s| s.to_ascii_lowercase())
                .collect(),
            remember: f.remember,
        })
    }

    fn lookup(set: &HashSet<String>, r: &ConsentRequest) -> bool {
        set.contains(&r.request_id.to_ascii_lowercase())
            || set.contains(&format!("*:{}", r.capability).to_ascii_lowercase())
    }
}

impl ConsentProvider for FileConsent {
    fn ask(&self, requests: &[ConsentRequest]) -> anyhow::Result<Vec<ConsentAnswer>> {
        Ok(requests
            .iter()
            .filter_map(|r| {
                // 明示の拒否 > 許可 > 回答なし
                if Self::lookup(&self.denies, r) {
                    Some(false)
                } else if Self::lookup(&self.grants, r) {
                    Some(true)
                } else {
                    None
                }
                .map(|grant| ConsentAnswer {
                    request_id: r.request_id.clone(),
                    grant,
                    remember: self.remember,
                })
            })
            .collect())
    }
    fn name(&self) -> &'static str {
        "consent-file"
    }
}

/// 同意の判定結果。
#[derive(Debug)]
pub enum ConsentOutcome {
    /// すべて同意済み (起動してよい)
    Ready,
    /// 回答のない capability がある (シェルなら問い合わせ中、headless なら非ゼロ終了)
    Required(Vec<ConsentRequest>),
    /// 拒否された capability がある → 起動不可
    Denied(Vec<ConsentRequest>),
}

/// ストアの記憶 → provider への問い合わせ、の順で全 request を判定する。
/// 「記憶する」と答えた分はストアに保存する (ストアが None なら保存しない)。
pub fn resolve_consent(
    requests: Vec<ConsentRequest>,
    store: Option<&mut ConsentStore>,
    provider: &dyn ConsentProvider,
) -> anyhow::Result<ConsentOutcome> {
    if requests.is_empty() {
        return Ok(ConsentOutcome::Ready);
    }
    let now = rfc3339_now();
    let mut pending: Vec<ConsentRequest> = Vec::new();
    let mut denied: Vec<ConsentRequest> = Vec::new();
    for r in requests {
        match store.as_ref().and_then(|s| s.get(&r.request_id, &now)) {
            Some(e) if e.granted => {}
            Some(_) => denied.push(r),
            None => pending.push(r),
        }
    }
    if !pending.is_empty() {
        let answers = provider.ask(&pending)?;
        let mut unanswered = Vec::new();
        let mut dirty = false;
        let mut store = store;
        for r in pending {
            match answers.iter().find(|a| a.request_id == r.request_id) {
                None => unanswered.push(r),
                Some(a) => {
                    if a.remember {
                        if let Some(s) = store.as_deref_mut() {
                            s.set(
                                r.request_id.clone(),
                                ConsentEntry {
                                    granted: a.grant,
                                    at: now.clone(),
                                    manifest_id: r.manifest_id.clone(),
                                    revision: r.revision,
                                    remember_until: None,
                                },
                            );
                            dirty = true;
                        }
                    }
                    if !a.grant {
                        denied.push(r);
                    }
                }
            }
        }
        if dirty {
            if let Some(s) = store.as_deref() {
                s.save()?;
            }
        }
        if !denied.is_empty() {
            return Ok(ConsentOutcome::Denied(denied));
        }
        if !unanswered.is_empty() {
            return Ok(ConsentOutcome::Required(unanswered));
        }
        return Ok(ConsentOutcome::Ready);
    }
    if !denied.is_empty() {
        return Ok(ConsentOutcome::Denied(denied));
    }
    Ok(ConsentOutcome::Ready)
}

/// 現在時刻の RFC 3339 (UTC、秒精度)。chrono を持ち込まないための最小実装。
pub fn rfc3339_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    rfc3339_from_unix(secs)
}

pub fn rfc3339_from_unix(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Howard Hinnant の civil_from_days
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_requires_local_dev_mode() {
        let f = ConsentFile {
            version: 1,
            grants: vec!["*:clock".into()],
            denies: vec![],
            remember: false,
        };
        assert!(FileConsent::from_file(f.clone(), false).is_err());
        assert!(FileConsent::from_file(f, true).is_ok());
        let exact = ConsentFile {
            version: 1,
            grants: vec![format!("{}:clock", "ab".repeat(32))],
            denies: vec![],
            remember: false,
        };
        assert!(FileConsent::from_file(exact, false).is_ok());
    }

    #[test]
    fn rfc3339_known_values() {
        assert_eq!(rfc3339_from_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_from_unix(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339_from_unix(1_762_300_800), "2025-11-05T00:00:00Z");
    }
}
