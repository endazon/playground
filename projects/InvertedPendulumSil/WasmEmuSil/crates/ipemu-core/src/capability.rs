//! capability 宣言 (manifest の `components[].capabilities`) の型と、
//! 部品が import する WIT インターフェース名との突き合わせ (V7 / V8)、
//! `approval.capabilitiesHash` の計算、同意の鍵の生成。
//!
//! 「既定はサンドボックス、例外は 3 段で通す」(01-concept.md) のうち、
//! 宣言 (この型) と承認 (ハッシュ) をここで扱い、同意は [`crate::consent`]、
//! 実施 (Linker への link) は `ipemu-wasm` が担う。

use crate::jcs;
use crate::manifest::Manifest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::net::{IpAddr, SocketAddr};

/// エンジンが提供するホスト拡張 (`abi/wit/host-ext.wit` のインターフェース名)。
pub const KNOWN_HOST_EXT: &[&str] = &["serial-port"];

/// instance 単位の capability 宣言。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net: Option<NetCapabilities>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fs: Vec<FsGrant>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub clock: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub random: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub host_ext: Vec<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetCapabilities {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tcp_connect: Vec<HostPort>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tcp_listen: Vec<HostPort>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub udp: Vec<HostPort>,
}

impl NetCapabilities {
    pub fn is_empty(&self) -> bool {
        self.tcp_connect.is_empty() && self.tcp_listen.is_empty() && self.udp.is_empty()
    }
    pub fn uses_tcp(&self) -> bool {
        !self.tcp_connect.is_empty() || !self.tcp_listen.is_empty()
    }
    pub fn uses_udp(&self) -> bool {
        !self.udp.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FsMode {
    Ro,
    Rw,
}

impl fmt::Display for FsMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FsMode::Ro => "ro",
            FsMode::Rw => "rw",
        })
    }
}

/// データディレクトリ配下の相対パスへのアクセス許可。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FsGrant {
    pub path: String,
    pub mode: FsMode,
}

impl FsGrant {
    /// 絶対パス・`..`・空要素を含まない相対パスか。実施側 (preopen) でも同じ判定を使う。
    pub fn is_confined_path(path: &str) -> bool {
        if path.is_empty() || path.starts_with('/') || path.starts_with('\\') {
            return false;
        }
        // "C:\" などのドライブ指定も拒否
        if path.len() >= 2 && path.as_bytes()[1] == b':' {
            return false;
        }
        path.split(['/', '\\'])
            .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
    }
}

/// `host:port` (先頭 `*.` でサブドメイン一致)。文字列として serde する。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostPort {
    pub host: HostPattern,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostPattern {
    /// IP アドレスそのもの
    Ip(IpAddr),
    /// ホスト名の完全一致 (大文字小文字を区別しない)
    Name(String),
    /// `*.example.local`: サブドメインに一致 (apex 自身は含まない)
    Subdomain(String),
}

impl HostPort {
    pub fn parse(s: &str) -> Result<Self, String> {
        let (host, port) = s
            .rsplit_once(':')
            .ok_or_else(|| format!("`{s}` は host:port の形式ではありません"))?;
        let port: u16 = port
            .parse()
            .map_err(|_| format!("`{s}` のポートが 0..65535 の整数ではありません"))?;
        if host.is_empty() {
            return Err(format!("`{s}` のホストが空です"));
        }
        let valid = |h: &str| {
            !h.is_empty()
                && h.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        };
        let pattern = if let Some(rest) = host.strip_prefix("*.") {
            if !valid(rest) || rest.parse::<IpAddr>().is_ok() {
                return Err(format!("`{s}` のサブドメイン指定が不正です"));
            }
            HostPattern::Subdomain(rest.to_ascii_lowercase())
        } else if let Ok(ip) = host.parse::<IpAddr>() {
            HostPattern::Ip(ip)
        } else if valid(host) {
            HostPattern::Name(host.to_ascii_lowercase())
        } else {
            return Err(format!("`{s}` のホストに使えない文字があります"));
        };
        Ok(Self {
            host: pattern,
            port,
        })
    }

    /// IP リテラルの宣言がこの宛先に一致するか (名前の宣言は [`Self::matches_name`] と名前解決で扱う)。
    pub fn matches_addr(&self, addr: &SocketAddr) -> bool {
        self.port == addr.port()
            && match &self.host {
                HostPattern::Ip(ip) => ip.to_canonical() == addr.ip().to_canonical(),
                _ => false,
            }
    }

    /// ホスト名の宣言がこの名前に一致するか。
    pub fn matches_name(&self, name: &str) -> bool {
        let name = name.trim_end_matches('.').to_ascii_lowercase();
        match &self.host {
            HostPattern::Ip(ip) => name.parse::<IpAddr>().ok() == Some(*ip),
            HostPattern::Name(n) => *n == name,
            HostPattern::Subdomain(d) => {
                name.len() > d.len() + 1
                    && name.ends_with(d)
                    && name[..name.len() - d.len()].ends_with('.')
            }
        }
    }

    pub fn is_name(&self) -> bool {
        !matches!(self.host, HostPattern::Ip(_))
    }
}

impl fmt::Display for HostPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.host {
            HostPattern::Ip(ip) => write!(f, "{ip}:{}", self.port),
            HostPattern::Name(n) => write!(f, "{n}:{}", self.port),
            HostPattern::Subdomain(d) => write!(f, "*.{d}:{}", self.port),
        }
    }
}

impl Serialize for HostPort {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for HostPort {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        HostPort::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// capability 1 件を表す正規の文字列。同意の鍵 (`sha256:capability`) と UI 表示に使う。
///
/// ```text
/// net.tcpConnect:192.168.10.5:502   net.tcpListen:127.0.0.1:9000   net.udp:…
/// fs:rw:logs                         clock   random   hostExt:serial-port
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    TcpConnect(HostPort),
    TcpListen(HostPort),
    Udp(HostPort),
    Fs(FsGrant),
    Clock,
    Random,
    HostExt(String),
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::TcpConnect(h) => write!(f, "net.tcpConnect:{h}"),
            Capability::TcpListen(h) => write!(f, "net.tcpListen:{h}"),
            Capability::Udp(h) => write!(f, "net.udp:{h}"),
            Capability::Fs(g) => write!(f, "fs:{}:{}", g.mode, g.path),
            Capability::Clock => f.write_str("clock"),
            Capability::Random => f.write_str("random"),
            Capability::HostExt(n) => write!(f, "hostExt:{n}"),
        }
    }
}

impl Capability {
    /// 同意ダイアログ向けの説明 (日本語)。
    pub fn describe(&self) -> String {
        match self {
            Capability::TcpConnect(h) => format!("TCP で {h} へ接続する"),
            Capability::TcpListen(h) => format!("TCP で {h} を待ち受ける"),
            Capability::Udp(h) => format!("UDP で {h} と通信する"),
            Capability::Fs(g) => match g.mode {
                FsMode::Ro => format!("データディレクトリ内の `{}` を読む", g.path),
                FsMode::Rw => format!("データディレクトリ内の `{}` を読み書きする", g.path),
            },
            Capability::Clock => "壁時計を読む (決定性の保証外になります)".into(),
            Capability::Random => "OS の乱数を使う (決定性の保証外になります)".into(),
            Capability::HostExt(n) => format!("ホスト拡張 `{n}` を使う"),
        }
    }
}

impl Capabilities {
    pub fn is_empty(&self) -> bool {
        self.net.as_ref().is_none_or(|n| n.is_empty())
            && self.fs.is_empty()
            && !self.clock
            && !self.random
            && self.host_ext.is_empty()
    }

    /// 宣言を個別の capability に展開する (順序は宣言順で安定)。
    pub fn items(&self) -> Vec<Capability> {
        let mut v = Vec::new();
        if let Some(n) = &self.net {
            v.extend(n.tcp_connect.iter().cloned().map(Capability::TcpConnect));
            v.extend(n.tcp_listen.iter().cloned().map(Capability::TcpListen));
            v.extend(n.udp.iter().cloned().map(Capability::Udp));
        }
        v.extend(self.fs.iter().cloned().map(Capability::Fs));
        if self.clock {
            v.push(Capability::Clock);
        }
        if self.random {
            v.push(Capability::Random);
        }
        v.extend(self.host_ext.iter().cloned().map(Capability::HostExt));
        v
    }

    /// import 一覧から「これを link するのに要る宣言」を逆引きする。
    /// manifest (宣言) がない文脈 (サーバの部品登録時の describe、`inspect`) で、deny-all 形の付与に使う。
    /// 値はダミー (`127.0.0.1:0`、`fs: sandbox`) で、許可リストとしての意味はない。
    pub fn covering_imports(imports: &[ImportInfo]) -> Capabilities {
        let mut c = Capabilities::default();
        let dummy = HostPort {
            host: HostPattern::Ip(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
            port: 0,
        };
        for i in imports.iter().filter(|i| !i.types_only) {
            match classify_import(&i.name) {
                ImportClass::Net => {
                    let n = c.net.get_or_insert_with(NetCapabilities::default);
                    if interface_of(&i.name).starts_with("udp") {
                        if n.udp.is_empty() {
                            n.udp.push(dummy.clone());
                        }
                    } else if n.tcp_connect.is_empty() {
                        n.tcp_connect.push(dummy.clone());
                    }
                }
                ImportClass::Fs => {
                    if c.fs.is_empty() {
                        c.fs.push(FsGrant {
                            path: "sandbox".into(),
                            mode: FsMode::Ro,
                        });
                    }
                }
                ImportClass::Clock => c.clock = true,
                ImportClass::Random => c.random = true,
                ImportClass::HostExt(n) if !c.host_ext.contains(&n) => c.host_ext.push(n),
                _ => {}
            }
        }
        c
    }

    /// 宣言そのものの整合性 (import とは無関係に分かる誤り)。
    pub fn self_check(&self) -> Vec<String> {
        let mut errs = Vec::new();
        for g in &self.fs {
            if !FsGrant::is_confined_path(&g.path) {
                errs.push(format!(
                    "fs.path `{}` はデータディレクトリ配下の相対パスである必要があります (絶対パス・`..` は不可)",
                    g.path
                ));
            }
        }
        for h in &self.host_ext {
            if !KNOWN_HOST_EXT.contains(&h.as_str()) {
                errs.push(format!(
                    "hostExt `{h}` はこのエンジンが提供しないホスト拡張です (提供: {})",
                    KNOWN_HOST_EXT.join(", ")
                ));
            }
        }
        errs
    }

    /// 部品の import 一覧と突き合わせる。戻り値は (V7 のエラー, V8 の警告)。
    ///
    /// - 型だけを使う instance import (`wasi:clocks/wall-clock` の `datetime` など) は
    ///   Linker に実体が要らないので対象外。
    /// - `wasi:io/*` は sockets / filesystem / monotonic-clock の土台なので、
    ///   それらのいずれかが宣言されていれば許す。
    /// - `wasi:cli/*` は capability として提供しない (stdout 等を暗黙に与えない)。
    pub fn check_imports(&self, imports: &[ImportInfo]) -> (Vec<String>, Vec<String>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let net = self.net.as_ref().filter(|n| !n.is_empty());
        let any_wasi = net.is_some() || !self.fs.is_empty() || self.clock;
        let mut seen_net = false;
        let mut seen_fs = false;
        let mut seen_clock = false;
        let mut seen_random = false;
        let mut seen_ext: Vec<&str> = Vec::new();

        for imp in imports.iter().filter(|i| !i.types_only) {
            let name = imp.name.as_str();
            match classify_import(name) {
                ImportClass::EmuBase => {}
                ImportClass::HostExt(ext) => {
                    if self.host_ext.contains(&ext) {
                        seen_ext.push(name);
                    } else {
                        errors.push(format!(
                            "`{name}` を import しますが hostExt に `{ext}` が宣言されていません"
                        ));
                    }
                }
                ImportClass::Net => {
                    seen_net = true;
                    let iface = interface_of(name);
                    let ok = match net {
                        None => false,
                        Some(n) if iface.starts_with("udp") => n.uses_udp(),
                        Some(n) if iface.starts_with("tcp") => n.uses_tcp(),
                        Some(_) => true,
                    };
                    if !ok {
                        errors.push(format!("`{name}` を import しますが net の宣言がありません (tcpConnect / tcpListen / udp)"));
                    }
                }
                ImportClass::Fs => {
                    seen_fs = true;
                    if self.fs.is_empty() {
                        errors.push(format!("`{name}` を import しますが fs の宣言がありません"));
                    }
                }
                ImportClass::Clock => {
                    seen_clock = true;
                    if !self.clock {
                        errors.push(format!(
                            "`{name}` を import しますが clock: true が宣言されていません"
                        ));
                    }
                }
                ImportClass::Random => {
                    seen_random = true;
                    if !self.random {
                        errors.push(format!(
                            "`{name}` を import しますが random: true が宣言されていません"
                        ));
                    }
                }
                ImportClass::Io => {
                    if !any_wasi {
                        errors.push(format!("`{name}` を import しますが、これを必要とする capability (net / fs / clock) の宣言がありません"));
                    }
                }
                ImportClass::Cli => errors.push(format!(
                    "`{name}` は capability として提供していません (wasi:cli は与えない方針)"
                )),
                ImportClass::OtherWasi | ImportClass::Unknown => {
                    errors.push(format!("`{name}` はエンジンが提供しない import です"));
                }
            }
        }

        if net.is_some() && !seen_net {
            warnings.push(
                "net を宣言していますが wasi:sockets を import していません (過剰宣言)".into(),
            );
        }
        if !self.fs.is_empty() && !seen_fs {
            warnings.push(
                "fs を宣言していますが wasi:filesystem を import していません (過剰宣言)".into(),
            );
        }
        if self.clock && !seen_clock {
            warnings.push(
                "clock を宣言していますが wasi:clocks を import していません (過剰宣言)".into(),
            );
        }
        if self.random && !seen_random {
            warnings.push(
                "random を宣言していますが wasi:random を import していません (過剰宣言)".into(),
            );
        }
        for h in &self.host_ext {
            if !seen_ext.iter().any(|n| interface_of(n) == h) {
                warnings.push(format!(
                    "hostExt `{h}` を宣言していますが import していません (過剰宣言)"
                ));
            }
        }
        (errors, warnings)
    }
}

/// コンポーネントの import 1 件 (`wasm-tools component wit` の `import` 行に相当)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportInfo {
    /// 例 `wasi:sockets/tcp@0.2.12`, `ip:emu/host@0.1.0`
    pub name: String,
    /// 型だけを export する instance (関数もリソースもない)。Linker に定義が要らない
    pub types_only: bool,
}

impl ImportInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            types_only: false,
        }
    }
}

/// import 名の分類。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportClass {
    /// `ip:emu/host`, `ip:emu/types`, `ip:emu/hal`: 常に link する
    EmuBase,
    /// `ip:emu/<name>` (host-ext.wit のインターフェース)
    HostExt(String),
    Net,
    Fs,
    Clock,
    Random,
    Io,
    Cli,
    OtherWasi,
    Unknown,
}

/// `ns:pkg/iface@ver` の iface 部分。
pub fn interface_of(name: &str) -> &str {
    let after = name.split_once('/').map(|(_, i)| i).unwrap_or("");
    after.split('@').next().unwrap_or("")
}

pub fn classify_import(name: &str) -> ImportClass {
    let Some((pkg, _)) = name.split_once('/') else {
        return ImportClass::Unknown;
    };
    let iface = interface_of(name);
    match pkg {
        "ip:emu" => match iface {
            "host" | "types" | "hal" => ImportClass::EmuBase,
            other => ImportClass::HostExt(other.to_owned()),
        },
        "wasi:sockets" => ImportClass::Net,
        "wasi:filesystem" => ImportClass::Fs,
        "wasi:clocks" => ImportClass::Clock,
        "wasi:random" => ImportClass::Random,
        "wasi:io" => ImportClass::Io,
        "wasi:cli" => ImportClass::Cli,
        p if p.starts_with("wasi:") => ImportClass::OtherWasi,
        _ => ImportClass::Unknown,
    }
}

/// manifest の承認情報 (`approval`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub approved_by: String,
    pub approved_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// [`capabilities_hash`] の値
    pub capabilities_hash: String,
}

/// 承認対象となる capability 宣言の正規形: `{ "<instance>": <capabilities> }`。
///
/// 宣言のない (または空の) instance は含めない。値は [`Capabilities`] の serde 出力
/// (既定値のフィールドを省いた形) なので、`"clock": false` の有無のような表記差では変わらない。
pub fn capabilities_document(manifest: &Manifest) -> serde_json::Value {
    let mut m = serde_json::Map::new();
    let mut by_instance: BTreeMap<&str, &Capabilities> = BTreeMap::new();
    for c in &manifest.components {
        if let Some(cap) = c.capabilities.as_ref().filter(|c| !c.is_empty()) {
            by_instance.insert(c.instance.as_str(), cap);
        }
    }
    for (k, v) in by_instance {
        m.insert(
            k.to_owned(),
            serde_json::to_value(v).expect("Capabilities は常に serialize できる"),
        );
    }
    serde_json::Value::Object(m)
}

/// `approval.capabilitiesHash`: [`capabilities_document`] を RFC 8785 で正規化して sha256 (hex 小文字)。
///
/// サーバ (M3/M4) は承認時にこれを `approval.capabilitiesHash` に書き、
/// クライアントは V12 で再計算して照合する。
pub fn capabilities_hash(manifest: &Manifest) -> String {
    use sha2::{Digest, Sha256};
    let canon = jcs::canonicalize(&capabilities_document(manifest));
    hex::encode(Sha256::digest(canon.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_port_parse_and_match() {
        let h = HostPort::parse("192.168.10.5:502").unwrap();
        assert!(h.matches_addr(&"192.168.10.5:502".parse().unwrap()));
        assert!(!h.matches_addr(&"192.168.10.5:503".parse().unwrap()));
        assert_eq!(h.to_string(), "192.168.10.5:502");

        let s = HostPort::parse("*.example.local:8080").unwrap();
        assert!(s.matches_name("a.example.local"));
        assert!(s.matches_name("B.a.Example.local."));
        assert!(!s.matches_name("example.local"));
        assert!(!s.matches_name("notexample.local"));
        assert_eq!(s.to_string(), "*.example.local:8080");

        let n = HostPort::parse("plc.local:502").unwrap();
        assert!(n.matches_name("PLC.local"));
        assert!(!n.matches_name("x.plc.local"));

        assert!(HostPort::parse("nohost").is_err());
        assert!(HostPort::parse(":80").is_err());
        assert!(HostPort::parse("a:99999").is_err());
        assert!(HostPort::parse("a b:80").is_err());
    }

    #[test]
    fn items_and_strings() {
        let c: Capabilities = serde_json::from_value(serde_json::json!({
            "net": { "tcpConnect": ["127.0.0.1:9000"], "tcpListen": ["127.0.0.1:9001"] },
            "fs": [ { "path": "logs", "mode": "rw" } ],
            "clock": true,
            "hostExt": ["serial-port"]
        }))
        .unwrap();
        let s: Vec<String> = c.items().iter().map(|i| i.to_string()).collect();
        assert_eq!(
            s,
            [
                "net.tcpConnect:127.0.0.1:9000",
                "net.tcpListen:127.0.0.1:9001",
                "fs:rw:logs",
                "clock",
                "hostExt:serial-port"
            ]
        );
        assert!(c.self_check().is_empty());
        // 往復で既定値は省かれる
        let v = serde_json::to_value(&c).unwrap();
        assert!(v.get("random").is_none());
        assert_eq!(serde_json::from_value::<Capabilities>(v).unwrap(), c);
    }

    #[test]
    fn fs_path_confinement() {
        assert!(FsGrant::is_confined_path("logs"));
        assert!(FsGrant::is_confined_path("logs/2026"));
        assert!(!FsGrant::is_confined_path("/etc"));
        assert!(!FsGrant::is_confined_path("../x"));
        assert!(!FsGrant::is_confined_path("a/../b"));
        assert!(!FsGrant::is_confined_path("C:\\x"));
        assert!(!FsGrant::is_confined_path(""));
    }

    #[test]
    fn classify() {
        assert_eq!(classify_import("ip:emu/host@0.1.0"), ImportClass::EmuBase);
        assert_eq!(
            classify_import("ip:emu/serial-port@0.1.0"),
            ImportClass::HostExt("serial-port".into())
        );
        assert_eq!(classify_import("wasi:sockets/tcp@0.2.12"), ImportClass::Net);
        assert_eq!(classify_import("wasi:cli/stdout@0.2.12"), ImportClass::Cli);
        assert_eq!(
            classify_import("wasi:http/types@0.2.12"),
            ImportClass::OtherWasi
        );
        assert_eq!(classify_import("foo"), ImportClass::Unknown);
    }

    #[test]
    fn check_imports_v7_v8() {
        let imports = vec![
            ImportInfo::new("ip:emu/host@0.1.0"),
            ImportInfo::new("wasi:io/streams@0.2.12"),
            ImportInfo::new("wasi:sockets/tcp@0.2.12"),
            ImportInfo {
                name: "wasi:clocks/wall-clock@0.2.12".into(),
                types_only: true,
            },
        ];
        // 未宣言 → V7
        let none = Capabilities::default();
        let (e, _) = none.check_imports(&imports);
        assert_eq!(e.len(), 2, "{e:?}"); // io と sockets
                                         // tcpConnect 宣言 → OK。型だけの wall-clock は不問
        let ok: Capabilities =
            serde_json::from_value(serde_json::json!({ "net": { "tcpConnect": ["127.0.0.1:1"] } }))
                .unwrap();
        let (e, w) = ok.check_imports(&imports);
        assert!(e.is_empty(), "{e:?}");
        assert!(w.is_empty(), "{w:?}");
        // udp しか宣言していないのに tcp → V7
        let udp: Capabilities =
            serde_json::from_value(serde_json::json!({ "net": { "udp": ["127.0.0.1:1"] } }))
                .unwrap();
        assert_eq!(udp.check_imports(&imports).0.len(), 1);
        // 過剰宣言 → V8
        let over: Capabilities = serde_json::from_value(serde_json::json!({
            "net": { "tcpConnect": ["127.0.0.1:1"] }, "fs": [{"path": "x", "mode": "ro"}], "random": true
        }))
        .unwrap();
        let (e, w) = over.check_imports(&imports);
        assert!(e.is_empty());
        assert_eq!(w.len(), 2, "{w:?}");
        // wasi:cli は常に拒否
        let cli = [ImportInfo::new("wasi:cli/stdout@0.2.12")];
        assert_eq!(over.check_imports(&cli).0.len(), 1);
    }

    #[test]
    fn covering_imports_reverses_classification() {
        let imports = vec![
            ImportInfo::new("wasi:sockets/tcp@0.2.12"),
            ImportInfo::new("wasi:sockets/udp@0.2.12"),
            ImportInfo::new("wasi:filesystem/types@0.2.12"),
            ImportInfo::new("wasi:random/random@0.2.12"),
            ImportInfo::new("ip:emu/serial-port@0.1.0"),
            ImportInfo {
                name: "wasi:clocks/wall-clock@0.2.12".into(),
                types_only: true,
            },
        ];
        let c = Capabilities::covering_imports(&imports);
        let n = c.net.as_ref().unwrap();
        assert!(n.uses_tcp() && n.uses_udp());
        assert_eq!(c.fs.len(), 1);
        assert!(c.random && !c.clock);
        assert_eq!(c.host_ext, ["serial-port"]);
        // 逆引きした宣言は V7 を通す
        assert!(c.check_imports(&imports).0.is_empty());
        assert!(Capabilities::covering_imports(&[]).is_empty());
    }

    #[test]
    fn hash_is_order_independent_and_normalized() {
        let a = Manifest::from_json(
            r#"{"manifestVersion":1,"id":"abc","revision":1,"name":"n","scheduler":{"baseStepNs":1000000},
              "components":[
                {"instance":"b","kind":"builtin:snapshot","capabilities":{"clock":false,"net":{"tcpConnect":["10.0.0.1:1"]}}},
                {"instance":"a","kind":"builtin:snapshot","capabilities":{"fs":[{"mode":"rw","path":"logs"}]}}
              ]}"#,
        )
        .unwrap();
        let b = Manifest::from_json(
            r#"{"manifestVersion":1,"id":"abc","revision":9,"name":"other","scheduler":{"baseStepNs":1000000},
              "components":[
                {"instance":"a","kind":"builtin:snapshot","capabilities":{"fs":[{"path":"logs","mode":"rw"}]}},
                {"instance":"c","kind":"builtin:snapshot","capabilities":{}},
                {"instance":"b","kind":"builtin:snapshot","capabilities":{"net":{"tcpConnect":["10.0.0.1:1"]}}}
              ]}"#,
        )
        .unwrap();
        assert_eq!(capabilities_hash(&a), capabilities_hash(&b));
        assert_eq!(
            jcs::canonicalize(&capabilities_document(&a)),
            r#"{"a":{"fs":[{"mode":"rw","path":"logs"}]},"b":{"net":{"tcpConnect":["10.0.0.1:1"]}}}"#
        );
        let mut c = a.clone();
        c.components[0].capabilities = None;
        assert_ne!(capabilities_hash(&a), capabilities_hash(&c));
    }
}
