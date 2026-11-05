//! bootstrap: `(server, manifestId, revision, credential)` から manifest を取得 → 署名検証 →
//! アーティファクトをキャッシュに揃える → ローカル manifest として `Session::load` に渡せる形にする。
//!
//! サーバに届かないときは、キャッシュに manifest とアーティファクトが揃っていればそれで起動する
//! (02-architecture.md 起動シーケンス 6〜9 の失敗時)。署名検証はどちらの経路でも行う。

use crate::cache::{check_manifest_id, Cache};
use crate::session::{ArtifactResolver, SessionOptions};
use anyhow::{anyhow, bail, Context};
use ipemu_sign::TrustedKeys;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// サーバに対する資格情報。
#[derive(Debug, Clone)]
pub enum Credential {
    /// 利用者のセッショントークン (シェル経由) または交換済みの短命トークン
    Token(String),
    /// headless / CI 用クライアント API キー
    ApiKey(String),
    /// `ipemu://` から来た単回チケット。bootstrap が短命トークンに交換する
    Ticket(String),
}

impl Credential {
    fn kind(&self) -> &'static str {
        match self {
            Credential::Token(_) => "token",
            Credential::ApiKey(_) => "api_key",
            Credential::Ticket(_) => "ticket",
        }
    }
}

/// hello / telemetry に載せるクライアント情報。
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub engine_version: String,
    pub wit_versions: Vec<String>,
    pub os: String,
    pub arch: String,
    pub client_id: Option<String>,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            wit_versions: vec!["ip:emu@0.1.0".into()],
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            client_id: None,
        }
    }
}

impl ClientInfo {
    fn to_json(&self) -> Value {
        json!({
            "clientId": self.client_id,
            "engineVersion": self.engine_version,
            "witVersions": self.wit_versions,
            "os": self.os,
            "arch": self.arch,
        })
    }
}

pub struct BootstrapOptions {
    pub server: String,
    pub manifest_id: String,
    /// None なら `default`
    pub revision: Option<u32>,
    pub credential: Option<Credential>,
    pub cache: Cache,
    pub trusted_keys: TrustedKeys,
    /// 署名のない manifest (editor の「検証用に起動」や鍵未設定の開発サーバ) を許す
    pub allow_unsigned: bool,
    /// サーバに問い合わせずキャッシュだけで起動する
    pub offline: bool,
    /// hello / telemetry を送る (設定で off にできる。D18)
    pub telemetry: bool,
    pub client: ClientInfo,
    pub timeout: Duration,
}

impl BootstrapOptions {
    /// `manifest_id` は CLI (`--launch`) や `ipemu://` から来る未検証の値なので、ここで検査する (D55)。
    pub fn new(server: &str, manifest_id: &str, cache: Cache) -> anyhow::Result<Self> {
        check_manifest_id(manifest_id)?;
        Ok(Self {
            server: server.trim_end_matches('/').to_owned(),
            manifest_id: manifest_id.to_owned(),
            revision: None,
            credential: None,
            cache,
            trusted_keys: TrustedKeys::new(),
            allow_unsigned: false,
            offline: false,
            telemetry: true,
            client: ClientInfo::default(),
            timeout: Duration::from_secs(30),
        })
    }
}

/// bootstrap の結果。`session_options` で `Session::load` に渡すオプションを作る。
#[derive(Debug)]
pub struct Bootstrapped {
    pub server: String,
    pub manifest_id: String,
    pub revision: u32,
    /// キャッシュ内の manifest ファイル (これを `Session::load` に渡す)
    pub manifest_path: PathBuf,
    pub manifest: Value,
    pub name: String,
    /// サーバに届かずキャッシュから起動した
    pub from_cache: bool,
    /// 署名を検証できた (keyId)
    pub verified_key_id: Option<String>,
    /// サーバが発行した起動 ID (telemetry の紐付け)
    pub launch_id: Option<String>,
    /// 取得したアーティファクト数 / キャッシュ命中数
    pub downloaded: usize,
    pub cache_hits: usize,
    /// 以後の通信 (telemetry) に使う資格情報
    token: Option<String>,
    telemetry_enabled: bool,
    timeout: Duration,
}

impl Bootstrapped {
    /// キャッシュのアーティファクトを指す resolver を付けた `SessionOptions`。
    pub fn session_options(&self, cache: &Cache, mut base: SessionOptions) -> SessionOptions {
        let cache = cache.clone();
        base.artifact_resolver = Some(ArtifactResolver(Arc::new(move |name, a| {
            let sha = a
                .sha256
                .as_deref()
                .ok_or_else(|| anyhow!("アーティファクト `{name}` に sha256 がありません"))?;
            let p = cache.artifact_path(sha);
            anyhow::ensure!(
                p.is_file(),
                "アーティファクト `{name}` ({sha}) がキャッシュにありません"
            );
            Ok(Some(p))
        })));
        base
    }

    /// telemetry 送信用のハンドル (無効なら None)。
    pub fn telemetry(&self) -> Option<Telemetry> {
        if !self.telemetry_enabled || self.from_cache {
            return None;
        }
        Some(Telemetry {
            server: self.server.clone(),
            token: self.token.clone()?,
            launch_id: self.launch_id.clone(),
            manifest_id: self.manifest_id.clone(),
            revision: self.revision,
            timeout: self.timeout,
        })
    }
}

/// 実行要約の送信 (best-effort。失敗しても実行は止めない)。
#[derive(Debug, Clone)]
pub struct Telemetry {
    server: String,
    token: String,
    launch_id: Option<String>,
    manifest_id: String,
    revision: u32,
    timeout: Duration,
}

impl Telemetry {
    pub fn send(&self, event: &str, detail: Value) -> anyhow::Result<()> {
        let client = http_client(self.timeout)?;
        let resp = client
            .post(format!("{}/api/v1/telemetry", self.server))
            .bearer_auth(&self.token)
            .json(&json!({
                "launchId": self.launch_id,
                "manifestId": self.manifest_id,
                "revision": self.revision,
                "event": event,
                "detail": detail,
            }))
            .send()?;
        anyhow::ensure!(
            resp.status().is_success(),
            "telemetry: HTTP {}",
            resp.status()
        );
        Ok(())
    }
}

fn http_client(timeout: Duration) -> anyhow::Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(timeout)
        .user_agent(format!("ipemu-engine/{}", env!("CARGO_PKG_VERSION")))
        .build()?)
}

fn error_body(resp: reqwest::blocking::Response) -> String {
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    let msg = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
        .unwrap_or(text);
    format!("HTTP {status}: {msg}")
}

/// 資格情報を Bearer トークンに解決する (チケットは交換する)。
fn resolve_token(
    client: &reqwest::blocking::Client,
    server: &str,
    cred: &Credential,
) -> anyhow::Result<String> {
    match cred {
        Credential::Token(t) | Credential::ApiKey(t) => Ok(t.clone()),
        Credential::Ticket(t) => {
            let resp = client
                .post(format!("{server}/api/v1/launch-tickets/{t}/exchange"))
                .send()
                .context("チケット交換の接続に失敗")?;
            if !resp.status().is_success() {
                bail!("チケットを交換できません ({})", error_body(resp));
            }
            let v: Value = resp.json()?;
            v["token"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("チケット交換の応答に token がありません"))
        }
    }
}

/// アーティファクトの url をサーバ基準で絶対化する。
fn absolutize(server: &str, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_owned()
    } else if let Some(rest) = url.strip_prefix('/') {
        format!("{server}/{rest}")
    } else {
        format!("{server}/{url}")
    }
}

/// 署名を検証し、manifest の revision / name を取り出す。
fn verify(
    manifest: &Value,
    opts: &BootstrapOptions,
    context: &str,
) -> anyhow::Result<Option<String>> {
    match ipemu_sign::verify_manifest(manifest, &opts.trusted_keys) {
        Ok(v) => Ok(Some(v.key_id)),
        Err(ipemu_sign::SignError::MissingSignature) if opts.allow_unsigned => {
            eprintln!("warning: {context}: manifest に署名がありません (--no-signature-check で許可)");
            Ok(None)
        }
        Err(e) if opts.allow_unsigned => {
            eprintln!("warning: {context}: 署名検証に失敗しましたが --no-signature-check で続行します ({e})");
            Ok(None)
        }
        Err(ipemu_sign::SignError::UnknownKey(k)) if opts.trusted_keys.is_empty() => Err(anyhow!(
            "{context}: 信頼済み公開鍵が 1 つもありません (keyId `{k}`)。--trusted-key keyId=hex か IPEMU_TRUSTED_KEYS で与えてください"
        )),
        Err(e) => Err(anyhow!("{context}: 署名検証に失敗: {e}")),
    }
}

fn rev_of(manifest: &Value) -> anyhow::Result<u32> {
    manifest["revision"]
        .as_u64()
        .map(|r| r as u32)
        .ok_or_else(|| anyhow!("manifest に revision がありません"))
}

fn name_of(manifest: &Value) -> String {
    manifest["name"].as_str().unwrap_or("").to_owned()
}

/// 入口。オンラインで取得し、失敗したらキャッシュに落ちる。
pub fn bootstrap(opts: &BootstrapOptions) -> anyhow::Result<Bootstrapped> {
    // フィールドは公開なので、new の後に書き換えられた場合に備えて入口でも検査する
    check_manifest_id(&opts.manifest_id)?;
    if opts.offline {
        return from_cache(opts, "offline 指定");
    }
    match online(opts) {
        Ok(b) => Ok(b),
        Err(e) => {
            // 認証エラーや 404 はキャッシュに落とさない (別の構成を誤って動かさないため)。
            // 接続できない / タイムアウトのときだけオフライン起動を試みる
            if e.downcast_ref::<reqwest::Error>()
                .is_some_and(|re| re.is_connect() || re.is_timeout() || re.is_request())
            {
                eprintln!("warning: サーバに届きません ({e:#})。キャッシュからの起動を試みます");
                from_cache(opts, "オフライン").with_context(|| format!("サーバにも届かず ({e:#})"))
            } else {
                Err(e)
            }
        }
    }
}

fn online(opts: &BootstrapOptions) -> anyhow::Result<Bootstrapped> {
    let client = http_client(opts.timeout)?;
    let server = opts.server.as_str();
    let cred = opts.credential.as_ref().ok_or_else(|| {
        anyhow!("サーバから取得するには資格情報 (--token / --api-key / --ticket) が必要です")
    })?;
    let token = resolve_token(&client, server, cred)?;

    // hello (best-effort)
    if opts.telemetry {
        let _ = client
            .post(format!("{server}/api/v1/clients/hello"))
            .bearer_auth(&token)
            .json(&opts.client.to_json())
            .send();
    }

    // manifest
    let rev_q = opts
        .revision
        .map(|r| r.to_string())
        .unwrap_or_else(|| "default".into());
    let resp = client
        .get(format!(
            "{server}/api/v1/manifests/{}?rev={rev_q}",
            opts.manifest_id
        ))
        .bearer_auth(&token)
        .header("x-ipemu-client", opts.client.to_json().to_string())
        .send()
        .context("manifest の取得に失敗")?;
    if !resp.status().is_success() {
        bail!(
            "manifest `{}` (rev {rev_q}) を取得できません ({})",
            opts.manifest_id,
            error_body(resp)
        );
    }
    let launch_id = resp
        .headers()
        .get("x-ipemu-launch-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let text = resp.text()?;
    let manifest: Value = serde_json::from_str(&text).context("manifest の JSON が不正")?;
    anyhow::ensure!(
        manifest["id"].as_str() == Some(opts.manifest_id.as_str()),
        "取得した manifest の id が一致しません"
    );
    let revision = rev_of(&manifest)?;
    let verified = verify(&manifest, opts, "サーバから取得した manifest")?;

    // artifacts
    let mut downloaded = 0usize;
    let mut cache_hits = 0usize;
    if let Some(arts) = manifest["artifacts"].as_object() {
        for (name, a) in arts {
            let sha = a["sha256"].as_str().ok_or_else(|| {
                anyhow!("アーティファクト `{name}` に sha256 がありません (配信 manifest には必須)")
            })?;
            if opts.cache.has_artifact(sha) {
                cache_hits += 1;
                continue;
            }
            let url = a["url"]
                .as_str()
                .map(|u| absolutize(server, u))
                .unwrap_or_else(|| format!("{server}/api/v1/artifacts/by-hash/{sha}"));
            let resp = client
                .get(&url)
                .bearer_auth(&token)
                .send()
                .with_context(|| format!("アーティファクト `{name}` の取得に失敗"))?;
            if !resp.status().is_success() {
                bail!(
                    "アーティファクト `{name}` を取得できません ({})",
                    error_body(resp)
                );
            }
            let bytes = resp.bytes()?;
            opts.cache
                .put_artifact(&bytes, sha)
                .with_context(|| format!("アーティファクト `{name}`"))?;
            downloaded += 1;
        }
    }

    let manifest_path = opts
        .cache
        .put_manifest(server, &opts.manifest_id, revision, &text)?;
    opts.cache.record_launch(
        server,
        &opts.manifest_id,
        revision,
        &name_of(&manifest),
        opts.revision.is_none(),
        verified.is_some(),
    )?;
    Ok(Bootstrapped {
        server: server.to_owned(),
        manifest_id: opts.manifest_id.clone(),
        revision,
        manifest_path,
        name: name_of(&manifest),
        manifest,
        from_cache: false,
        verified_key_id: verified,
        launch_id,
        downloaded,
        cache_hits,
        token: Some(token),
        telemetry_enabled: opts.telemetry,
        timeout: opts.timeout,
    })
}

fn from_cache(opts: &BootstrapOptions, why: &str) -> anyhow::Result<Bootstrapped> {
    let server = opts.server.as_str();
    let revision = match opts.revision {
        Some(r) => r,
        None => opts
            .cache
            .default_revision(server, &opts.manifest_id)
            .ok_or_else(|| anyhow!("{why}: `{}` はキャッシュにありません", opts.manifest_id))?,
    };
    let manifest = opts
        .cache
        .get_manifest(server, &opts.manifest_id, revision)?
        .ok_or_else(|| {
            anyhow!(
                "{why}: `{}` rev {revision} はキャッシュにありません",
                opts.manifest_id
            )
        })?;
    let verified = verify(&manifest, opts, "キャッシュの manifest")?;
    let missing = opts.cache.missing_artifacts(&manifest);
    if !missing.is_empty() {
        bail!(
            "{why}: `{}` rev {revision} のアーティファクトがキャッシュに揃っていません: {}",
            opts.manifest_id,
            missing.join(", ")
        );
    }
    let cache_hits = manifest["artifacts"]
        .as_object()
        .map(|a| a.len())
        .unwrap_or(0);
    opts.cache.record_launch(
        server,
        &opts.manifest_id,
        revision,
        &name_of(&manifest),
        opts.revision.is_none(),
        verified.is_some(),
    )?;
    Ok(Bootstrapped {
        server: server.to_owned(),
        manifest_id: opts.manifest_id.clone(),
        revision,
        manifest_path: opts
            .cache
            .manifest_path(server, &opts.manifest_id, revision)?,
        name: name_of(&manifest),
        manifest,
        from_cache: true,
        verified_key_id: verified,
        launch_id: None,
        downloaded: 0,
        cache_hits,
        token: None,
        telemetry_enabled: false,
        timeout: opts.timeout,
    })
}

/// サーバのカタログ (`GET /api/v1/catalog`)。
pub fn remote_catalog(server: &str, cred: &Credential, timeout: Duration) -> anyhow::Result<Value> {
    let server = server.trim_end_matches('/');
    let client = http_client(timeout)?;
    let token = resolve_token(&client, server, cred)?;
    let resp = client
        .get(format!("{server}/api/v1/catalog"))
        .bearer_auth(&token)
        .send()
        .context("カタログの取得に失敗")?;
    if !resp.status().is_success() {
        bail!("カタログを取得できません ({})", error_body(resp));
    }
    Ok(resp.json()?)
}

/// 既定の信頼鍵: 同梱の `keys/trusted-keys.json` + 環境変数 `IPEMU_TRUSTED_KEYS` (`keyId=hex,...`)。
pub fn default_trusted_keys() -> anyhow::Result<TrustedKeys> {
    let mut keys = TrustedKeys::from_json(EMBEDDED_TRUSTED_KEYS)?;
    if let Ok(env) = std::env::var("IPEMU_TRUSTED_KEYS") {
        for e in TrustedKeys::parse_list(&env)?.to_entries() {
            keys.insert_hex(&e.key_id, &e.public_key)?;
        }
    }
    if let Ok(path) = std::env::var("IPEMU_TRUSTED_KEYS_FILE") {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("IPEMU_TRUSTED_KEYS_FILE {path} を読めません"))?;
        for e in TrustedKeys::from_json(&text)?.to_entries() {
            keys.insert_hex(&e.key_id, &e.public_key)?;
        }
    }
    Ok(keys)
}

/// クライアント本体に同梱する公開鍵 (keyId で世代管理。鍵の更新 = 本体の更新)。
pub const EMBEDDED_TRUSTED_KEYS: &str = include_str!("../keys/trusted-keys.json");

#[allow(dead_code)]
fn _credential_kind(c: &Credential) -> &'static str {
    c.kind()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_manifest_id_is_rejected_at_entry() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().join("cache")).unwrap();
        for bad in ["../x", "../../x", "A", "a", ""] {
            assert!(
                BootstrapOptions::new("http://127.0.0.1:9", bad, cache.clone()).is_err(),
                "{bad:?}"
            );
        }
        // new の後に書き換えても bootstrap の入口で止まる (通信もキャッシュ参照もしない)
        let mut bo = BootstrapOptions::new("http://127.0.0.1:9", "cartpole", cache).unwrap();
        bo.offline = true;
        bo.manifest_id = "../../x".into();
        let err = format!("{:#}", bootstrap(&bo).unwrap_err());
        assert!(err.contains("manifest id"), "{err}");
    }
}
